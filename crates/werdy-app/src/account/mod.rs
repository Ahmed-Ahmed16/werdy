//! Account state owns the durable outbox. UI edits use the same store wrapper
//! as background completion, so neither can overwrite the other's snapshot.

pub mod config;
pub mod runtime;
pub mod sharing;
pub mod signin;
pub mod storage;

use serde::{Deserialize, Serialize};
use std::{cell::RefCell, rc::Rc};
use storage::{File, Vault};
use werdy_store::{OwnerData, Store, StoreError, from_json, to_json};
use werdy_sync::{Session, Sync};

#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub owner_id: String,
    pub install_id: String,
    pub engine: Sync,
    /// Write ahead copy: engine and records commit in one file or storage key.
    pub records: String,
    pub last_sync: Option<u64>,
}

/// Times use monotonic elapsed milliseconds, so a wrong wall clock cannot
/// postpone a retry or a periodic sync for years.
#[derive(Default)]
pub struct Schedule {
    due: Option<u64>,
    periodic: Option<u64>,
    running: bool,
    dirty: bool,
}

impl Schedule {
    pub fn is_running(&self) -> bool {
        self.running
    }
    pub fn saved(&mut self, now: u64) {
        self.due = Some(now.saturating_add(3_000));
        self.dirty = true;
    }
    pub fn start(&mut self, now: u64) -> bool {
        let due = self.due.into_iter().chain(self.periodic).min();
        if self.running || due.is_none_or(|due| now < due) {
            return false;
        }
        self.running = true;
        self.dirty = false;
        true
    }
    pub fn finish(&mut self, now: u64, success: bool) {
        self.running = false;
        self.periodic = success.then(|| now.saturating_add(300_000));
        if !self.dirty {
            self.due = (!success).then(|| now.saturating_add(30_000));
        }
    }
    pub fn immediate(&mut self) {
        self.periodic = Some(0);
    }
}

#[derive(Default)]
pub struct Account {
    pub sharing_busy: bool,
    pub session: Option<Session>,
    pub checkpoint: Option<Checkpoint>,
    pub schedule: Schedule,
    pub generation: u64,
    pub error: Option<&'static str>,
    pub signing_in: bool,
}

pub struct AccountStore {
    pub active_task: RefCell<Option<dioxus::dioxus_core::Task>>,
    pub inner: Rc<dyn Store>,
    pub vault: Rc<dyn Vault>,
    pub account: RefCell<Account>,
    pub enabled: bool,
}

fn storage_error() -> StoreError {
    StoreError::Backend("could not save account state".into())
}

impl AccountStore {
    pub fn open(inner: Rc<dyn Store>, vault: Rc<dyn Vault>, enabled: bool) -> Self {
        let mut account = Account::default();
        if enabled {
            match restore(vault.as_ref()) {
                Ok((session, checkpoint)) => {
                    account.session = session;
                    account.checkpoint = checkpoint;
                    if account.session.is_some() {
                        account.schedule.immediate();
                    }
                }
                Err(_) => account.error = Some("account-storage-error"),
            }
        }
        Self {
            inner,
            vault,
            account: RefCell::new(account),
            enabled,
            active_task: RefCell::new(None),
        }
    }

    pub fn sign_in(&self, session: Session, records: &OwnerData) -> Result<(), StoreError> {
        if !self.enabled {
            return Err(storage_error());
        }
        session.validate().map_err(|_| storage_error())?;
        let mut records = records.clone();
        let install_id = signin::random_hex::<16>().map_err(|_| storage_error())?;
        let mut engine = Sync::new(&install_id).map_err(|_| storage_error())?;
        engine.capture(&records).map_err(|_| storage_error())?;
        engine.remember_deed_keys(&mut records);
        let checkpoint = Checkpoint {
            owner_id: session.user_id.clone(),
            install_id,
            engine,
            records: to_json(&records)?,
            last_sync: None,
        };
        self.write_checkpoint(&checkpoint)?;
        self.write_session(&session)?;
        if self.inner.save(&records).is_err() {
            dioxus::prelude::warn!("Records mirror save failed; sync checkpoint keeps the records");
        }
        let mut account = self.account.borrow_mut();
        account.generation = account.generation.wrapping_add(1);
        account.session = Some(session);
        account.checkpoint = Some(checkpoint);
        account.error = None;
        account.signing_in = false;
        account.schedule.immediate();
        Ok(())
    }

    pub fn sign_out(&self, records: &OwnerData) -> Result<(), StoreError> {
        let mut kept = records.clone();
        if let Some(checkpoint) = &self.account.borrow().checkpoint {
            checkpoint.engine.remember_deed_keys(&mut kept);
        }
        self.inner.save(&kept)?;
        // Removing the session first also stops startup from reviving a leftover
        // sync file if deleting that second file fails.
        self.vault.remove(File::Session)?;
        if let Some(task) = self.active_task.borrow_mut().take() {
            task.cancel();
        }
        let removed = self.vault.remove(File::Sync);
        let mut account = self.account.borrow_mut();
        let generation = account.generation.wrapping_add(1);
        *account = Account {
            generation,
            ..Account::default()
        };
        removed
    }

    /// Settles an account delete request. Only a confirmed server delete
    /// forgets the session; the records stay on this device either way.
    /// A reply for an older session leaves a newer sign in alone.
    pub fn settle_delete(
        &self,
        result: Result<(), werdy_sync::SyncError>,
        records: &OwnerData,
        generation: u64,
    ) -> Result<(), &'static str> {
        result.map_err(|_| "account-delete-error")?;
        if self.account.borrow().generation != generation {
            return Ok(());
        }
        self.sign_out(records).map_err(|_| "account-storage-error")
    }

    pub fn write_session(&self, session: &Session) -> Result<(), StoreError> {
        let json = serde_json::to_string(session).map_err(|_| storage_error())?;
        self.vault.write(File::Session, &json)
    }

    pub fn write_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), StoreError> {
        let json = serde_json::to_string(checkpoint).map_err(|_| storage_error())?;
        self.vault.write(File::Sync, &json)
    }

    pub fn complete(
        &self,
        completed: &Checkpoint,
        generation: u64,
        now: u64,
    ) -> Result<Option<OwnerData>, StoreError> {
        let mut account = self.account.borrow_mut();
        if account.generation != generation || account.session.is_none() {
            return Ok(None);
        }
        let mut current = account.checkpoint.clone().ok_or_else(storage_error)?;
        let mut records = from_json(&current.records)?;
        current
            .engine
            .finish_from(&completed.engine, &mut records)
            .map_err(|_| storage_error())?;
        current.records = to_json(&records)?;
        current.last_sync = Some(now);
        self.write_checkpoint(&current)?;
        account.checkpoint = Some(current);
        if self.inner.save(&records).is_err() {
            dioxus::prelude::warn!("Records mirror save failed; sync checkpoint keeps the records");
        }
        Ok(Some(records))
    }
}

impl Store for AccountStore {
    fn load(&self) -> Result<Option<OwnerData>, StoreError> {
        if let Some(checkpoint) = &self.account.borrow().checkpoint {
            return from_json(&checkpoint.records).map(Some);
        }
        self.inner.load()
    }
    fn save(&self, records: &OwnerData) -> Result<(), StoreError> {
        let mut records = records.clone();
        let mut account = self.account.borrow_mut();
        if let Some(mut checkpoint) = account.checkpoint.clone() {
            checkpoint
                .engine
                .capture(&records)
                .map_err(|_| storage_error())?;
            checkpoint.engine.remember_deed_keys(&mut records);
            checkpoint.records = to_json(&records)?;
            self.write_checkpoint(&checkpoint)?;
            account.checkpoint = Some(checkpoint);
            account.schedule.saved(runtime::elapsed_ms());
        }
        self.inner.save(&records)
    }
}

fn restore(vault: &dyn Vault) -> Result<(Option<Session>, Option<Checkpoint>), StoreError> {
    let Some(json) = vault.read(File::Session)? else {
        return Ok((None, None));
    };
    let session: Session = serde_json::from_str(&json).map_err(|_| storage_error())?;
    session.validate().map_err(|_| storage_error())?;
    let json = vault.read(File::Sync)?.ok_or_else(storage_error)?;
    let checkpoint: Checkpoint = serde_json::from_str(&json).map_err(|_| storage_error())?;
    if checkpoint.owner_id != session.user_id {
        return Err(storage_error());
    }
    from_json(&checkpoint.records)?;
    Ok((Some(session), Some(checkpoint)))
}

#[cfg(test)]
mod tests;
