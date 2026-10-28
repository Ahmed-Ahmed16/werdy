use crate::{Accepted, Backend, Batch, Pending, Stamp, SyncError, invalid, rows};

/// One in memory account with a controllable Unix millisecond clock.
/// Stamps never move backwards even when the fake wall clock does.
/// The append log also serves as the retry receipt ledger.
#[derive(Clone, Debug, Default)]
pub struct FakeBackend {
    now: u64,
    log: Vec<Accepted>,
    fail_next_push: bool,
}

impl FakeBackend {
    pub fn new(now: u64) -> Self {
        Self {
            now,
            ..Self::default()
        }
    }

    pub fn set_time(&mut self, now: u64) {
        self.now = now;
    }

    /// Simulates failure before accepting any part of the next batch.
    pub fn fail_next_push(&mut self) {
        self.fail_next_push = true;
    }

    fn cursor(&self) -> u64 {
        self.log.last().map_or(0, |row| row.stamp.sequence)
    }

    fn time(&self) -> u64 {
        self.now
            .max(self.log.last().map_or(0, |row| row.stamp.received_at))
    }

    fn accept(&mut self, pending: &Pending) -> Result<Accepted, SyncError> {
        rows::validate_pending(pending)?;
        if let Some(previous) = self.log.iter().find(|row| {
            row.pending.install == pending.install && row.pending.operation == pending.operation
        }) {
            if previous.pending != *pending {
                return Err(invalid("operation id reused for different change"));
            }
            return Ok(previous.clone());
        }
        let accepted = Accepted {
            pending: pending.clone(),
            stamp: Stamp {
                received_at: self.time(),
                sequence: self.cursor().checked_add(1).ok_or(SyncError::Exhausted)?,
            },
        };
        self.log.push(accepted.clone());
        Ok(accepted)
    }
}

impl Backend for FakeBackend {
    async fn push(&mut self, changes: &[Pending]) -> Result<Batch, SyncError> {
        if self.fail_next_push {
            self.fail_next_push = false;
            return Err(SyncError::Backend("fake push failed".to_owned()));
        }
        let mut staged = self.clone();
        let mut accepted = Vec::new();
        for pending in changes {
            accepted.push(staged.accept(pending)?);
        }
        *self = staged;
        Ok(Batch {
            changes: accepted,
            cursor: self.cursor(),
        })
    }

    async fn pull(&mut self, cursor: u64) -> Result<Batch, SyncError> {
        if cursor > self.cursor() {
            return Err(invalid("cursor is ahead of server"));
        }
        Ok(Batch {
            changes: self
                .log
                .iter()
                .filter(|row| row.stamp.sequence > cursor)
                .cloned()
                .collect(),
            cursor: self.cursor(),
        })
    }

    async fn server_time(&mut self) -> Result<u64, SyncError> {
        Ok(self.time())
    }
}
