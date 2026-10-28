//! The only seam between owner records and a backend (ADRs 0015, 0016, 0027).
//!
//! Call [`Sync::capture`] after local edits, even while offline. Save the whole
//! engine alongside `OwnerData`, atomically, after capture and after every sync
//! attempt, including errors. A failed attempt may have captured new edits.
//! Keep one engine per account and install. Never reuse it for another account.
//! The caller supplies a unique, lasting install id and runs sync in the background.
//! The optional `supabase` feature provides a network adapter. No executor or
//! blocking storage is hidden here.
//!
//! Share code redemption belongs at this backend seam in issue #24. It is not
//! part of the owner change stream yet.

#![forbid(unsafe_code)]

mod fake;
mod rows;
#[cfg(feature = "supabase")]
mod supabase;

pub use fake::FakeBackend;
pub use rows::{Change, DeedKey, DeedRow, Setting};
#[cfg(feature = "supabase")]
pub use supabase::SupabaseBackend;
#[cfg(feature = "supabase")]
pub use supabase::auth::{Session, SupabaseAuth};

use serde::{Deserialize, Serialize};
use std::future::Future;
use werdy_core::DeedId;
use werdy_store::OwnerData;

/// Failures leave local records and queued operations available for retry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyncError {
    Http(u16),
    Backend(String),
    Invalid(String),
    Exhausted,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(status) => write!(formatter, "Supabase returned HTTP {status}"),
            Self::Backend(reason) => write!(formatter, "sync backend failed: {reason}"),
            Self::Invalid(reason) => write!(formatter, "invalid sync data: {reason}"),
            Self::Exhausted => formatter.write_str("sync counter exhausted"),
        }
    }
}

impl std::error::Error for SyncError {}

fn invalid(reason: &str) -> SyncError {
    SyncError::Invalid(reason.to_owned())
}

/// A lasting operation identity makes an ambiguous network failure safe to retry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pending {
    pub install: String,
    pub operation: u64,
    pub change: Change,
}

/// Server time in Unix milliseconds. Sequence breaks ties within a millisecond.
/// Time never decreases and sequence strictly increases within an account.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Stamp {
    pub received_at: u64,
    pub sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Accepted {
    pub pending: Pending,
    pub stamp: Stamp,
}

/// Push acknowledges every operation in request order, with its original stamp
/// on retries. Pull returns a complete ordered stream through `cursor`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub changes: Vec<Accepted>,
    pub cursor: u64,
}

/// Werdy operations, scoped by the adapter to one authenticated owner.
/// The cursor is a server sequence, never a phone timestamp. Push is atomic and
/// deduplicates `(install, operation)` forever, rejecting a changed retry payload.
/// Pull includes tombstones and must not omit operations through its cursor.
/// Native returned futures avoid an async trait crate and allow a Send executor.
/// The engine is generic because object dispatch is not needed at this seam.
pub trait Backend {
    fn push(&mut self, changes: &[Pending]) -> impl BackendFuture<Result<Batch, SyncError>>;
    fn pull(&mut self, cursor: u64) -> impl BackendFuture<Result<Batch, SyncError>>;
    /// Actual server time in Unix milliseconds, sampled on every sync attempt.
    fn server_time(&mut self) -> impl BackendFuture<Result<u64, SyncError>>;
}

/// Native backends can run on a Send executor. Browser fetch futures stay on
/// the browser thread, so wasm backends do not need Send.
#[cfg(not(target_arch = "wasm32"))]
pub trait BackendFuture<T>: Future<Output = T> + Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T, F: Future<Output = T> + Send> BackendFuture<T> for F {}

#[cfg(target_arch = "wasm32")]
pub trait BackendFuture<T>: Future<Output = T> {}
#[cfg(target_arch = "wasm32")]
impl<T, F: Future<Output = T>> BackendFuture<T> for F {}

/// Server minus phone time. A positive offset means the phone is behind.
/// Estimate uses the midpoint of the caller's two clock samples. Network delay
/// adds uncertainty, reported as round trip time. Caller decides what to log.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockOffset {
    pub milliseconds: i128,
    pub round_trip_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Binding {
    local: DeedId,
    remote: DeedKey,
}

/// Serializable outbox, cursor, row stamps and local id bindings.
/// Lists keep composite keys serializable in JSON. Phone revision timestamps
/// are never consulted. `merge_records` is for local tabs and would lose deeds
/// and choose marks by phone time, so it cannot implement this account merge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sync {
    install: String,
    next_deed: u64,
    next_operation: u64,
    bindings: Vec<Binding>,
    observed: Vec<Change>,
    outbox: Vec<Pending>,
    accepted: Vec<Accepted>,
    cursor: u64,
}

impl Sync {
    /// Caller generates a unique install id, for example 128 random bits as hex,
    /// and saves it before use. Reinstall needs a new id. This crate has no random
    /// source, so uniqueness across installs is the caller's explicit contract.
    pub fn new(install: &str) -> Result<Self, SyncError> {
        rows::validate_install(install)?;
        Ok(Self {
            install: install.to_owned(),
            next_deed: 1,
            next_operation: 1,
            bindings: Vec::new(),
            observed: rows::settings_snapshot(&werdy_store::Settings::default()),
            outbox: Vec::new(),
            accepted: Vec::new(),
            cursor: 0,
        })
    }

    pub fn outbox(&self) -> &[Pending] {
        &self.outbox
    }

    /// Installs a completed background copy while keeping edits captured since
    /// that copy started. Existing local id bindings take priority, so a deed
    /// created during the request cannot alias a newly downloaded deed.
    /// The caller must save this engine and the resulting records atomically.
    pub fn finish_from(
        &mut self,
        completed: &Self,
        records: &mut OwnerData,
    ) -> Result<(), SyncError> {
        if self.install != completed.install
            || self.next_operation < completed.next_operation
            || self.cursor > completed.cursor
            || !completed.outbox.is_empty()
        {
            return Err(invalid("background sync belongs to another state"));
        }
        let mut staged = self.clone();
        for row in &completed.accepted {
            staged.accept(row.clone())?;
        }
        staged.cursor = completed.cursor;
        staged
            .outbox
            .retain(|pending| pending.operation >= completed.next_operation);
        let mut merged = records.clone();
        rows::project_with_outbox(&mut staged, &mut merged)?;
        staged.observed = rows::snapshot(&mut staged, &merged)?;
        *self = staged;
        *records = merged;
        Ok(())
    }

    /// Captures changed rows, including explicit clears and deed deletion.
    /// First capture imports all local history with fresh namespaced deed ids.
    /// Remote deeds retain their wire ids through saved bindings. Existing local
    /// numeric ids need no migration and two offline counters cannot collide.
    /// Untouched default settings do not replace account preferences on import.
    /// No state changes on invalid local input or counter exhaustion.
    pub fn capture(&mut self, records: &OwnerData) -> Result<(), SyncError> {
        let mut staged = self.clone();
        for (local, remote) in &records.deed_keys {
            rows::validate_install(&remote.install)?;
            if remote.sequence == 0
                || staged.bindings.iter().any(|binding| {
                    (binding.local == *local && binding.remote != *remote)
                        || (binding.remote == *remote && binding.local != *local)
                })
            {
                return Err(invalid("conflicting deed identity"));
            }
            if !staged
                .bindings
                .iter()
                .any(|binding| binding.local == *local)
            {
                staged.bindings.push(Binding {
                    local: *local,
                    remote: remote.clone(),
                });
            }
            if remote.install == staged.install {
                staged.next_deed = staged
                    .next_deed
                    .max(remote.sequence.checked_add(1).ok_or(SyncError::Exhausted)?);
            }
        }
        let snapshot = rows::snapshot(&mut staged, records)?;
        for change in &snapshot {
            if !staged.observed.contains(change) {
                let operation = staged.next_operation;
                staged.next_operation = operation.checked_add(1).ok_or(SyncError::Exhausted)?;
                staged.outbox.push(Pending {
                    install: staged.install.clone(),
                    operation,
                    change: change.clone(),
                });
            }
        }
        staged.observed = snapshot;
        *self = staged;
        Ok(())
    }

    /// After capture, persist these domain identities with the same records.
    /// They survive deletion of the account session and sync cursor.
    pub fn remember_deed_keys(&self, records: &mut OwnerData) {
        for binding in &self.bindings {
            if records.deeds.iter().any(|deed| deed.id == binding.local)
                || records.deleted_deeds.contains(&binding.local)
            {
                records
                    .deed_keys
                    .insert(binding.local, binding.remote.clone());
            }
        }
    }

    /// Captures edits, measures the clock, pushes then pulls from the last read
    /// cursor. The push cursor must not skip changes made by another phone.
    /// Failed pushes, pulls, validation and cancelled futures keep the outbox.
    /// Backend retries keep original server stamps, even if a reply was lost.
    /// Only a fully validated exchange replaces records and clears the queue.
    pub async fn sync<B: Backend>(
        &mut self,
        backend: &mut B,
        records: &mut OwnerData,
        mut phone_time: impl FnMut() -> u64,
    ) -> Result<ClockOffset, SyncError> {
        self.capture(records)?;
        let before = phone_time();
        let server_time = backend.server_time().await?;
        let after = phone_time();
        let round_trip_ms = after
            .checked_sub(before)
            .ok_or_else(|| invalid("phone clock moved backwards during sample"))?;
        let milliseconds = i128::from(server_time) - (i128::from(before) + i128::from(after)) / 2;
        let mut pushed = Batch {
            changes: Vec::new(),
            cursor: self.cursor,
        };
        // Each push is atomic. Small batches also fit the server byte limit
        // when every operation is at its maximum permitted size.
        for batch in self.outbox.chunks(32) {
            let receipt = backend.push(batch).await?;
            if receipt.changes.len() != batch.len()
                || receipt
                    .changes
                    .iter()
                    .zip(batch)
                    .any(|(row, pending)| &row.pending != pending)
                || receipt.cursor < pushed.cursor
            {
                return Err(invalid("incomplete push acknowledgement"));
            }
            pushed.cursor = receipt.cursor;
            pushed.changes.extend(receipt.changes);
        }
        if self.outbox.is_empty() {
            pushed = backend.push(&[]).await?;
        }
        self.validate_push(&pushed)?;
        let pulled = backend.pull(self.cursor).await?;
        self.validate_pull(&pulled, pushed.cursor)?;
        for receipt in &pushed.changes {
            if !pulled.changes.contains(receipt) {
                return Err(invalid("push receipt differs from pull stream"));
            }
        }
        let mut staged = self.clone();
        for accepted in pushed.changes.into_iter().chain(pulled.changes) {
            staged.accept(accepted)?;
        }
        let mut merged = records.clone();
        rows::project(&mut staged, &mut merged)?;
        staged.cursor = pulled.cursor;
        staged.outbox.clear();
        staged.observed = rows::snapshot(&mut staged, &merged)?;
        *records = merged;
        *self = staged;
        Ok(ClockOffset {
            milliseconds,
            round_trip_ms,
        })
    }

    fn validate_push(&self, pushed: &Batch) -> Result<(), SyncError> {
        if pushed.changes.len() != self.outbox.len() || pushed.cursor < self.cursor {
            return Err(invalid("incomplete push acknowledgement"));
        }
        for (accepted, pending) in pushed.changes.iter().zip(&self.outbox) {
            validate_accepted(accepted, pushed.cursor)?;
            if &accepted.pending != pending {
                return Err(invalid("push acknowledged a different operation"));
            }
        }
        Ok(())
    }

    fn validate_pull(&self, pulled: &Batch, pushed_cursor: u64) -> Result<(), SyncError> {
        if pulled.cursor < pushed_cursor {
            return Err(invalid("pull cursor moved backwards"));
        }
        let mut previous = self.cursor;
        let mut previous_time = self
            .accepted
            .iter()
            .map(|row| row.stamp.received_at)
            .max()
            .unwrap_or(0);
        for accepted in &pulled.changes {
            validate_accepted(accepted, pulled.cursor)?;
            if accepted.stamp.sequence != previous.checked_add(1).ok_or(SyncError::Exhausted)?
                || accepted.stamp.received_at < previous_time
            {
                return Err(invalid("pull stream is incomplete or out of order"));
            }
            previous = accepted.stamp.sequence;
            previous_time = accepted.stamp.received_at;
        }
        if previous != pulled.cursor {
            return Err(invalid("pull cursor skipped changes"));
        }
        Ok(())
    }

    fn accept(&mut self, incoming: Accepted) -> Result<(), SyncError> {
        let key = incoming.pending.change.key();
        if let Some(current) = self
            .accepted
            .iter_mut()
            .find(|row| row.pending.change.key() == key)
        {
            if incoming.stamp == current.stamp && incoming != *current {
                return Err(invalid("server stamp has two values"));
            }
            if incoming.stamp > current.stamp {
                *current = incoming;
            }
        } else {
            self.accepted.push(incoming);
        }
        Ok(())
    }

    fn wire_id(&mut self, local: DeedId) -> Result<DeedKey, SyncError> {
        if let Some(binding) = self.bindings.iter().find(|binding| binding.local == local) {
            return Ok(binding.remote.clone());
        }
        let sequence = self.next_deed;
        self.next_deed = sequence.checked_add(1).ok_or(SyncError::Exhausted)?;
        let remote = DeedKey {
            install: self.install.clone(),
            sequence,
        };
        self.bindings.push(Binding {
            local,
            remote: remote.clone(),
        });
        Ok(remote)
    }

    fn local_id(&mut self, remote: &DeedKey, records: &mut OwnerData) -> Result<DeedId, SyncError> {
        if let Some(binding) = self
            .bindings
            .iter()
            .find(|binding| &binding.remote == remote)
        {
            return Ok(binding.local);
        }
        let candidate = records.next_deed_id();
        if candidate.0 <= records.last_deed_id
            || records.deleted_deeds.contains(&candidate)
            || records.deeds.iter().any(|deed| deed.id == candidate)
            || self
                .bindings
                .iter()
                .any(|binding| binding.local == candidate)
        {
            return Err(SyncError::Exhausted);
        }
        records.last_deed_id = candidate.0;
        self.bindings.push(Binding {
            local: candidate,
            remote: remote.clone(),
        });
        Ok(candidate)
    }
}

fn validate_accepted(accepted: &Accepted, cursor: u64) -> Result<(), SyncError> {
    rows::validate_pending(&accepted.pending)?;
    if accepted.stamp.sequence == 0 || accepted.stamp.sequence > cursor {
        return Err(invalid("server stamp outside cursor"));
    }
    Ok(())
}
