//! Local store for the owner's records (spec module 3, seam 5).
//!
//! The records are turned into JSON here. Where the JSON lives is behind the
//! [`Store`] trait, so the browser's localStorage, a file or SQLite can hold it
//! without any change to the rest of the app.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use werdy_core::{
    CalendarSettings, DayNote, Deed, DeedId, HijriOffset, MarkEntry, OwnerRecords, WerdyDay,
};

/// Version of the JSON layout written by this code.
pub const SCHEMA_VERSION: u32 = 8;

/// Device only coordinates, stored as hundredths of a degree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Location {
    /// Bundled city key, or none for a browser location.
    pub city_key: Option<String>,
    /// Two letter country code, when known.
    pub country: Option<String>,
    pub latitude_hundredths: i32,
    pub longitude_hundredths: i32,
}

impl Location {
    pub fn new(
        city_key: Option<String>,
        country: Option<String>,
        latitude: f64,
        longitude: f64,
    ) -> Option<Self> {
        if !latitude.is_finite()
            || !longitude.is_finite()
            || !(-90.0..=90.0).contains(&latitude)
            || !(-180.0..=180.0).contains(&longitude)
        {
            return None;
        }
        Some(Self {
            city_key,
            country,
            latitude_hundredths: (latitude * 100.0).round() as i32,
            longitude_hundredths: (longitude * 100.0).round() as i32,
        })
    }

    pub fn coordinates(&self) -> (f64, f64) {
        (
            f64::from(self.latitude_hundredths) / 100.0,
            f64::from(self.longitude_hundredths) / 100.0,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrayerMethod {
    EgyptianGeneralAuthority,
    #[default]
    MuslimWorldLeague,
}

/// Error from loading or saving records. Bad data never panics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    /// The text is not valid JSON or holds an invalid value.
    Corrupt(String),
    /// The data was written by a newer werdy with a layout we do not know.
    UnsupportedVersion(u32),
    /// Two deeds with one id, two marks for one day, or two notes for one day.
    DuplicateEntry,
    /// The place that holds the JSON failed, such as a full or blocked browser storage.
    Backend(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corrupt(reason) => write!(formatter, "stored data is corrupt: {reason}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "stored data uses unknown version {version}")
            }
            Self::DuplicateEntry => formatter.write_str("stored data has a duplicate entry"),
            Self::Backend(reason) => write!(formatter, "storage failed: {reason}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// The name shown for a deed: a translation key for starter deeds, or the
/// owner's own text.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeedName {
    /// Key of a Fluent message, such as `starter-fajr-on-time`.
    Key(String),
    /// Text the owner typed.
    Text(String),
}

/// Colour theme setting (ADR 0023).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Follow the system setting.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// The owner's settings that live with the records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Interface language code, such as `ar` or `en`.
    pub language: String,
    /// Colour theme.
    pub theme: Theme,
    /// Dated local moon sighting corrections for Hijri dates and schedules.
    pub calendar: CalendarSettings,
    #[serde(default)]
    pub location: Option<Location>,
    #[serde(default)]
    pub prayer_method: PrayerMethod,
}

fn default_calendar() -> CalendarSettings {
    CalendarSettings::new(HijriOffset::PLUS_ONE)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "ar".to_owned(),
            theme: Theme::System,
            calendar: default_calendar(),
            location: None,
            prayer_method: PrayerMethod::default(),
        }
    }
}

/// Everything the app keeps for one owner, in the shapes core works with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OwnerData {
    /// Setup is complete. Older saved records count as complete.
    pub first_run_complete: bool,
    /// Set keys currently chosen by the owner.
    pub chosen_sets: BTreeSet<String>,
    /// Catalog key of each deed added from a set.
    pub starter_keys: BTreeMap<DeedId, String>,
    /// Deeds with their dated rules.
    pub deeds: Vec<Deed>,
    /// Highest id issued, including deeds removed later.
    pub last_deed_id: u64,
    /// Deleted deed ids. A stale tab cannot restore these deeds or their marks.
    pub deleted_deeds: BTreeSet<DeedId>,
    /// Display name of each deed.
    pub names: BTreeMap<DeedId, DeedName>,
    /// Marks keyed by deed and werdy day.
    pub marks: BTreeMap<(DeedId, WerdyDay), MarkEntry>,
    /// Change time in Unix milliseconds for each live mark. Old marks use zero.
    pub mark_revisions: BTreeMap<(DeedId, WerdyDay), u64>,
    /// Cleared marks and their change times. These prevent an old tab restoring a mark.
    pub cleared_marks: BTreeMap<(DeedId, WerdyDay), u64>,
    /// Day notes keyed by werdy day.
    pub day_notes: BTreeMap<WerdyDay, DayNote>,
    /// Change time for each live note.
    pub note_revisions: BTreeMap<WerdyDay, u64>,
    /// Deleted notes and their change times.
    pub cleared_notes: BTreeMap<WerdyDay, u64>,
    /// Settings.
    pub settings: Settings,
}

impl OwnerData {
    /// Borrows the records in the form core's scoring functions take.
    pub fn records(&self) -> OwnerRecords<'_> {
        OwnerRecords {
            deeds: &self.deeds,
            marks: &self.marks,
            day_notes: &self.day_notes,
            calendar: self.settings.calendar.clone(),
        }
    }

    /// Returns the next id candidate. Callers reject it when ids are exhausted.
    pub fn next_deed_id(&self) -> DeedId {
        let highest = self
            .deeds
            .iter()
            .map(|deed| deed.id.0)
            .chain(self.deleted_deeds.iter().map(|id| id.0))
            .max()
            .unwrap_or(0)
            .max(self.last_deed_id);
        DeedId(highest.saturating_add(1))
    }

    /// Records a mark or a clear. The change time grows within this copy of the data.
    pub fn set_mark(&mut self, deed_id: DeedId, day: WerdyDay, entry: Option<MarkEntry>, now: u64) {
        let key = (deed_id, day);
        let previous = self
            .mark_revisions
            .get(&key)
            .copied()
            .unwrap_or(0)
            .max(self.cleared_marks.get(&key).copied().unwrap_or(0));
        let changed_at = now.max(previous.saturating_add(1));
        match entry {
            Some(entry) => {
                self.marks.insert(key, entry);
                self.mark_revisions.insert(key, changed_at);
                self.cleared_marks.remove(&key);
            }
            None => {
                self.marks.remove(&key);
                self.mark_revisions.remove(&key);
                self.cleared_marks.insert(key, changed_at);
            }
        }
    }

    /// Records a note edit or deletion with a time that grows in this copy.
    pub fn set_day_note(&mut self, day: WerdyDay, note: Option<DayNote>, now: u64) {
        let previous = self
            .note_revisions
            .get(&day)
            .copied()
            .unwrap_or(0)
            .max(self.cleared_notes.get(&day).copied().unwrap_or(0));
        let changed_at = now.max(previous.saturating_add(1));
        match note {
            Some(note) => {
                self.day_notes.insert(day, note);
                self.note_revisions.insert(day, changed_at);
                self.cleared_notes.remove(&day);
            }
            None => {
                self.day_notes.remove(&day);
                self.note_revisions.remove(&day);
                self.cleared_notes.insert(day, changed_at);
            }
        }
    }
}

/// Combines marks, notes, deleted deeds and issued ids from tab copies.
/// Live deeds, names and settings use the stored copy.
/// For each deed and day, the greater change time wins.
/// A mark or note clear has a saved tombstone. Equal times keep the stored copy.
/// Version one marks have time zero, so any new change wins over them.
pub fn merge_records(stored: &OwnerData, local: &OwnerData) -> OwnerData {
    let mut merged = stored.clone();
    merged.last_deed_id = merged.last_deed_id.max(local.last_deed_id);
    merged.deleted_deeds.extend(&local.deleted_deeds);
    merge_live_marks(&mut merged, local);
    merge_clears(&mut merged, local);
    merge_live_notes(&mut merged, local);
    merge_cleared_notes(&mut merged, local);
    merged
        .deeds
        .retain(|deed| !merged.deleted_deeds.contains(&deed.id));
    merged
        .names
        .retain(|id, _| !merged.deleted_deeds.contains(id));
    merged
        .starter_keys
        .retain(|id, _| !merged.deleted_deeds.contains(id));
    merged
        .marks
        .retain(|(id, _), _| !merged.deleted_deeds.contains(id));
    merged
        .mark_revisions
        .retain(|(id, _), _| !merged.deleted_deeds.contains(id));
    merged
        .cleared_marks
        .retain(|(id, _), _| !merged.deleted_deeds.contains(id));
    merged
}

fn note_change_time(data: &OwnerData, day: WerdyDay) -> u64 {
    data.note_revisions
        .get(&day)
        .or_else(|| data.cleared_notes.get(&day))
        .copied()
        .unwrap_or(0)
}

fn merge_live_notes(merged: &mut OwnerData, local: &OwnerData) {
    for (day, note) in &local.day_notes {
        let time = local.note_revisions.get(day).copied().unwrap_or(0);
        if time > note_change_time(merged, *day)
            || (!merged.day_notes.contains_key(day) && !merged.cleared_notes.contains_key(day))
        {
            merged.day_notes.insert(*day, note.clone());
            merged.cleared_notes.remove(day);
            if time > 0 {
                merged.note_revisions.insert(*day, time);
            } else {
                merged.note_revisions.remove(day);
            }
        }
    }
}

fn merge_cleared_notes(merged: &mut OwnerData, local: &OwnerData) {
    for (day, time) in &local.cleared_notes {
        if *time > note_change_time(merged, *day)
            || (!merged.day_notes.contains_key(day) && !merged.cleared_notes.contains_key(day))
        {
            merged.day_notes.remove(day);
            merged.note_revisions.remove(day);
            merged.cleared_notes.insert(*day, *time);
        }
    }
}

fn merge_live_marks(merged: &mut OwnerData, local: &OwnerData) {
    for (key, entry) in &local.marks {
        let local_time = local.mark_revisions.get(key).copied().unwrap_or(0);
        if local_time > change_time(merged, key) || !has_change(merged, key) {
            merged.marks.insert(*key, *entry);
            merged.cleared_marks.remove(key);
            if local_time > 0 {
                merged.mark_revisions.insert(*key, local_time);
            } else {
                merged.mark_revisions.remove(key);
            }
        }
    }
}

fn merge_clears(merged: &mut OwnerData, local: &OwnerData) {
    for (key, local_time) in &local.cleared_marks {
        if *local_time > change_time(merged, key) || !has_change(merged, key) {
            merged.marks.remove(key);
            merged.mark_revisions.remove(key);
            merged.cleared_marks.insert(*key, *local_time);
        }
    }
}

fn has_change(data: &OwnerData, key: &(DeedId, WerdyDay)) -> bool {
    data.marks.contains_key(key) || data.cleared_marks.contains_key(key)
}

fn change_time(data: &OwnerData, key: &(DeedId, WerdyDay)) -> u64 {
    data.mark_revisions
        .get(key)
        .or_else(|| data.cleared_marks.get(key))
        .copied()
        .unwrap_or(0)
}

/// JSON layout. Maps become lists because JSON object keys must be text.
#[derive(Serialize, Deserialize)]
struct DataFile<S = Settings> {
    schema_version: u32,
    #[serde(default = "legacy_complete")]
    first_run_complete: bool,
    #[serde(default)]
    chosen_sets: Vec<String>,
    #[serde(default)]
    starter_keys: Vec<StarterKeyEntry>,
    deeds: Vec<Deed>,
    #[serde(default)]
    last_deed_id: u64,
    #[serde(default)]
    deleted_deeds: Vec<DeedId>,
    names: Vec<NameEntry>,
    marks: Vec<MarkRecord>,
    #[serde(default)]
    cleared_marks: Vec<ClearRecord>,
    #[serde(default)]
    day_notes: Vec<NoteRecord>,
    #[serde(default)]
    cleared_notes: Vec<NoteClearRecord>,
    settings: S,
}

/// Settings written through schema version three.
#[derive(Deserialize)]
struct LegacySettings {
    language: String,
    theme: Theme,
    #[serde(default = "default_hijri_offset")]
    hijri_offset: HijriOffset,
}

fn legacy_complete() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
struct StarterKeyEntry {
    deed_id: DeedId,
    key: String,
}

fn default_hijri_offset() -> HijriOffset {
    HijriOffset::PLUS_ONE
}

impl From<LegacySettings> for Settings {
    fn from(old: LegacySettings) -> Self {
        Self {
            language: old.language,
            theme: old.theme,
            calendar: CalendarSettings::new(old.hijri_offset),
            location: None,
            prayer_method: PrayerMethod::default(),
        }
    }
}

impl<S> DataFile<S> {
    fn map_settings<T>(self, convert: impl FnOnce(S) -> T) -> DataFile<T> {
        DataFile {
            schema_version: self.schema_version,
            first_run_complete: self.first_run_complete,
            chosen_sets: self.chosen_sets,
            starter_keys: self.starter_keys,
            deeds: self.deeds,
            last_deed_id: self.last_deed_id,
            deleted_deeds: self.deleted_deeds,
            names: self.names,
            marks: self.marks,
            cleared_marks: self.cleared_marks,
            day_notes: self.day_notes,
            cleared_notes: self.cleared_notes,
            settings: convert(self.settings),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct NameEntry {
    deed_id: DeedId,
    name: DeedName,
}

#[derive(Serialize, Deserialize)]
struct MarkRecord {
    deed_id: DeedId,
    day: WerdyDay,
    entry: MarkEntry,
    #[serde(default)]
    changed_at: u64,
}

#[derive(Serialize, Deserialize)]
struct ClearRecord {
    deed_id: DeedId,
    day: WerdyDay,
    changed_at: u64,
}

#[derive(Serialize, Deserialize)]
struct NoteRecord {
    day: WerdyDay,
    note: DayNote,
    #[serde(default)]
    changed_at: u64,
}

#[derive(Serialize, Deserialize)]
struct NoteClearRecord {
    day: WerdyDay,
    changed_at: u64,
}

/// Version only, read first so a newer layout is reported clearly.
#[derive(Deserialize)]
struct VersionProbe {
    schema_version: u32,
}

/// Turns the records into JSON text.
pub fn to_json(data: &OwnerData) -> Result<String, StoreError> {
    let file = DataFile {
        schema_version: SCHEMA_VERSION,
        first_run_complete: data.first_run_complete,
        chosen_sets: data.chosen_sets.iter().cloned().collect(),
        starter_keys: data
            .starter_keys
            .iter()
            .map(|(deed_id, key)| StarterKeyEntry {
                deed_id: *deed_id,
                key: key.clone(),
            })
            .collect(),
        deeds: data.deeds.clone(),
        last_deed_id: data
            .last_deed_id
            .max(data.deeds.iter().map(|deed| deed.id.0).max().unwrap_or(0)),
        deleted_deeds: data.deleted_deeds.iter().copied().collect(),
        names: data
            .names
            .iter()
            .map(|(deed_id, name)| NameEntry {
                deed_id: *deed_id,
                name: name.clone(),
            })
            .collect(),
        marks: data
            .marks
            .iter()
            .map(|((deed_id, day), entry)| MarkRecord {
                deed_id: *deed_id,
                day: *day,
                entry: *entry,
                changed_at: data
                    .mark_revisions
                    .get(&(*deed_id, *day))
                    .copied()
                    .unwrap_or(0),
            })
            .collect(),
        cleared_marks: data
            .cleared_marks
            .iter()
            .map(|((deed_id, day), changed_at)| ClearRecord {
                deed_id: *deed_id,
                day: *day,
                changed_at: *changed_at,
            })
            .collect(),
        day_notes: data
            .day_notes
            .iter()
            .map(|(day, note)| NoteRecord {
                day: *day,
                note: note.clone(),
                changed_at: data.note_revisions.get(day).copied().unwrap_or(0),
            })
            .collect(),
        cleared_notes: data
            .cleared_notes
            .iter()
            .map(|(day, changed_at)| NoteClearRecord {
                day: *day,
                changed_at: *changed_at,
            })
            .collect(),
        settings: data.settings.clone(),
    };
    serde_json::to_string(&file).map_err(|error| StoreError::Corrupt(error.to_string()))
}

/// Reads records from JSON text. Bad JSON or invalid values return an error.
pub fn from_json(text: &str) -> Result<OwnerData, StoreError> {
    let corrupt = |error: serde_json::Error| StoreError::Corrupt(error.to_string());
    let probe: VersionProbe = serde_json::from_str(text).map_err(corrupt)?;
    if !(1..=SCHEMA_VERSION).contains(&probe.schema_version) {
        return Err(StoreError::UnsupportedVersion(probe.schema_version));
    }
    let file: DataFile = if probe.schema_version < 4 {
        let legacy: DataFile<LegacySettings> = serde_json::from_str(text).map_err(corrupt)?;
        legacy.map_settings(Settings::from)
    } else {
        serde_json::from_str(text).map_err(corrupt)?
    };
    if let Some(location) = &file.settings.location
        && (!(-9_000..=9_000).contains(&location.latitude_hundredths)
            || !(-18_000..=18_000).contains(&location.longitude_hundredths))
    {
        return Err(StoreError::Corrupt(
            "location is outside coordinate range".to_owned(),
        ));
    }

    let mut deed_ids = BTreeSet::new();
    for deed in &file.deeds {
        if !deed_ids.insert(deed.id) {
            return Err(StoreError::DuplicateEntry);
        }
    }
    let mut deleted_deeds = BTreeSet::new();
    for id in file.deleted_deeds {
        if deed_ids.contains(&id) || !deleted_deeds.insert(id) {
            return Err(StoreError::DuplicateEntry);
        }
    }
    let last_deed_id = file.last_deed_id.max(
        deed_ids
            .iter()
            .chain(&deleted_deeds)
            .map(|id| id.0)
            .max()
            .unwrap_or(0),
    );

    let mut marks = BTreeMap::new();
    let mut mark_revisions = BTreeMap::new();
    for record in file.marks {
        if marks
            .insert((record.deed_id, record.day), record.entry)
            .is_some()
        {
            return Err(StoreError::DuplicateEntry);
        }
        if record.changed_at > 0 {
            mark_revisions.insert((record.deed_id, record.day), record.changed_at);
        }
    }
    let mut cleared_marks = BTreeMap::new();
    for record in file.cleared_marks {
        let key = (record.deed_id, record.day);
        if marks.contains_key(&key) || cleared_marks.insert(key, record.changed_at).is_some() {
            return Err(StoreError::DuplicateEntry);
        }
    }
    let mut day_notes = BTreeMap::new();
    let mut note_revisions = BTreeMap::new();
    for record in file.day_notes {
        if day_notes.insert(record.day, record.note).is_some() {
            return Err(StoreError::DuplicateEntry);
        }
        if record.changed_at > 0 {
            note_revisions.insert(record.day, record.changed_at);
        }
    }
    let mut cleared_notes = BTreeMap::new();
    for record in file.cleared_notes {
        if day_notes.contains_key(&record.day)
            || cleared_notes
                .insert(record.day, record.changed_at)
                .is_some()
        {
            return Err(StoreError::DuplicateEntry);
        }
    }
    let mut names = BTreeMap::new();
    for entry in file.names {
        if names.insert(entry.deed_id, entry.name).is_some() {
            return Err(StoreError::DuplicateEntry);
        }
    }
    let chosen_sets: BTreeSet<_> = file.chosen_sets.iter().cloned().collect();
    if chosen_sets.len() != file.chosen_sets.len() {
        return Err(StoreError::DuplicateEntry);
    }
    let mut starter_keys = BTreeMap::new();
    for entry in file.starter_keys {
        if !deed_ids.contains(&entry.deed_id)
            || starter_keys.insert(entry.deed_id, entry.key).is_some()
        {
            return Err(StoreError::DuplicateEntry);
        }
    }
    Ok(OwnerData {
        first_run_complete: file.first_run_complete,
        chosen_sets,
        starter_keys,
        deeds: file.deeds,
        last_deed_id,
        deleted_deeds,
        names,
        marks,
        mark_revisions,
        cleared_marks,
        day_notes,
        note_revisions,
        cleared_notes,
        settings: file.settings,
    })
}

/// A place that holds the owner's records.
pub trait Store {
    /// Loads the records, or `None` when nothing was saved yet.
    fn load(&self) -> Result<Option<OwnerData>, StoreError>;
    /// Saves the records, replacing what was there.
    fn save(&self, data: &OwnerData) -> Result<(), StoreError>;
}

/// Store that keeps the JSON in memory. Used in tests and as a fallback when
/// no lasting storage is available.
#[derive(Debug, Default)]
pub struct MemoryStore {
    text: RefCell<Option<String>>,
}

impl Store for MemoryStore {
    fn load(&self) -> Result<Option<OwnerData>, StoreError> {
        self.text.borrow().as_deref().map(from_json).transpose()
    }

    fn save(&self, data: &OwnerData) -> Result<(), StoreError> {
        *self.text.borrow_mut() = Some(to_json(data)?);
        Ok(())
    }
}
