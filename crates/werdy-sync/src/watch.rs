use crate::{Batch, Sync, SyncError, rows};
use werdy_store::OwnerData;

/// An isolated projection of one owner's stream. No capture, push or mutable
/// records API is exposed. Drop this view when access is refused or left.
#[derive(Clone)]
pub struct WatchView {
    engine: Sync,
    records: OwnerData,
}
impl PartialEq for WatchView {
    fn eq(&self, other: &Self) -> bool {
        self.cursor() == other.cursor() && self.records == other.records
    }
}

impl WatchView {
    pub fn new() -> Result<Self, SyncError> {
        Ok(Self {
            engine: Sync::new("watch_view")?,
            records: OwnerData::default(),
        })
    }

    pub fn cursor(&self) -> u64 {
        self.engine.cursor
    }
    pub fn records(&self) -> &OwnerData {
        &self.records
    }

    /// Pages must be complete through their cursor, including deferred headers.
    /// Bad input leaves both records and cursor untouched.
    pub fn apply(&mut self, page: Batch) -> Result<(), SyncError> {
        self.engine.validate_pull(&page, self.cursor())?;
        let mut staged = self.clone();
        for row in page.changes {
            staged.engine.accept(row)?;
        }
        rows::project(&mut staged.engine, &mut staged.records)?;
        staged.engine.cursor = page.cursor;
        *self = staged;
        Ok(())
    }
}
