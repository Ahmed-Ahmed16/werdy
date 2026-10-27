//! App state: the owner's data, the day on screen, and what each tap does.
//! It calls werdy-core for every number and holds no scoring of its own.

use std::collections::{BTreeMap, BTreeSet};

use werdy_core::{
    CategoryKey, DailySheet, DayNote, Deed, DeedId, DeedRule, Error, ExcuseCredit, Mark, MarkEntry,
    Period, Report, Schedule, Score, ScoreBands, SlotKey, Target, TodaySummary, Weekday, WerdyDay,
    build_report, convert_mark_for_target, daily_sheet, delete_deed, finished_day_summary, score,
    today_summary,
};
use werdy_store::{DeedName, OwnerData, Store, merge_records};

/// Keeps the owner's past day in view, or follows today when today moves.
pub fn viewing_after_today_change(
    old_today: WerdyDay,
    new_today: WerdyDay,
    viewed: WerdyDay,
) -> WerdyDay {
    if viewed == old_today {
        new_today
    } else {
        viewed.min(new_today)
    }
}

/// Approximate local time slots until real prayer times arrive in ticket #11.
pub fn current_slot(hour: u8, minute: u8) -> &'static str {
    // TODO(#11): replace fixed bounds with computed prayer times.
    let time = u16::from(hour) * 60 + u16::from(minute);
    match time {
        240..360 => "fajr",
        360..720 => "morning",
        720..930 => "dhuhr",
        930..1080 => "asr",
        1080..1170 => "maghrib",
        1170..1320 => "isha",
        _ => "night",
    }
}

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

/// Whether the owner's last change could be made and saved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveStatus {
    /// Everything is saved.
    Saved,
    /// The last save failed. The next change tries again.
    SaveFailed,
    /// Saved data could not be read, so nothing is written over it.
    LoadFailed,
    /// Today's date could not be read, so a change was not made.
    DateFailed,
}

/// Screen selected in the navigation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    /// Daily sheet.
    Sheet,
    /// Reports for finished days.
    Reports,
    /// Deed list.
    Deeds,
    /// New or existing deed editor.
    Editor(Option<DeedId>),
}

/// Editable fields. A new deed starts daily, at any time, with no target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorDraft {
    pub name: String,
    /// Name shown when editing began, including its translation.
    pub original_name: Option<String>,
    pub category: CategoryKey,
    pub slot: SlotKey,
    pub schedule: Schedule,
    pub target_enabled: bool,
    pub target_amount: u32,
    pub target_unit: String,
}

impl Default for EditorDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            original_name: None,
            category: CategoryKey::new("other"),
            slot: SlotKey::new("any_time"),
            schedule: Schedule::EveryDay,
            target_enabled: false,
            target_amount: 1,
            target_unit: "times".to_owned(),
        }
    }
}

impl EditorDraft {
    pub fn weekdays(&mut self) {
        self.schedule = Schedule::Weekdays(BTreeSet::new());
    }

    pub fn toggle_weekday(&mut self, weekday: Weekday) {
        if let Schedule::Weekdays(days) = &mut self.schedule
            && !days.insert(weekday)
        {
            days.remove(&weekday);
        }
    }

    fn rule(&self) -> Result<DeedRule, EditorError> {
        if matches!(&self.schedule, Schedule::Weekdays(days) if days.is_empty()) {
            return Err(EditorError::NoWeekday);
        }
        let target = self
            .target_enabled
            .then(|| Target::new(self.target_amount, &self.target_unit))
            .transpose()
            .map_err(|_| EditorError::InvalidTarget)?;
        Ok(DeedRule {
            schedule: self.schedule.clone(),
            target,
            category: self.category.clone(),
            slot: self.slot.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorError {
    EmptyName,
    NoWeekday,
    InvalidTarget,
    MissingDeed,
    RuleChange,
    DeleteNotConfirmed,
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EditorError {}

/// One pending change to a deed until storage accepts it.
#[derive(Clone, Debug, PartialEq)]
enum DeedChange {
    Upsert {
        deed: Box<Deed>,
        name: DeedName,
        created: bool,
    },
    Delete,
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
    /// Screen on view.
    pub screen: Screen,
    /// Selected rolling report period.
    pub report_period: Period,
    /// Number of whole periods before the latest one.
    pub report_steps_back: usize,
    settings_dirty: bool,
    deed_changes: BTreeMap<DeedId, DeedChange>,
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
            screen: Screen::Sheet,
            report_period: Period::Week,
            report_steps_back: 0,
            settings_dirty: false,
            deed_changes: BTreeMap::new(),
        }
    }

    /// Returns the sheet for the day on screen.
    pub fn sheet(&self) -> DailySheet {
        daily_sheet(
            &self.data.deeds,
            &self.data.marks,
            self.viewing,
            &self.slot_order,
            self.data.settings.calendar.clone(),
        )
    }

    /// Returns done, partial and open or missed counts for the day on screen.
    pub fn summary(&self) -> TodaySummary {
        let summarize = if self.is_today() {
            today_summary
        } else {
            finished_day_summary
        };
        summarize(
            &self.data.deeds,
            &self.data.marks,
            self.viewing,
            &self.data.settings.calendar,
        )
    }

    /// Today's live counts, even when the sheet is viewing a past day.
    pub fn summary_today(&self) -> TodaySummary {
        today_summary(
            &self.data.deeds,
            &self.data.marks,
            self.today,
            &self.data.settings.calendar,
        )
    }

    /// Core score for the viewed finished day. Today has no score.
    pub fn viewing_score(&self) -> Result<Option<Score>, Error> {
        if self.is_today() {
            return Ok(None);
        }
        score(&self.data.records(), &[self.viewing], self.today).map(Some)
    }

    /// Note attached to the viewed day.
    pub fn viewing_note(&self) -> Option<&DayNote> {
        self.data.day_notes.get(&self.viewing)
    }

    /// Saves a note with the current day as its entry day.
    pub fn save_day_note(
        &mut self,
        day: WerdyDay,
        text: &str,
        credit: ExcuseCredit,
    ) -> Result<(), Error> {
        let note = DayNote::new(text, credit, self.today)?;
        self.data
            .set_day_note(day, Some(note), crate::platform::change_time());
        Ok(())
    }

    /// Deletes a note on its chosen day.
    pub fn delete_day_note(&mut self, day: WerdyDay) {
        self.data
            .set_day_note(day, None, crate::platform::change_time());
    }

    /// Builds the selected report from saved records before it is rendered.
    pub fn report(&self) -> Result<Report, Error> {
        build_report(
            &self.data.records(),
            self.report_period,
            self.report_steps_back,
            self.today,
            ScoreBands::default(),
        )
    }

    /// Builds the adjacent earlier report for comparison.
    pub fn previous_report(&self) -> Result<Report, Error> {
        build_report(
            &self.data.records(),
            self.report_period,
            self.report_steps_back.saturating_add(1),
            self.today,
            ScoreBands::default(),
        )
    }

    /// Opens one screen.
    pub fn show_screen(&mut self, screen: Screen) {
        self.screen = screen;
    }

    /// Selects a period and returns to its latest window.
    pub fn select_report_period(&mut self, period: Period) {
        self.report_period = period;
        self.report_steps_back = 0;
    }

    /// Moves one whole period into the past.
    pub fn show_previous_period(&mut self) {
        self.report_steps_back = self.report_steps_back.saturating_add(1);
    }

    /// Moves one whole period toward yesterday, stopping at the latest window.
    pub fn show_next_period(&mut self) {
        self.report_steps_back = self.report_steps_back.saturating_sub(1);
    }

    /// Says whether the day on screen is today.
    pub fn is_today(&self) -> bool {
        self.viewing == self.today
    }

    /// Returns the stored name of a deed.
    pub fn deed_name(&self, deed_id: DeedId) -> Option<&DeedName> {
        self.data.names.get(&deed_id)
    }

    pub fn draft_for(&self, id: DeedId) -> Option<EditorDraft> {
        let deed = self.data.deeds.iter().find(|deed| deed.id == id)?;
        let rule = deed.rule_on(self.today)?;
        let name = match self.data.names.get(&id)? {
            DeedName::Text(text) | DeedName::Key(text) => text.clone(),
        };
        Some(EditorDraft {
            name,
            original_name: None,
            category: rule.category.clone(),
            slot: rule.slot.clone(),
            schedule: rule.schedule.clone(),
            target_enabled: rule.target.is_some(),
            target_amount: rule.target.as_ref().map_or(1, Target::amount),
            target_unit: rule
                .target
                .as_ref()
                .map_or_else(|| "times".to_owned(), |target| target.unit().to_owned()),
        })
    }

    pub fn save_deed(
        &mut self,
        id: Option<DeedId>,
        draft: &EditorDraft,
    ) -> Result<DeedId, EditorError> {
        let name = draft.name.trim();
        if name.is_empty() {
            return Err(EditorError::EmptyName);
        }
        let rule = draft.rule()?;
        let created = id.is_none();
        let id = if let Some(id) = id {
            let deed = self
                .data
                .deeds
                .iter_mut()
                .find(|deed| deed.id == id)
                .ok_or(EditorError::MissingDeed)?;
            let old_target = deed
                .rule_on(self.today)
                .and_then(|rule| rule.target.clone());
            let new_target = rule.target.clone();
            deed.change_rule(self.today, rule)
                .map_err(|_| EditorError::RuleChange)?;
            if old_target.is_some() != new_target.is_some()
                && let Some(entry) = self.data.marks.get(&(id, self.today)).copied()
            {
                let mark =
                    convert_mark_for_target(entry.mark, old_target.as_ref(), new_target.as_ref());
                self.data.set_mark(
                    id,
                    self.today,
                    Some(MarkEntry::new(mark, self.today)),
                    crate::platform::change_time(),
                );
            }
            id
        } else {
            let id = self.data.next_deed_id();
            if id.0 <= self.data.last_deed_id
                || self.data.deleted_deeds.contains(&id)
                || self.data.deeds.iter().any(|deed| deed.id == id)
            {
                return Err(EditorError::RuleChange);
            }
            self.data.last_deed_id = id.0;
            self.data.deeds.push(Deed::new(id, self.today, rule));
            id
        };
        let name = match self.data.names.get(&id) {
            Some(DeedName::Key(key))
                if draft.original_name.as_deref() == Some(name) || key == name =>
            {
                DeedName::Key(key.clone())
            }
            _ => DeedName::Text(name.to_owned()),
        };
        self.data.names.insert(id, name.clone());
        if let Some(deed) = self.data.deeds.iter().find(|deed| deed.id == id) {
            let created = created
                || matches!(
                    self.deed_changes.get(&id),
                    Some(DeedChange::Upsert { created: true, .. })
                );
            self.deed_changes.insert(
                id,
                DeedChange::Upsert {
                    deed: Box::new(deed.clone()),
                    name,
                    created,
                },
            );
        }
        Ok(id)
    }

    pub fn archive_deed(&mut self, id: DeedId) -> Result<(), EditorError> {
        let deed = self
            .data
            .deeds
            .iter_mut()
            .find(|deed| deed.id == id)
            .ok_or(EditorError::MissingDeed)?;
        deed.archive(self.today)
            .map_err(|_| EditorError::RuleChange)?;
        self.record_upsert(id);
        Ok(())
    }

    pub fn restore_deed(&mut self, id: DeedId) -> Result<(), EditorError> {
        let deed = self
            .data
            .deeds
            .iter_mut()
            .find(|deed| deed.id == id)
            .ok_or(EditorError::MissingDeed)?;
        deed.restore(self.today)
            .map_err(|_| EditorError::RuleChange)?;
        self.record_upsert(id);
        Ok(())
    }

    pub fn delete_deed_confirmed(
        &mut self,
        id: DeedId,
        confirmed: bool,
    ) -> Result<(), EditorError> {
        if !confirmed {
            return Err(EditorError::DeleteNotConfirmed);
        }
        if !self.data.deeds.iter().any(|deed| deed.id == id) {
            return Err(EditorError::MissingDeed);
        }
        let unsaved = matches!(
            self.deed_changes.get(&id),
            Some(DeedChange::Upsert { created: true, .. })
        );
        delete_deed(&mut self.data.deeds, &mut self.data.marks, id);
        self.data.names.remove(&id);
        self.data
            .mark_revisions
            .retain(|(deed_id, _), _| *deed_id != id);
        self.data
            .cleared_marks
            .retain(|(deed_id, _), _| *deed_id != id);
        if unsaved {
            self.deed_changes.remove(&id);
        } else {
            self.data.deleted_deeds.insert(id);
            self.deed_changes.insert(id, DeedChange::Delete);
        }
        Ok(())
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

    /// Reads a fresh day without moving a past day the owner chose.
    pub fn update_today(&mut self, today: WerdyDay) {
        self.viewing = viewing_after_today_change(self.today, today, self.viewing);
        self.today = today;
    }

    /// Applies a change only after today's date is read, so marks use that day.
    pub fn change_on_day(
        &mut self,
        today: Result<WerdyDay, werdy_core::Error>,
        change: impl FnOnce(&mut Self),
    ) -> bool {
        match today {
            Ok(today) => {
                self.update_today(today);
                change(self);
                true
            }
            Err(_) => {
                self.save_status = SaveStatus::DateFailed;
                false
            }
        }
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
        self.settings_dirty = true;
    }

    fn set_mark(&mut self, deed_id: DeedId, mark: Option<Mark>) {
        // The entry day is today, so core can flag a late mark (ADR 0021).
        self.data.set_mark(
            deed_id,
            self.viewing,
            mark.map(|mark| MarkEntry::new(mark, self.today)),
            crate::platform::change_time(),
        );
    }

    fn record_upsert(&mut self, id: DeedId) {
        if let Some(deed) = self.data.deeds.iter().find(|deed| deed.id == id)
            && let Some(name) = self.data.names.get(&id)
        {
            let created = matches!(
                self.deed_changes.get(&id),
                Some(DeedChange::Upsert { created: true, .. })
            );
            self.deed_changes.insert(
                id,
                DeedChange::Upsert {
                    deed: Box::new(deed.clone()),
                    name: name.clone(),
                    created,
                },
            );
        }
    }

    fn rebase_new_deeds(&mut self, stored: &OwnerData) -> bool {
        let collisions: Vec<_> = self
            .deed_changes
            .iter()
            .filter_map(|(id, change)| {
                (matches!(change, DeedChange::Upsert { created: true, .. })
                    && (stored.deeds.iter().any(|deed| deed.id == *id)
                        || stored.deleted_deeds.contains(id)))
                .then_some(*id)
            })
            .collect();
        for old_id in collisions {
            let new_id = DeedId(self.data.next_deed_id().0.max(stored.next_deed_id().0));
            if new_id == old_id || new_id.0 <= self.data.last_deed_id.max(stored.last_deed_id) {
                return false;
            }
            self.move_deed_id(old_id, new_id);
        }
        true
    }

    fn move_deed_id(&mut self, old_id: DeedId, new_id: DeedId) {
        self.data.last_deed_id = new_id.0;
        if let Some(deed) = self.data.deeds.iter_mut().find(|deed| deed.id == old_id) {
            deed.id = new_id;
        }
        if let Some(name) = self.data.names.remove(&old_id) {
            self.data.names.insert(new_id, name);
        }
        self.move_mark_ids(old_id, new_id);
        if let Some(DeedChange::Upsert { mut deed, name, .. }) = self.deed_changes.remove(&old_id) {
            deed.id = new_id;
            self.deed_changes.insert(
                new_id,
                DeedChange::Upsert {
                    deed,
                    name,
                    created: true,
                },
            );
        }
    }

    fn move_mark_ids(&mut self, old_id: DeedId, new_id: DeedId) {
        let days: Vec<_> = self
            .data
            .marks
            .keys()
            .filter_map(|(id, day)| (*id == old_id).then_some(*day))
            .collect();
        for day in days {
            if let Some(entry) = self.data.marks.remove(&(old_id, day)) {
                self.data.marks.insert((new_id, day), entry);
            }
            if let Some(revision) = self.data.mark_revisions.remove(&(old_id, day)) {
                self.data.mark_revisions.insert((new_id, day), revision);
            }
        }
        let cleared: Vec<_> = self
            .data
            .cleared_marks
            .keys()
            .filter_map(|(id, day)| (*id == old_id).then_some(*day))
            .collect();
        for day in cleared {
            if let Some(revision) = self.data.cleared_marks.remove(&(old_id, day)) {
                self.data.cleared_marks.insert((new_id, day), revision);
            }
        }
    }

    fn apply_deed_changes(&self, merged: &mut OwnerData) {
        for (id, change) in &self.deed_changes {
            match change {
                DeedChange::Upsert { deed, name, .. } if !merged.deleted_deeds.contains(id) => {
                    if let Some(existing) =
                        merged.deeds.iter_mut().find(|existing| existing.id == *id)
                    {
                        *existing = *deed.clone();
                    } else {
                        merged.deeds.push(*deed.clone());
                    }
                    merged.names.insert(*id, name.clone());
                }
                DeedChange::Delete => {
                    merged.deleted_deeds.insert(*id);
                    delete_deed(&mut merged.deeds, &mut merged.marks, *id);
                    merged.names.remove(id);
                    merged
                        .mark_revisions
                        .retain(|(deed_id, _), _| deed_id != id);
                    merged.cleared_marks.retain(|(deed_id, _), _| deed_id != id);
                }
                _ => {}
            }
        }
        merged.last_deed_id = merged.last_deed_id.max(self.data.last_deed_id);
    }

    /// Saves the data, unless saved data could not be read at start.
    pub fn save_to(&mut self, store: &dyn Store) {
        if self.save_status == SaveStatus::LoadFailed {
            return;
        }
        self.save_status = match store.load() {
            Ok(stored) => {
                if let Some(ref data) = stored
                    && !self.rebase_new_deeds(data)
                {
                    self.save_status = SaveStatus::SaveFailed;
                    return;
                }
                let mut merged = stored.map_or_else(
                    || self.data.clone(),
                    |data| merge_records(&data, &self.data),
                );
                if self.settings_dirty {
                    merged.settings = self.data.settings.clone();
                }
                self.apply_deed_changes(&mut merged);
                let live_ids: BTreeSet<_> = merged.deeds.iter().map(|deed| deed.id).collect();
                merged.marks.retain(|(id, _), _| live_ids.contains(id));
                merged
                    .mark_revisions
                    .retain(|(id, _), _| live_ids.contains(id));
                merged
                    .cleared_marks
                    .retain(|(id, _), _| live_ids.contains(id));
                match store.save(&merged) {
                    Ok(()) => {
                        self.data = merged;
                        self.settings_dirty = false;
                        self.deed_changes.clear();
                        SaveStatus::Saved
                    }
                    Err(_) => SaveStatus::SaveFailed,
                }
            }
            Err(_) => SaveStatus::SaveFailed,
        };
    }

    /// Reloads another tab's save and keeps local changes that are newer.
    pub fn reload_from(&mut self, store: &dyn Store) {
        if self.save_status == SaveStatus::LoadFailed {
            return;
        }
        match store.load() {
            Ok(Some(stored)) => {
                if !self.rebase_new_deeds(&stored) {
                    self.save_status = SaveStatus::SaveFailed;
                    return;
                }
                let mut merged = merge_records(&stored, &self.data);
                if self.settings_dirty {
                    merged.settings = self.data.settings.clone();
                }
                self.apply_deed_changes(&mut merged);
                if merged == stored {
                    self.data = stored;
                    self.save_status = SaveStatus::Saved;
                    self.deed_changes.clear();
                    self.settings_dirty = false;
                } else {
                    self.data = merged;
                    self.save_to(store);
                }
            }
            Ok(None) => {}
            Err(_) => self.save_status = SaveStatus::LoadFailed,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use werdy_core::{DeedId, Error, ExcuseCredit, Mark, Period, WerdyDay};
    use werdy_store::{MemoryStore, OwnerData, Store, StoreError};

    use super::{
        AppState, EditorDraft, EditorError, SaveStatus, Screen, Step, Tap, is_on, stepped, tapped,
    };
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
    fn add_and_edit_deed_save_rules_from_today_without_moving_past_due_days() -> TestResult {
        let store = MemoryStore::default();
        let mut state = started(&store)?;
        let mut draft = EditorDraft {
            name: "Reading".to_owned(),
            ..EditorDraft::default()
        };
        let id = state.save_deed(None, &draft)?;
        state.save_to(&store);
        let created = today()?;
        assert!(
            state
                .data
                .deeds
                .iter()
                .find(|deed| deed.id == id)
                .is_some_and(|deed| deed.is_due_on(created, &state.data.settings.calendar))
        );
        let later = today()?.next()?;
        state.update_today(later);
        draft.name = "Friday reading".to_owned();
        draft.weekdays();
        draft.toggle_weekday(werdy_core::Weekday::Friday);
        state.save_deed(Some(id), &draft)?;
        state.save_to(&store);
        let saved = started(&store)?;
        let deed = saved
            .data
            .deeds
            .iter()
            .find(|deed| deed.id == id)
            .ok_or("missing deed")?;
        assert!(deed.is_due_on(today()?, &saved.data.settings.calendar));
        assert_eq!(
            deed.rule_on(later).map(|rule| &rule.schedule),
            Some(&draft.schedule)
        );
        assert_eq!(
            saved.data.names.get(&id),
            Some(&werdy_store::DeedName::Text("Friday reading".to_owned()))
        );
        Ok(())
    }

    #[test]
    fn archive_restore_and_confirmed_delete_change_due_days_and_remove_marks() -> TestResult {
        let store = MemoryStore::default();
        let mut state = started(&store)?;
        let id = DeedId(1);
        state.tap(id, Tap::Done);
        let archived = today()?.next()?;
        state.update_today(archived);
        state.archive_deed(id)?;
        state.save_to(&store);
        assert!(!state.data.deeds[0].is_due_on(archived, &state.data.settings.calendar));
        let restored = archived.next()?;
        state.update_today(restored);
        state.restore_deed(id)?;
        state.save_to(&store);
        let loaded = started(&store)?;
        let deed = &loaded.data.deeds[0];
        assert!(deed.is_due_on(today()?, &loaded.data.settings.calendar));
        assert!(!deed.is_due_on(archived, &loaded.data.settings.calendar));
        assert!(deed.is_due_on(restored, &loaded.data.settings.calendar));
        assert_eq!(
            state.delete_deed_confirmed(id, false),
            Err(EditorError::DeleteNotConfirmed)
        );
        assert!(state.data.marks.contains_key(&(id, today()?)));
        state.delete_deed_confirmed(id, true)?;
        state.save_to(&store);
        let loaded = started(&store)?;
        assert!(!loaded.data.deeds.iter().any(|deed| deed.id == id));
        assert!(!loaded.data.marks.keys().any(|(deed_id, _)| *deed_id == id));
        assert!(!loaded.data.names.contains_key(&id));
        Ok(())
    }

    #[test]
    fn stale_tab_cannot_restore_marks_for_a_deleted_deed() -> TestResult {
        let store = MemoryStore::default();
        let mut deleting = started(&store)?;
        let mut stale = started(&store)?;
        let id = DeedId(1);
        deleting.delete_deed_confirmed(id, true)?;
        deleting.save_to(&store);
        stale.tap(id, Tap::Done);
        stale.save_to(&store);

        let saved = started(&store)?;
        assert!(!saved.data.deeds.iter().any(|deed| deed.id == id));
        assert!(!saved.data.marks.keys().any(|(deed_id, _)| *deed_id == id));
        Ok(())
    }

    struct FailOnceStore {
        inner: MemoryStore,
        fail_next: Cell<bool>,
    }

    impl Store for FailOnceStore {
        fn load(&self) -> Result<Option<OwnerData>, StoreError> {
            self.inner.load()
        }

        fn save(&self, data: &OwnerData) -> Result<(), StoreError> {
            if self.fail_next.replace(false) {
                Err(StoreError::Backend("test".to_owned()))
            } else {
                self.inner.save(data)
            }
        }
    }

    #[test]
    fn failed_deed_save_retry_keeps_another_tabs_deed_and_mark() -> TestResult {
        let store = FailOnceStore {
            inner: MemoryStore::default(),
            fail_next: Cell::new(false),
        };
        let mut first = started(&store)?;
        let mut second = started(&store)?;
        let local = first.save_deed(
            None,
            &EditorDraft {
                name: "Local".to_owned(),
                ..EditorDraft::default()
            },
        )?;
        first.tap(local, Tap::Missed);
        store.fail_next.set(true);
        first.save_to(&store);
        assert_eq!(first.save_status, SaveStatus::SaveFailed);
        let other = second.save_deed(
            None,
            &EditorDraft {
                name: "Other".to_owned(),
                ..EditorDraft::default()
            },
        )?;
        second.tap(other, Tap::Done);
        second.save_to(&store);
        first.save_to(&store);
        let saved = started(&store)?;
        assert_eq!(
            saved.data.names.get(&other),
            Some(&werdy_store::DeedName::Text("Other".to_owned()))
        );
        assert_eq!(saved.mark(other), Some(Mark::Done));
        let local_id = saved
            .data
            .names
            .iter()
            .find_map(|(id, name)| {
                (name == &werdy_store::DeedName::Text("Local".to_owned())).then_some(*id)
            })
            .ok_or("missing local deed")?;
        assert_ne!(local_id, other);
        assert_eq!(saved.mark(local_id), Some(Mark::Missed));
        Ok(())
    }

    #[test]
    fn local_edit_does_not_undo_another_tabs_delete() -> TestResult {
        let store = MemoryStore::default();
        let mut deleting = started(&store)?;
        let mut editing = started(&store)?;
        deleting.delete_deed_confirmed(DeedId(5), true)?;
        deleting.save_to(&store);
        let mut draft = editing.draft_for(DeedId(1)).ok_or("missing draft")?;
        draft.name = "Changed".to_owned();
        editing.save_deed(Some(DeedId(1)), &draft)?;
        editing.save_to(&store);
        assert!(
            !started(&store)?
                .data
                .deeds
                .iter()
                .any(|deed| deed.id == DeedId(5))
        );
        Ok(())
    }

    #[test]
    fn deleting_unsaved_deed_does_not_delete_another_tabs_new_deed() -> TestResult {
        let store = MemoryStore::default();
        let mut first = started(&store)?;
        let mut second = started(&store)?;
        let unused = first.save_deed(
            None,
            &EditorDraft {
                name: "Unused".to_owned(),
                ..EditorDraft::default()
            },
        )?;
        first.delete_deed_confirmed(unused, true)?;
        let other = second.save_deed(
            None,
            &EditorDraft {
                name: "Other".to_owned(),
                ..EditorDraft::default()
            },
        )?;
        second.save_to(&store);
        first.save_to(&store);
        assert_eq!(
            started(&store)?.data.names.get(&other),
            Some(&werdy_store::DeedName::Text("Other".to_owned()))
        );
        Ok(())
    }

    #[test]
    fn deleting_highest_id_does_not_attach_stale_mark_to_new_deed() -> TestResult {
        let store = MemoryStore::default();
        let mut deleting = started(&store)?;
        let mut stale = started(&store)?;
        deleting.delete_deed_confirmed(DeedId(5), true)?;
        deleting.save_to(&store);
        let new_id = deleting.save_deed(
            None,
            &EditorDraft {
                name: "New".to_owned(),
                ..EditorDraft::default()
            },
        )?;
        deleting.save_to(&store);
        stale.tap(DeedId(5), Tap::Done);
        stale.save_to(&store);
        let saved = started(&store)?;
        assert_ne!(new_id, DeedId(5));
        assert!(!saved.data.marks.keys().any(|(id, _)| *id == new_id));
        Ok(())
    }

    #[test]
    fn unchanged_starter_name_stays_a_key() -> TestResult {
        let store = MemoryStore::default();
        let mut state = started(&store)?;
        let mut draft = state.draft_for(DeedId(1)).ok_or("missing draft")?;
        draft.name = "Translated name".to_owned();
        draft.original_name = Some(draft.name.clone());
        state.save_deed(Some(DeedId(1)), &draft)?;
        state.save_to(&store);
        assert!(matches!(
            started(&store)?.deed_name(DeedId(1)),
            Some(werdy_store::DeedName::Key(_))
        ));
        Ok(())
    }

    #[test]
    fn target_edit_converts_todays_mark_and_keeps_past_mark() -> TestResult {
        let store = MemoryStore::default();
        let mut state = started(&store)?;
        let id = DeedId(1);
        state.tap(id, Tap::Done);
        state.save_to(&store);
        let past = today()?;
        state.update_today(past.next()?);
        state.tap(id, Tap::Done);
        let mut draft = state.draft_for(id).ok_or("missing draft")?;
        draft.target_enabled = true;
        draft.target_amount = 5;
        state.save_deed(Some(id), &draft)?;
        assert_eq!(state.mark(id), Some(Mark::Count(5)));
        assert_eq!(state.data.marks[&(id, past)].mark, Mark::Done);
        state.save_to(&store);
        assert_eq!(
            started(&store)?.data.marks[&(id, past.next()?)].mark,
            Mark::Count(5)
        );
        Ok(())
    }

    #[test]
    fn editor_validation_keeps_bad_changes_out_of_state() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        let original = state.data.clone();
        assert_eq!(
            state.save_deed(None, &EditorDraft::default()),
            Err(EditorError::EmptyName)
        );
        let mut draft = EditorDraft {
            name: "Reading".to_owned(),
            ..EditorDraft::default()
        };
        draft.weekdays();
        assert_eq!(state.save_deed(None, &draft), Err(EditorError::NoWeekday));
        assert_eq!(state.data, original);
        Ok(())
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

    #[test]
    fn marking_yesterday_changes_its_score() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        state.update_today(today()?.next()?);
        state.show_previous_day();
        assert_eq!(
            state.viewing_score()?.and_then(|score| score.percent),
            Some(0)
        );
        state.tap(DeedId(1), Tap::Done);
        assert_eq!(
            state.viewing_score()?.and_then(|score| score.percent),
            Some(20)
        );
        Ok(())
    }

    #[test]
    fn half_credit_changes_yesterdays_score_and_keeps_entry_day() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        state.update_today(today()?.next()?);
        state.show_previous_day();
        let yesterday = state.viewing;
        state.save_day_note(yesterday, "Ill", ExcuseCredit::Half)?;
        assert_eq!(
            state.viewing_score()?.and_then(|score| score.percent),
            Some(50)
        );
        assert_eq!(
            state.viewing_note().map(|note| note.entered_on()),
            Some(state.today)
        );
        assert!(state.data.marks.is_empty());
        assert!(state.data.day_notes.contains_key(&yesterday));
        Ok(())
    }

    #[test]
    fn empty_note_without_credit_leaves_day_unchanged() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        assert_eq!(
            state.save_day_note(state.viewing, "", ExcuseCredit::None),
            Err(Error::EmptyDayNote)
        );
        assert!(state.viewing_note().is_none());
        Ok(())
    }

    #[test]
    fn note_dialog_day_survives_today_rolling_over() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        let opened_day = state.viewing;
        let entry_day = opened_day.next()?;
        state.update_today(entry_day);
        state.save_day_note(opened_day, "Ill", ExcuseCredit::None)?;
        assert_eq!(
            state
                .data
                .day_notes
                .get(&opened_day)
                .map(|note| note.entered_on()),
            Some(entry_day)
        );
        assert!(!state.data.day_notes.contains_key(&entry_day));
        Ok(())
    }

    #[test]
    fn stale_tab_keeps_note_edit_and_deletion() -> TestResult {
        let store = MemoryStore::default();
        let mut first = started(&store)?;
        let mut stale = started(&store)?;
        first.show_previous_day();
        let day = first.viewing;
        first.save_day_note(day, "Ill", ExcuseCredit::Half)?;
        first.save_to(&store);
        stale.tap(DeedId(1), Tap::Done);
        stale.save_to(&store);
        assert_eq!(
            stale.data.day_notes.get(&day).map(|note| note.credit()),
            Some(ExcuseCredit::Half)
        );
        first.delete_day_note(day);
        first.save_to(&store);
        stale.save_to(&store);
        assert!(!started(&store)?.data.day_notes.contains_key(&day));
        Ok(())
    }

    #[test]
    fn moving_today_follows_only_when_viewing_old_today() -> TestResult {
        let old = today()?;
        let new = old.next()?;
        assert_eq!(super::viewing_after_today_change(old, new, old), new);
        assert_eq!(
            super::viewing_after_today_change(old, new, old.previous()?),
            old.previous()?
        );
        assert_eq!(super::viewing_after_today_change(new, old, new), old);
        Ok(())
    }

    #[test]
    fn current_slot_uses_local_clock_boundaries() {
        assert_eq!(super::current_slot(3, 59), "night");
        assert_eq!(super::current_slot(4, 0), "fajr");
        assert_eq!(super::current_slot(6, 0), "morning");
        assert_eq!(super::current_slot(12, 0), "dhuhr");
        assert_eq!(super::current_slot(15, 30), "asr");
        assert_eq!(super::current_slot(18, 0), "maghrib");
        assert_eq!(super::current_slot(19, 30), "isha");
        assert_eq!(super::current_slot(22, 0), "night");
    }

    #[test]
    fn period_arrows_step_whole_windows_and_stop_at_latest() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        state.show_screen(Screen::Reports);
        assert_eq!(state.screen, Screen::Reports);
        let latest = state.report()?.days;
        state.show_next_period();
        assert_eq!(state.report()?.days, latest);
        state.show_previous_period();
        assert_eq!(
            state.report()?.days.last(),
            latest.first().and_then(|day| day.previous().ok()).as_ref()
        );
        state.show_next_period();
        assert_eq!(state.report()?.days, latest);
        state.select_report_period(Period::Month);
        assert_eq!(state.report_steps_back, 0);
        assert_eq!(state.report()?.days.len(), 30);
        Ok(())
    }

    #[test]
    fn screens_share_navigation_without_losing_report_period() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        assert_eq!(state.screen, Screen::Sheet);
        state.show_screen(Screen::Reports);
        state.select_report_period(Period::Month);
        state.show_screen(Screen::Deeds);
        assert_eq!(state.screen, Screen::Deeds);
        state.show_screen(Screen::Editor(None));
        assert_eq!(state.screen, Screen::Editor(None));
        state.show_screen(Screen::Reports);
        assert_eq!(state.report_period, Period::Month);
        Ok(())
    }

    #[test]
    fn empty_report_has_no_percent_or_missed_deeds() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        state.data.deeds.clear();
        let report = state.report()?;
        assert_eq!(report.overall.due, 0);
        assert_eq!(report.overall.percent, None);
        assert!(report.missed.is_empty());
        Ok(())
    }

    #[test]
    fn date_change_moves_sheet_and_refreshes_counts() -> TestResult {
        let store = MemoryStore::default();
        let mut state = started(&store)?;
        state.tap(DeedId(1), Tap::Done);
        assert_eq!(state.summary().done, 1);

        let new_today = today()?.next()?;
        state.update_today(new_today);
        assert_eq!(state.viewing, new_today);
        assert_eq!(state.summary().done, 0);
        assert_eq!(state.summary().open, 5);
        Ok(())
    }

    #[test]
    fn unmarked_due_deeds_on_a_past_day_show_as_missed() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        state.update_today(today()?.next()?);
        state.show_previous_day();

        assert_eq!(state.summary().open, 0);
        assert_eq!(state.summary().missed, 5);
        Ok(())
    }

    #[test]
    fn a_mark_refreshes_today_but_keeps_a_chosen_past_day() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        let old_today = state.today;
        let new_today = old_today.next()?;

        assert!(state.change_on_day(Ok(new_today), |state| state.tap(DeedId(1), Tap::Done)));
        assert_eq!(state.viewing, new_today);
        assert_eq!(
            state.data.marks[&(DeedId(1), new_today)].entered_on,
            new_today
        );
        assert!(!state.data.marks.contains_key(&(DeedId(1), old_today)));

        state.show_previous_day();
        assert!(state.change_on_day(Ok(new_today.next()?), |state| {
            state.tap(DeedId(2), Tap::Done)
        }));
        assert_eq!(state.viewing, old_today);
        assert_eq!(
            state.data.marks[&(DeedId(2), old_today)].entered_on,
            new_today.next()?
        );
        Ok(())
    }

    #[test]
    fn a_mark_is_skipped_when_today_cannot_be_read() -> TestResult {
        let mut state = started(&MemoryStore::default())?;

        assert!(!state.change_on_day(Err(Error::InvalidDate), |state| {
            state.tap(DeedId(1), Tap::Done);
        }));

        assert_eq!(state.save_status, SaveStatus::DateFailed);
        assert!(state.data.marks.is_empty());
        Ok(())
    }

    #[test]
    fn stale_tab_save_keeps_other_tabs_mark_and_clear() -> TestResult {
        let store = MemoryStore::default();
        let mut first = started(&store)?;
        let mut stale = started(&store)?;
        first.tap(DeedId(1), Tap::Done);
        first.save_to(&store);
        stale.tap(DeedId(2), Tap::Missed);
        stale.save_to(&store);
        assert_eq!(stale.mark(DeedId(1)), Some(Mark::Done));
        assert_eq!(stale.mark(DeedId(2)), Some(Mark::Missed));

        first.reload_from(&store);
        assert_eq!(first.mark(DeedId(2)), Some(Mark::Missed));
        first.tap(DeedId(1), Tap::Done);
        first.save_to(&store);
        stale.save_to(&store);
        let saved = started(&store)?;
        assert_eq!(saved.mark(DeedId(1)), None);
        assert_eq!(saved.mark(DeedId(2)), Some(Mark::Missed));
        Ok(())
    }

    #[test]
    fn stale_mark_save_keeps_language_change_from_other_tab() -> TestResult {
        let store = MemoryStore::default();
        let mut first = started(&store)?;
        let mut stale = started(&store)?;
        first.set_language("en");
        first.save_to(&store);

        stale.tap(DeedId(1), Tap::Done);
        stale.save_to(&store);
        assert_eq!(stale.data.settings.language, "en");

        stale.set_language("ar");
        stale.save_to(&store);
        assert_eq!(started(&store)?.data.settings.language, "ar");
        Ok(())
    }
}
