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
    session_write_pending: bool,
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
    /// Identifies this copy of the app for the sync lease.
    tab: String,
    /// The checkpoint JSON this copy last read or wrote, so an unchanged
    /// checkpoint is not parsed again.
    seen: RefCell<String>,
}

pub const LEASE_MS: u64 = 120_000;
pub type Flight = (Session, Checkpoint, u64);

#[derive(Serialize, Deserialize)]
struct Lease {
    holder: String,
    /// Wall clock milliseconds, shared by every copy on the device.
    until: u64,
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
            tab: signin::random_hex::<16>().unwrap_or_else(|()| runtime::elapsed_ms().to_string()),
            seen: RefCell::new(String::new()),
        }
    }

    /// Starts a sync run if no other copy of the app on this device holds
    /// the sync lease. Reads what another copy saved before starting the run.
    pub fn claim_flight(&self, now: u64) -> Result<Option<Flight>, StoreError> {
        if !self.take_lease(now)? {
            return Ok(None);
        }
        let mut account = self.account.borrow_mut();
        let reread = self
            .reread_checkpoint(&mut account)
            .and_then(|()| self.reread_session(&mut account));
        if let Err(error) = reread {
            drop(account);
            self.release_flight();
            return Err(error);
        }
        Ok(account
            .session
            .clone()
            .zip(account.checkpoint.clone())
            .map(|(session, checkpoint)| (session, checkpoint, account.generation)))
    }

    /// Gives the sync lease back so another copy can sync. A copy that
    /// closes mid run never calls this; its lease runs out after `LEASE_MS`.
    pub fn release_flight(&self) {
        let ours = self
            .vault
            .read(File::Lease)
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str::<Lease>(&json).ok())
            .is_some_and(|lease| lease.holder == self.tab);
        if ours && self.vault.remove(File::Lease).is_err() {
            dioxus::prelude::warn!("Sync lease release failed; it runs out by itself");
        }
    }

    fn take_lease(&self, now: u64) -> Result<bool, StoreError> {
        let held = self
            .vault
            .read(File::Lease)?
            .and_then(|json| serde_json::from_str::<Lease>(&json).ok());
        // A lease that ends further ahead than one full lease comes from a
        // clock that moved backwards, so it does not count.
        if held.is_some_and(|lease| {
            lease.holder != self.tab
                && now < lease.until
                && lease.until <= now.saturating_add(LEASE_MS)
        }) {
            return Ok(false);
        }
        let lease = Lease {
            holder: self.tab.clone(),
            until: now.saturating_add(LEASE_MS),
        };
        let json = serde_json::to_string(&lease).map_err(|_| storage_error())?;
        self.vault.write(File::Lease, &json)?;
        Ok(true)
    }

    /// Another copy of the app may have saved since this one last read or
    /// wrote the checkpoint. Its counters must be the base for the next
    /// write, or both copies hand out the same operation number.
    fn reread_checkpoint(&self, account: &mut Account) -> Result<(), StoreError> {
        let Some(current) = account.checkpoint.as_ref() else {
            return Ok(());
        };
        let Some(json) = self.vault.read(File::Sync)? else {
            return Ok(());
        };
        if *self.seen.borrow() == json {
            return Ok(());
        }
        let saved: Checkpoint = serde_json::from_str(&json).map_err(|_| storage_error())?;
        if saved.owner_id == current.owner_id {
            account.checkpoint = Some(saved);
            *self.seen.borrow_mut() = json;
        }
        Ok(())
    }

    /// Use the session another copy saved unless our refresh still needs saving.
    fn reread_session(&self, account: &mut Account) -> Result<(), StoreError> {
        let Some(json) = self.vault.read(File::Session)? else {
            return Ok(());
        };
        let saved = serde_json::from_str::<Session>(&json)
            .ok()
            .filter(|saved| saved.validate().is_ok());
        if !account.session_write_pending
            && let Some(saved) = saved
            && account
                .session
                .as_ref()
                .is_some_and(|session| session.user_id == saved.user_id)
        {
            account.session = Some(saved);
        }
        Ok(())
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
        account.session_write_pending = false;
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
        self.release_flight();
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
        self.vault.write(File::Session, &json)?;
        let mut account = self.account.borrow_mut();
        if account.session.as_ref() == Some(session) {
            account.session_write_pending = false;
        }
        Ok(())
    }

    pub fn write_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), StoreError> {
        let json = serde_json::to_string(checkpoint).map_err(|_| storage_error())?;
        self.vault.write(File::Sync, &json)?;
        *self.seen.borrow_mut() = json;
        Ok(())
    }

    pub fn complete(
        &self,
        completed: &Checkpoint,
        generation: u64,
        now: u64,
    ) -> Result<Option<OwnerData>, StoreError> {
        self.finish_attempt(generation, |current, records| {
            current
                .engine
                .finish_from(&completed.engine, records)
                .map_err(|_| storage_error())?;
            current.last_sync = Some(now);
            Ok(())
        })
    }

    fn recover_collision(
        &self,
        collision: &werdy_sync::Collision,
        generation: u64,
    ) -> Result<Option<OwnerData>, StoreError> {
        self.finish_attempt(generation, |current, records| {
            let install = signin::random_hex::<16>().map_err(|_| storage_error())?;
            current
                .engine
                .recover_collision(collision, &install, records)
                .map_err(|_| storage_error())?;
            current.install_id = install;
            Ok(())
        })
    }

    fn finish_attempt(
        &self,
        generation: u64,
        merge: impl FnOnce(&mut Checkpoint, &mut OwnerData) -> Result<(), StoreError>,
    ) -> Result<Option<OwnerData>, StoreError> {
        let mut account = self.account.borrow_mut();
        if account.generation != generation || account.session.is_none() {
            return Ok(None);
        }
        self.reread_checkpoint(&mut account)?;
        let mut current = account.checkpoint.clone().ok_or_else(storage_error)?;
        let mut records = from_json(&current.records)?;
        merge(&mut current, &mut records)?;
        // The first sync after sign in can bring the account's copy of each
        // starter deed next to this device's copy. Merge them and queue the
        // repair, so every device and the account end with one copy.
        current.engine.remember_deed_keys(&mut records);
        let unsent = current.engine.unsent_marks();
        if records.merge_duplicate_starters(&unsent) {
            current
                .engine
                .capture(&records)
                .map_err(|_| storage_error())?;
            current.engine.remember_deed_keys(&mut records);
            account.schedule.saved(runtime::elapsed_ms());
        }
        current.records = to_json(&records)?;
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
        let mut account = self.account.borrow_mut();
        self.reread_checkpoint(&mut account)?;
        if let Some(checkpoint) = &account.checkpoint {
            return from_json(&checkpoint.records).map(Some);
        }
        self.inner.load()
    }
    fn save(&self, records: &OwnerData) -> Result<(), StoreError> {
        let mut records = records.clone();
        let mut account = self.account.borrow_mut();
        self.reread_checkpoint(&mut account)?;
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
