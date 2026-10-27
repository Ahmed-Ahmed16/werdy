//! App state: the owner's data, the day on screen, and what each tap does.
//! It calls werdy-core for every number and holds no scoring of its own.

use std::collections::{BTreeMap, BTreeSet};

use werdy_core::{
    CategoryKey, DailySheet, DayNote, Deed, DeedId, DeedRule, Error, ExcuseCredit, Mark, MarkEntry,
    Period, Report, Schedule, Score, ScoreBands, SlotKey, Target, TodaySummary, Weekday, WerdyDay,
    build_report, convert_mark_for_target, daily_sheet, delete_deed, finished_day_summary, score,
    today_summary,
};
use werdy_store::{DeedName, Location, OwnerData, PrayerMethod, Store, merge_records};

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
    FirstRunLocation,
    SetPicker,
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
    sets_dirty: bool,
    deed_changes: BTreeMap<DeedId, DeedChange>,
}

impl AppState {
    /// Loads saved data. Empty storage opens setup. Unreadable data is never overwritten.
    pub fn start(store: &dyn Store, today: WerdyDay, slot_order: Vec<SlotKey>) -> Self {
        let (mut data, save_status) = match store.load() {
            Ok(Some(data)) => (data, SaveStatus::Saved),
            Ok(None) => (OwnerData::default(), SaveStatus::Saved),
            Err(_) => (OwnerData::default(), SaveStatus::LoadFailed),
        };
        let migrated = crate::starter::migrate_legacy_prayers(&mut data);
        let screen = if data.first_run_complete {
            Screen::Sheet
        } else {
            Screen::FirstRunLocation
        };
        Self {
            data,
            today,
            viewing: today,
            slot_order,
            save_status,
            screen,
            report_period: Period::Week,
            report_steps_back: 0,
            settings_dirty: false,
            sets_dirty: migrated,
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

    /// Saves a changed note with the current day as its entry day.
    pub fn save_day_note(
        &mut self,
        day: WerdyDay,
        text: &str,
        credit: ExcuseCredit,
    ) -> Result<(), Error> {
        if self
            .data
            .day_notes
            .get(&day)
            .is_some_and(|note| note.text() == text && note.credit() == credit)
        {
            return Ok(());
        }
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

    pub fn set_location(&mut self, location: Location) -> Result<(), EditorError> {
        let method = crate::location::method_for_country(location.country.as_deref().unwrap_or(""))
            .map_err(|_| EditorError::RuleChange)?;
        self.data.settings.prayer_method = method;
        self.data.settings.location = Some(location);
        self.settings_dirty = true;
        Ok(())
    }

    pub fn set_prayer_method(&mut self, method: PrayerMethod) {
        self.data.settings.prayer_method = method;
        self.settings_dirty = true;
    }

    pub fn clear_location(&mut self) {
        self.data.settings.location = None;
        self.settings_dirty = true;
    }

    pub fn finish_first_run(&mut self, screen: Screen) {
        if !self.data.first_run_complete {
            self.redate_setup_deeds();
        }
        self.data.first_run_complete = true;
        self.settings_dirty = true;
        self.screen = screen;
    }

    pub fn toggle_set(&mut self, key: &str) -> Result<(), EditorError> {
        let previous = self.clone();
        let result = self.toggle_set_inner(key);
        if result.is_err() {
            *self = previous;
        }
        result
    }

    fn toggle_set_inner(&mut self, key: &str) -> Result<(), EditorError> {
        let catalog = crate::editor_data::catalog().map_err(|_| EditorError::RuleChange)?;
        if catalog.set(key).is_none() {
            return Err(EditorError::RuleChange);
        }
        let mut selected = self.data.chosen_sets.clone();
        if !selected.insert(key.to_owned()) {
            selected.remove(key);
        }
        let wanted = crate::starter::selected_deeds(catalog, &selected);
        self.add_selected_deeds(catalog, &selected)?;
        self.restore_selected_deeds(&wanted)?;
        self.remove_unselected_deeds(&wanted)?;
        self.data.chosen_sets = selected;
        self.sets_dirty = true;
        Ok(())
    }

    fn add_selected_deeds(
        &mut self,
        catalog: &werdy_data::Catalog,
        selected: &BTreeSet<String>,
    ) -> Result<(), EditorError> {
        let missing = crate::starter::missing_deeds(catalog, &self.data, selected);
        for key in missing {
            let deed = catalog.deed(&key).ok_or(EditorError::RuleChange)?;
            let rule = crate::starter::rule(deed).ok_or(EditorError::RuleChange)?;
            let id = self.data.next_deed_id();
            if id.0 <= self.data.last_deed_id {
                return Err(EditorError::RuleChange);
            }
            self.data.last_deed_id = id.0;
            let created_deed = Deed::new(id, self.today, rule);
            self.data.deeds.push(created_deed.clone());
            self.data.names.insert(id, DeedName::Key(deed.name.clone()));
            self.data.starter_keys.insert(id, key);
            self.deed_changes.insert(
                id,
                DeedChange::Upsert {
                    deed: Box::new(created_deed),
                    name: DeedName::Key(deed.name.clone()),
                    created: true,
                },
            );
        }
        Ok(())
    }

    fn restore_selected_deeds(&mut self, wanted: &BTreeSet<String>) -> Result<(), EditorError> {
        let restore: Vec<_> = self
            .data
            .starter_keys
            .iter()
            .filter(|(_, key)| wanted.contains(*key))
            .filter_map(|(id, _)| {
                self.data
                    .deeds
                    .iter()
                    .find(|deed| deed.id == *id && deed.is_archived_on(self.today))
                    .map(|_| *id)
            })
            .collect();
        for id in restore {
            self.restore_deed(id)?;
        }
        Ok(())
    }

    fn remove_unselected_deeds(&mut self, wanted: &BTreeSet<String>) -> Result<(), EditorError> {
        let remove: Vec<_> = self
            .data
            .starter_keys
            .iter()
            .filter(|(_, key)| !wanted.contains(*key))
            .filter_map(|(id, _)| {
                self.data
                    .deeds
                    .iter()
                    .find(|deed| deed.id == *id && !deed.is_archived_on(self.today))
                    .map(|_| *id)
            })
            .collect();
        for id in remove {
            let has_past_due = self
                .data
                .deeds
                .iter()
                .find(|deed| deed.id == id)
                .is_some_and(|deed| {
                    let mut day = deed.created_on();
                    while day < self.today {
                        if deed.is_due_on(day, &self.data.settings.calendar) {
                            return true;
                        }
                        let Ok(next) = day.next() else { break };
                        day = next;
                    }
                    false
                });
            if has_past_due || self.data.marks.keys().any(|(deed_id, _)| *deed_id == id) {
                self.archive_deed(id)?;
            } else {
                self.delete_deed_confirmed(id, true)?;
                self.data.starter_keys.remove(&id);
            }
        }
        Ok(())
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
        self.data.starter_keys.remove(&id);
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
        let moved = today != self.today;
        self.viewing = viewing_after_today_change(self.today, today, self.viewing);
        self.today = today;
        if moved && !self.data.first_run_complete {
            self.redate_setup_deeds();
        }
    }

    fn redate_setup_deeds(&mut self) {
        let ids: Vec<_> = self
            .data
            .deeds
            .iter()
            .filter(|deed| deed.archived_from().is_none())
            .filter(|deed| !self.data.marks.keys().any(|(id, _)| *id == deed.id))
            .map(|deed| deed.id)
            .collect();
        for id in ids {
            if let Some(deed) = self.data.deeds.iter_mut().find(|deed| deed.id == id)
                && let Some(rule) = deed.rule_on(deed.created_on()).cloned()
            {
                *deed = Deed::new(id, self.today, rule);
                self.record_upsert(id);
            }
        }
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
        if let Some(key) = self.data.starter_keys.remove(&old_id) {
            self.data.starter_keys.insert(new_id, key);
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
                    if let Some(key) = self.data.starter_keys.get(id) {
                        merged.starter_keys.insert(*id, key.clone());
                    }
                }
                DeedChange::Delete => {
                    merged.deleted_deeds.insert(*id);
                    delete_deed(&mut merged.deeds, &mut merged.marks, *id);
                    merged.names.remove(id);
                    merged.starter_keys.remove(id);
                    merged
                        .mark_revisions
                        .retain(|(deed_id, _), _| deed_id != id);
                    merged.cleared_marks.retain(|(deed_id, _), _| deed_id != id);
                }
                _ => {}
            }
        }
        for (id, key) in &self.data.starter_keys {
            merged
                .starter_keys
                .entry(*id)
                .or_insert_with(|| key.clone());
        }
        merged.last_deed_id = merged.last_deed_id.max(self.data.last_deed_id);
        let live_ids: BTreeSet<_> = merged.deeds.iter().map(|deed| deed.id).collect();
        merged.starter_keys.retain(|id, _| live_ids.contains(id));
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
                merged.first_run_complete |= self.data.first_run_complete;
                if self.sets_dirty {
                    merged.chosen_sets = self.data.chosen_sets.clone();
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
                        self.sets_dirty = false;
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
                merged.first_run_complete |= self.data.first_run_complete;
                if self.sets_dirty {
                    merged.chosen_sets = self.data.chosen_sets.clone();
                }
                self.apply_deed_changes(&mut merged);
                if merged == stored {
                    self.data = stored;
                    self.save_status = SaveStatus::Saved;
                    self.deed_changes.clear();
                    self.settings_dirty = false;
                    self.sets_dirty = false;
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
    use werdy_store::{Location, MemoryStore, OwnerData, PrayerMethod, Store, StoreError};

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
        if store.load().ok().flatten().is_none()
            && let Ok(data) = five_prayers(today()?)
        {
            let _ = store.save(&data);
        }
        Ok(AppState::start(store, today()?, slot_order()))
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
    fn first_run_opens_setup_without_adding_deeds() -> TestResult {
        let store = MemoryStore::default();
        let state = AppState::start(&store, today()?, slot_order());
        assert_eq!(state.screen, Screen::FirstRunLocation);
        assert!(state.data.deeds.is_empty());
        assert_eq!(state.save_status, SaveStatus::Saved);
        assert_eq!(store.load()?, None);
        Ok(())
    }

    #[test]
    fn first_run_location_and_set_choice_survive_restart() -> TestResult {
        let store = MemoryStore::default();
        let mut state = AppState::start(&store, today()?, slot_order());
        let place = Location::new(
            Some("cairo".to_owned()),
            Some("EG".to_owned()),
            30.04,
            31.24,
        )
        .ok_or("bad test place")?;
        state.set_location(place)?;
        assert_eq!(
            state.data.settings.prayer_method,
            PrayerMethod::EgyptianGeneralAuthority
        );
        state.show_screen(Screen::SetPicker);
        state.toggle_set("five_prayers")?;
        state.save_to(&store);
        state.finish_first_run(Screen::Sheet);
        state.save_to(&store);

        let again = AppState::start(&store, today()?, slot_order());
        assert_eq!(again.screen, Screen::Sheet);
        assert_eq!(again.data.deeds.len(), 5);
        assert!(again.data.chosen_sets.contains("five_prayers"));
        assert_eq!(again.data.settings.location, state.data.settings.location);
        Ok(())
    }

    #[test]
    fn removing_set_keeps_unmarked_past_due_prayers_in_score() -> TestResult {
        let yesterday = today()?;
        let store = MemoryStore::default();
        let mut state = AppState::start(&store, yesterday, slot_order());
        state.toggle_set("five_prayers")?;
        state.finish_first_run(Screen::Sheet);
        let id = *state.data.starter_keys.keys().next().ok_or("no prayer")?;
        state.tap(id, Tap::Done);
        state.save_to(&store);
        state.update_today(yesterday.next()?);
        state.show_previous_day();
        let before = state.summary();
        assert_eq!(before.done, 1);
        assert_eq!(before.missed, 4);
        state.toggle_set("five_prayers")?;
        state.save_to(&store);
        assert_eq!(state.summary(), before);
        assert_eq!(state.data.deeds.len(), 5);
        let mut loaded = AppState::start(&store, state.today, slot_order());
        loaded.show_previous_day();
        assert_eq!(loaded.summary(), before);
        Ok(())
    }

    #[test]
    fn stale_set_add_rebases_after_custom_deed() -> TestResult {
        let store = MemoryStore::default();
        let mut custom = AppState::start(&store, today()?, slot_order());
        let mut stale = AppState::start(&store, today()?, slot_order());
        let custom_id = custom.save_deed(
            None,
            &EditorDraft {
                name: "Reading".to_owned(),
                ..EditorDraft::default()
            },
        )?;
        custom.tap(custom_id, Tap::Done);
        custom.save_to(&store);
        stale.toggle_set("five_prayers")?;
        let prayer_id = *stale.data.starter_keys.keys().next().ok_or("no prayer")?;
        stale.tap(prayer_id, Tap::Missed);
        stale.save_to(&store);
        let loaded = AppState::start(&store, today()?, slot_order());
        assert_eq!(loaded.data.deeds.len(), 6);
        assert_eq!(loaded.data.starter_keys.len(), 5);
        assert!(!loaded.data.starter_keys.contains_key(&custom_id));
        assert_eq!(loaded.data.marks[&(custom_id, today()?)].mark, Mark::Done);
        let rebased_prayer = loaded
            .data
            .starter_keys
            .iter()
            .find_map(|(id, key)| (key == "fajr_on_time").then_some(*id))
            .ok_or("missing Fajr")?;
        assert_ne!(rebased_prayer, custom_id);
        assert_eq!(
            loaded.data.marks[&(rebased_prayer, today()?)].mark,
            Mark::Missed
        );
        Ok(())
    }

    #[test]
    fn stale_language_save_does_not_restore_deleted_starter_key() -> TestResult {
        let store = MemoryStore::default();
        let mut initial = AppState::start(&store, today()?, slot_order());
        initial.toggle_set("five_prayers")?;
        initial.save_to(&store);
        let id = *initial.data.starter_keys.keys().next().ok_or("no prayer")?;
        let mut deleting = AppState::start(&store, today()?, slot_order());
        let mut stale = AppState::start(&store, today()?, slot_order());
        deleting.delete_deed_confirmed(id, true)?;
        deleting.save_to(&store);
        stale.set_language("en");
        stale.save_to(&store);
        let saved = store.load()?.ok_or("no saved data")?;
        let loaded = werdy_store::from_json(&werdy_store::to_json(&saved)?)?;
        assert!(!loaded.starter_keys.contains_key(&id));
        assert!(!loaded.deeds.iter().any(|deed| deed.id == id));
        assert_eq!(loaded.settings.language, "en");
        Ok(())
    }

    #[test]
    fn stale_language_reload_does_not_restore_deleted_starter_key() -> TestResult {
        let store = MemoryStore::default();
        let mut initial = AppState::start(&store, today()?, slot_order());
        initial.toggle_set("five_prayers")?;
        initial.save_to(&store);
        let id = *initial.data.starter_keys.keys().next().ok_or("no prayer")?;
        let mut deleting = AppState::start(&store, today()?, slot_order());
        let mut stale = AppState::start(&store, today()?, slot_order());
        deleting.delete_deed_confirmed(id, true)?;
        deleting.save_to(&store);
        stale.set_language("en");
        stale.reload_from(&store);
        let saved = store.load()?.ok_or("no saved data")?;
        let loaded = werdy_store::from_json(&werdy_store::to_json(&saved)?)?;
        assert!(!loaded.starter_keys.contains_key(&id));
        assert!(!loaded.deeds.iter().any(|deed| deed.id == id));
        assert_eq!(loaded.settings.language, "en");
        Ok(())
    }

    #[test]
    fn finishing_setup_dates_added_deeds_on_finish_day() -> TestResult {
        let start = today()?;
        let mut state = AppState::start(&MemoryStore::default(), start, slot_order());
        state.toggle_set("five_prayers")?;
        let finish = start.next()?;
        state.update_today(finish);
        state.finish_first_run(Screen::Sheet);
        assert_eq!(state.summary().open, 5);
        assert!(
            state
                .data
                .deeds
                .iter()
                .all(|deed| deed.created_on() == finish)
        );
        Ok(())
    }

    #[test]
    fn setup_day_moving_back_keeps_new_deeds_due() -> TestResult {
        let start = today()?;
        let mut state = AppState::start(&MemoryStore::default(), start, slot_order());
        state.toggle_set("five_prayers")?;
        let earlier = start.previous()?;
        state.update_today(earlier);
        assert_eq!(state.summary().open, 5);
        assert!(
            state
                .data
                .deeds
                .iter()
                .all(|deed| deed.created_on() == earlier)
        );
        Ok(())
    }

    #[test]
    fn finishing_setup_does_not_restore_archived_deed() -> TestResult {
        let start = today()?;
        let mut state = AppState::start(&MemoryStore::default(), start, slot_order());
        state.toggle_set("five_prayers")?;
        let id = *state.data.starter_keys.keys().next().ok_or("no prayer")?;
        state.archive_deed(id)?;
        state.finish_first_run(Screen::Sheet);
        assert_eq!(state.summary().open, 4);
        assert!(
            state
                .data
                .deeds
                .iter()
                .find(|deed| deed.id == id)
                .is_some_and(|deed| deed.is_archived_on(state.today))
        );
        Ok(())
    }

    #[test]
    fn older_prayer_sheet_migrates_without_duplicate_deeds() -> TestResult {
        let day = today()?;
        let mut old = five_prayers(day)?;
        old.starter_keys.clear();
        old.chosen_sets.clear();
        for name in old.names.values_mut() {
            if let werdy_store::DeedName::Key(key) = name {
                *key = key.replacen("deed-", "starter-", 1);
            }
        }
        let id = old.deeds[0].id;
        old.set_mark(
            id,
            day,
            Some(werdy_core::MarkEntry::new(Mark::Done, day)),
            1,
        );
        let later = day.next()?;
        let mut changed_rule = old.deeds[0].rule_on(day).ok_or("no rule")?.clone();
        changed_rule.slot = werdy_core::SlotKey::new("morning");
        old.deeds[0].change_rule(later, changed_rule)?;
        let mut json: serde_json::Value = serde_json::from_str(&werdy_store::to_json(&old)?)?;
        json["schema_version"] = 6.into();
        json.as_object_mut()
            .ok_or("invalid file")?
            .remove("starter_keys");
        json.as_object_mut()
            .ok_or("invalid file")?
            .remove("chosen_sets");
        let store = MemoryStore::default();
        store.save(&werdy_store::from_json(&json.to_string())?)?;
        let mut state = AppState::start(&store, later, slot_order());
        assert!(state.data.chosen_sets.contains("five_prayers"));
        assert_eq!(state.data.starter_keys.len(), 5);
        state.toggle_set("five_prayers")?;
        state.save_to(&store);
        let mut state = AppState::start(&store, later, slot_order());
        assert!(!state.data.chosen_sets.contains("five_prayers"));
        state.toggle_set("five_prayers")?;
        assert_eq!(state.data.deeds.len(), 5);
        assert_eq!(state.data.marks[&(id, day)].mark, Mark::Done);
        assert_eq!(
            state.data.deeds[0].rule_on(later),
            old.deeds[0].rule_on(later)
        );
        Ok(())
    }

    #[test]
    fn skipping_location_clears_saved_place() -> TestResult {
        let store = MemoryStore::default();
        let mut state = AppState::start(&store, today()?, slot_order());
        state.set_location(
            Location::new(None, Some("EG".to_owned()), 30.04, 31.24).ok_or("place")?,
        )?;
        state.clear_location();
        state.save_to(&store);
        assert!(store.load()?.ok_or("no data")?.settings.location.is_none());
        Ok(())
    }

    #[test]
    fn removing_one_overlapping_set_keeps_shared_deeds() -> TestResult {
        let catalog = werdy_data::builtin()?;
        let first = catalog
            .sets
            .iter()
            .find(|a| {
                catalog
                    .sets
                    .iter()
                    .any(|b| a.key != b.key && a.deeds.iter().any(|key| b.deeds.contains(key)))
            })
            .ok_or("no overlap")?;
        let second = catalog
            .sets
            .iter()
            .find(|b| first.key != b.key && first.deeds.iter().any(|key| b.deeds.contains(key)))
            .ok_or("no second set")?;
        let mut state = AppState::start(&MemoryStore::default(), today()?, slot_order());
        state.toggle_set(&first.key)?;
        state.toggle_set(&second.key)?;
        let shared = first
            .deeds
            .iter()
            .find(|key| second.deeds.contains(key))
            .ok_or("no shared deed")?;
        assert_eq!(
            state
                .data
                .starter_keys
                .values()
                .filter(|key| *key == shared)
                .count(),
            1
        );
        state.toggle_set(&first.key)?;
        assert_eq!(
            state
                .data
                .starter_keys
                .values()
                .filter(|key| *key == shared)
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn removing_a_marked_set_preserves_history_and_readding_restores_deeds() -> TestResult {
        let mut state = AppState::start(&MemoryStore::default(), today()?, slot_order());
        state.toggle_set("five_prayers")?;
        let id = *state
            .data
            .starter_keys
            .iter()
            .find(|(_, key)| *key == "fajr_on_time")
            .map(|(id, _)| id)
            .ok_or("missing Fajr")?;
        state.tap(id, Tap::Done);
        state.toggle_set("five_prayers")?;
        assert!(
            state
                .data
                .deeds
                .iter()
                .find(|deed| deed.id == id)
                .is_some_and(|deed| deed.is_archived_on(state.today))
        );
        state.toggle_set("five_prayers")?;
        assert_eq!(state.data.deeds.len(), 5);
        assert_eq!(state.mark(id), Some(Mark::Done));
        assert!(
            state
                .data
                .deeds
                .iter()
                .find(|deed| deed.id == id)
                .is_some_and(|deed| !deed.is_archived_on(state.today))
        );
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
        let mut state = AppState::start(&BrokenStore, today()?, slot_order());

        state.finish_first_run(Screen::Sheet);
        state.save_to(&store);

        assert_eq!(state.save_status, SaveStatus::LoadFailed);
        assert!(state.data.deeds.is_empty());
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
    fn unchanged_note_keeps_original_entry_day_and_is_not_late() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        let day = state.viewing;
        state.save_day_note(day, "Ill", ExcuseCredit::Half)?;
        state.update_today(today()?.next()?.next()?.next()?);
        state.save_day_note(day, "Ill", ExcuseCredit::Half)?;
        let note = state.data.day_notes.get(&day).ok_or("missing note")?;
        assert_eq!(note.entered_on(), day);
        assert!(!note.is_late_for(day));
        Ok(())
    }

    #[test]
    fn edited_note_records_edit_day() -> TestResult {
        let mut state = started(&MemoryStore::default())?;
        let day = state.viewing;
        state.save_day_note(day, "Ill", ExcuseCredit::Half)?;
        state.update_today(today()?.next()?.next()?.next()?);
        state.save_day_note(day, "Ill", ExcuseCredit::Full)?;
        let note = state.data.day_notes.get(&day).ok_or("missing note")?;
        assert_eq!(note.entered_on(), state.today);
        assert!(note.is_late_for(day));
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
