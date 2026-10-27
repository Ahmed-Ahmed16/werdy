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
pub const SCHEMA_VERSION: u32 = 3;

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
    /// Local moon sighting correction for Hijri dates and schedules.
    #[serde(default = "default_hijri_offset")]
    pub hijri_offset: HijriOffset,
}

fn default_hijri_offset() -> HijriOffset {
    HijriOffset::PLUS_ONE
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "ar".to_owned(),
            theme: Theme::System,
            hijri_offset: default_hijri_offset(),
        }
    }
}

/// Everything the app keeps for one owner, in the shapes core works with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OwnerData {
    /// Deeds with their dated rules.
    pub deeds: Vec<Deed>,
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
            calendar: CalendarSettings {
                hijri_offset: self.settings.hijri_offset,
            },
        }
    }

    /// Returns an id one above the highest deed id, so ids are never reused.
    pub fn next_deed_id(&self) -> DeedId {
        let highest = self.deeds.iter().map(|deed| deed.id.0).max().unwrap_or(0);
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
}

/// Combines mark changes from tab copies. Other records use the stored copy.
/// For each deed and day, the greater change time wins.
/// A clear has a saved tombstone. Equal times keep the first, stored copy.
/// Version one marks have time zero, so any new change wins over them.
pub fn merge_records(stored: &OwnerData, local: &OwnerData) -> OwnerData {
    let mut merged = stored.clone();
    merge_live_marks(&mut merged, local);
    merge_clears(&mut merged, local);
    merged
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
struct DataFile {
    schema_version: u32,
    deeds: Vec<Deed>,
    names: Vec<NameEntry>,
    marks: Vec<MarkRecord>,
    #[serde(default)]
    cleared_marks: Vec<ClearRecord>,
    day_notes: Vec<NoteRecord>,
    settings: Settings,
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
        deeds: data.deeds.clone(),
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
    let file: DataFile = serde_json::from_str(text).map_err(corrupt)?;

    let mut deed_ids = BTreeSet::new();
    for deed in &file.deeds {
        if !deed_ids.insert(deed.id) {
            return Err(StoreError::DuplicateEntry);
        }
    }

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
    for record in file.day_notes {
        if day_notes.insert(record.day, record.note).is_some() {
            return Err(StoreError::DuplicateEntry);
        }
    }
    let mut names = BTreeMap::new();
    for entry in file.names {
        if names.insert(entry.deed_id, entry.name).is_some() {
            return Err(StoreError::DuplicateEntry);
        }
    }
    Ok(OwnerData {
        deeds: file.deeds,
        names,
        marks,
        mark_revisions,
        cleared_marks,
        day_notes,
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
