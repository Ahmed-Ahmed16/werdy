//! App state: the owner's data, the day on screen, and what each tap does.
//! It calls werdy-core for every number and holds no scoring of its own.

use werdy_core::{
    DailySheet, DeedId, Mark, MarkEntry, SlotKey, TodaySummary, WerdyDay, daily_sheet,
    today_summary,
};
use werdy_store::{DeedName, OwnerData, Store};

/// One of the three mark buttons on a deed row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tap {
    /// The check button.
    Done,
    /// The half circle button.
    Partial,
    /// The cross button.
    Missed,
}

/// One press on a count stepper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    /// Add one.
    Plus,
    /// Take one away.
    Minus,
}

/// Returns the mark after a tap. Tapping the button that is already on clears it.
pub fn tapped(current: Option<Mark>, tap: Tap) -> Option<Mark> {
    if is_on(current, tap) {
        return None;
    }
    Some(match tap {
        Tap::Done => Mark::Done,
        Tap::Partial => Mark::partial_without_target(),
        Tap::Missed => Mark::Missed,
    })
}

/// Returns the mark after a stepper press. Taking away the last one clears the
/// mark; Missed is its own button, so the stepper never stores a zero count.
pub fn stepped(current: Option<Mark>, step: Step) -> Option<Mark> {
    let count = shown_count(current);
    match step {
        Step::Plus => Some(Mark::Count(count.saturating_add(1))),
        Step::Minus if count > 1 => Some(Mark::Count(count - 1)),
        Step::Minus => None,
    }
}

/// Says whether a mark button shows as on for this mark.
pub fn is_on(current: Option<Mark>, tap: Tap) -> bool {
    matches!(
        (current, tap),
        (Some(Mark::Done), Tap::Done)
            | (Some(Mark::Partial(_)), Tap::Partial)
            | (Some(Mark::Missed | Mark::Count(0)), Tap::Missed)
    )
}

/// Returns the count a stepper shows for this mark.
pub fn shown_count(current: Option<Mark>) -> u32 {
    match current {
        Some(Mark::Count(count)) => count,
        _ => 0,
    }
}

/// Whether the owner's data can be saved, and if the last save worked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveStatus {
    /// Everything is saved.
    Saved,
    /// The last save failed. The next change tries again.
    SaveFailed,
    /// Saved data could not be read, so nothing is written over it.
    LoadFailed,
}

/// Everything the screen shows, in one place.
#[derive(Clone, Debug, PartialEq)]
pub struct AppState {
    /// The owner's records and settings.
    pub data: OwnerData,
    /// The current werdy day.
    pub today: WerdyDay,
    /// The day on screen. Never after today.
    pub viewing: WerdyDay,
    /// Time slot order from the data file.
    pub slot_order: Vec<SlotKey>,
    /// Save state shown in a banner when something went wrong.
    pub save_status: SaveStatus,
}

impl AppState {
    /// Loads saved data. On first run, fills the sheet with the starter set and
    /// saves it. If saved data cannot be read, runs on the starter set in
    /// memory and never writes over the saved data.
    pub fn start<E>(
        store: &dyn Store,
        today: WerdyDay,
        slot_order: Vec<SlotKey>,
        starter: impl FnOnce(WerdyDay) -> Result<OwnerData, E>,
    ) -> Self {
        let (data, save_status) = match store.load() {
            Ok(Some(data)) => (data, SaveStatus::Saved),
            Ok(None) => {
                let data = starter(today).unwrap_or_default();
                let status = match store.save(&data) {
                    Ok(()) => SaveStatus::Saved,
                    Err(_) => SaveStatus::SaveFailed,
                };
                (data, status)
            }
            Err(_) => (starter(today).unwrap_or_default(), SaveStatus::LoadFailed),
        };
        Self {
            data,
            today,
            viewing: today,
            slot_order,
            save_status,
        }
    }

    /// Returns the sheet for the day on screen.
    pub fn sheet(&self) -> DailySheet {
        daily_sheet(
            &self.data.deeds,
            &self.data.marks,
            self.viewing,
            &self.slot_order,
        )
    }

    /// Returns done, partial and open counts for the day on screen.
    pub fn summary(&self) -> TodaySummary {
        today_summary(&self.data.deeds, &self.data.marks, self.viewing)
    }

    /// Says whether the day on screen is today.
    pub fn is_today(&self) -> bool {
        self.viewing == self.today
    }

    /// Returns the stored name of a deed.
    pub fn deed_name(&self, deed_id: DeedId) -> Option<&DeedName> {
        self.data.names.get(&deed_id)
    }

    /// Returns the mark for a deed on the day on screen.
    pub fn mark(&self, deed_id: DeedId) -> Option<Mark> {
        self.data
            .marks
            .get(&(deed_id, self.viewing))
            .map(|entry| entry.mark)
    }

    /// Moves to the day before. Stays put at the day count limit.
    pub fn show_previous_day(&mut self) {
        if let Ok(day) = self.viewing.previous() {
            self.viewing = day;
        }
    }

    /// Moves to the next day, but never past today.
    pub fn show_next_day(&mut self) {
        if let Ok(day) = self.viewing.next()
            && day <= self.today
        {
            self.viewing = day;
        }
    }

    /// Jumps back to today.
    pub fn show_today(&mut self) {
        self.viewing = self.today;
    }

    /// Applies a mark button tap on the day on screen.
    pub fn tap(&mut self, deed_id: DeedId, tap: Tap) {
        let mark = tapped(self.mark(deed_id), tap);
        self.set_mark(deed_id, mark);
    }

    /// Applies a stepper press on the day on screen.
    pub fn step(&mut self, deed_id: DeedId, step: Step) {
        let mark = stepped(self.mark(deed_id), step);
        self.set_mark(deed_id, mark);
    }

    /// Switches the interface language and keeps it in settings.
    pub fn set_language(&mut self, code: &str) {
        code.clone_into(&mut self.data.settings.language);
    }

    fn set_mark(&mut self, deed_id: DeedId, mark: Option<Mark>) {
        let key = (deed_id, self.viewing);
        match mark {
            // The entry day is today, so core can flag a late mark (ADR 0021).
            Some(mark) => {
                self.data
                    .marks
                    .insert(key, MarkEntry::new(mark, self.today));
            }
            None => {
                self.data.marks.remove(&key);
            }
        }
    }

    /// Saves the data, unless saved data could not be read at start.
    pub fn save_to(&mut self, store: &dyn Store) {
        if self.save_status == SaveStatus::LoadFailed {
            return;
        }
        self.save_status = match store.save(&self.data) {
            Ok(()) => SaveStatus::Saved,
            Err(_) => SaveStatus::SaveFailed,
        };
    }
}

#[cfg(test)]
mod tests {
    use werdy_core::{DeedId, Error, Mark, WerdyDay};
    use werdy_store::{MemoryStore, OwnerData, Store, StoreError};

    use super::{AppState, SaveStatus, Step, Tap, is_on, stepped, tapped};
    use crate::starter::{five_prayers, slot_order};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Store whose load and save always fail, like blocked browser storage.
    struct BrokenStore;

    impl Store for BrokenStore {
        fn load(&self) -> Result<Option<OwnerData>, StoreError> {
            Err(StoreError::Corrupt("test".to_owned()))
        }

        fn save(&self, _data: &OwnerData) -> Result<(), StoreError> {
            Err(StoreError::Backend("test".to_owned()))
        }
    }

    fn today() -> Result<WerdyDay, Error> {
        WerdyDay::new(2026, 10, 27)
    }

    fn started(store: &dyn Store) -> Result<AppState, Error> {
        Ok(AppState::start(store, today()?, slot_order(), five_prayers))
    }

    #[test]
    fn a_tap_sets_the_mark_and_a_second_tap_clears_it() {
        assert_eq!(tapped(None, Tap::Done), Some(Mark::Done));
        assert_eq!(tapped(Some(Mark::Done), Tap::Done), None);
        assert_eq!(
            tapped(None, Tap::Partial),
            Some(Mark::partial_without_target())
        );
        assert_eq!(
            tapped(Some(Mark::partial_without_target()), Tap::Partial),
            None
        );
        assert_eq!(tapped(Some(Mark::Missed), Tap::Missed), None);
    }

    #[test]
    fn a_tap_on_another_button_switches_the_mark() {
        assert_eq!(tapped(Some(Mark::Done), Tap::Missed), Some(Mark::Missed));
        assert_eq!(
            tapped(Some(Mark::Count(3)), Tap::Missed),
            Some(Mark::Missed)
        );
        assert_eq!(tapped(Some(Mark::Count(0)), Tap::Missed), None);
    }

    #[test]
    fn the_stepper_counts_up_and_clears_below_one() {
        assert_eq!(stepped(None, Step::Plus), Some(Mark::Count(1)));
        assert_eq!(
            stepped(Some(Mark::Missed), Step::Plus),
            Some(Mark::Count(1))
        );
        assert_eq!(
            stepped(Some(Mark::Count(4)), Step::Plus),
            Some(Mark::Count(5))
        );
        assert_eq!(
            stepped(Some(Mark::Count(2)), Step::Minus),
            Some(Mark::Count(1))
        );
        assert_eq!(stepped(Some(Mark::Count(1)), Step::Minus), None);
        assert_eq!(stepped(None, Step::Minus), None);
    }

    #[test]
    fn only_the_matching_button_shows_as_on() {
        assert!(is_on(Some(Mark::Done), Tap::Done));
        assert!(!is_on(Some(Mark::Done), Tap::Partial));
        assert!(is_on(Some(Mark::partial_without_target()), Tap::Partial));
        assert!(is_on(Some(Mark::Count(0)), Tap::Missed));
        assert!(!is_on(None, Tap::Missed));
    }

    #[test]
    fn first_run_fills_the_five_prayers_and_saves_them() -> TestResult {
        let store = MemoryStore::default();

        let state = started(&store)?;

        assert_eq!(state.data.deeds.len(), 5);
        assert_eq!(state.save_status, SaveStatus::Saved);
        assert_eq!(store.load()?, Some(state.data));
        Ok(())
    }

    #[test]
    fn a_mark_is_saved_and_comes_back_on_the_next_start() -> TestResult {
        let store = MemoryStore::default();
        let mut state = started(&store)?;

        state.tap(DeedId(1), Tap::Done);
        state.save_to(&store);
        let again = started(&store)?;

        assert_eq!(again.mark(DeedId(1)), Some(Mark::Done));
        assert_eq!(again.summary().done, 1);
        assert_eq!(again.summary().open, 4);
        Ok(())
    }

    #[test]
    fn unreadable_saved_data_is_never_written_over() -> TestResult {
        let store = MemoryStore::default();
        let mut state = AppState::start(&BrokenStore, today()?, slot_order(), five_prayers);

        state.tap(DeedId(1), Tap::Done);
        state.save_to(&store);

        assert_eq!(state.save_status, SaveStatus::LoadFailed);
        assert_eq!(state.data.deeds.len(), 5);
        assert_eq!(store.load()?, None);
        Ok(())
    }

    #[test]
    fn a_failed_save_is_reported() -> TestResult {
        let mut state = started(&MemoryStore::default())?;

        state.tap(DeedId(1), Tap::Done);
        state.save_to(&BrokenStore);

        assert_eq!(state.save_status, SaveStatus::SaveFailed);
        Ok(())
    }

    #[test]
    fn the_next_day_stops_at_today() -> TestResult {
        let mut state = started(&MemoryStore::default())?;

        state.show_next_day();
        assert_eq!(state.viewing, today()?);

        state.show_previous_day();
        state.show_previous_day();
        state.show_next_day();
        assert_eq!(state.viewing, today()?.previous()?);
        assert!(!state.is_today());

        state.show_today();
        assert!(state.is_today());
        Ok(())
    }

    #[test]
    fn a_mark_on_a_past_day_records_today_as_its_entry_day() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        state.show_previous_day();
        let yesterday = state.viewing;

        state.tap(DeedId(2), Tap::Missed);

        let entry = state.data.marks.get(&(DeedId(2), yesterday));
        assert_eq!(entry.map(|entry| entry.entered_on), Some(today()?));
        Ok(())
    }
}
