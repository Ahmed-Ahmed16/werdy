use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use werdy_core::{CalendarSettings, DayNote, Deed, DeedId, DeedRule, MarkEntry, WerdyDay};
use werdy_store::{DeedName, Digits, OwnerData, Settings, Theme};

use crate::{Pending, Sync, SyncError, invalid};

/// Stable across phones. Local numeric ids are only projection keys.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DeedKey {
    pub install: String,
    pub sequence: u64,
}

/// Deed metadata is one row. Rules are separate rows keyed by their start day,
/// so edits to different dated rules do not discard one another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeedRow {
    pub created_on: WerdyDay,
    pub archived_from: Option<WerdyDay>,
    pub past_archives: Vec<(WerdyDay, WerdyDay)>,
    pub name: Option<DeedName>,
    pub starter_key: Option<String>,
}

/// Each setting has its own conflict key. Location and prayer method stay local.
/// Setup, demo identity, issued local ids and welcome notices also stay local.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Setting {
    Language(String),
    Theme(Theme),
    Digits(Digits),
    Calendar(CalendarSettings),
}

/// A whole row replacement. `None` is a lasting tombstone, never an omission.
/// Entry days are display history and do not determine conflict order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Change {
    Deed {
        id: DeedKey,
        row: Option<DeedRow>,
    },
    Rule {
        id: DeedKey,
        from_day: WerdyDay,
        rule: DeedRule,
    },
    Mark {
        id: DeedKey,
        day: WerdyDay,
        entry: Option<MarkEntry>,
    },
    Note {
        day: WerdyDay,
        note: Option<DayNote>,
    },
    Setting(Setting),
    ChosenSet {
        key: String,
        chosen: bool,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum RowKey {
    Deed(DeedKey),
    Rule(DeedKey, WerdyDay),
    Mark(DeedKey, WerdyDay),
    Note(WerdyDay),
    Setting(u8),
    ChosenSet(String),
}

impl Change {
    pub(crate) fn key(&self) -> RowKey {
        match self {
            Self::Deed { id, .. } => RowKey::Deed(id.clone()),
            Self::Rule { id, from_day, .. } => RowKey::Rule(id.clone(), *from_day),
            Self::Mark { id, day, .. } => RowKey::Mark(id.clone(), *day),
            Self::Note { day, .. } => RowKey::Note(*day),
            Self::ChosenSet { key, .. } => RowKey::ChosenSet(key.clone()),
            Self::Setting(setting) => RowKey::Setting(match setting {
                Setting::Language(_) => 0,
                Setting::Theme(_) => 1,
                Setting::Digits(_) => 2,
                Setting::Calendar(_) => 3,
            }),
        }
    }
}

pub(crate) fn validate_install(install: &str) -> Result<(), SyncError> {
    if install.is_empty()
        || install.len() > 128
        || !install
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(invalid(
            "install id must be 1 to 128 ASCII letters, digits or underscores",
        ));
    }
    Ok(())
}

pub(crate) fn validate_pending(pending: &Pending) -> Result<(), SyncError> {
    validate_install(&pending.install)?;
    if pending.operation == 0 {
        return Err(invalid("operation id is zero"));
    }
    match &pending.change {
        Change::Deed { id, .. } | Change::Rule { id, .. } | Change::Mark { id, .. } => {
            validate_install(&id.install)?;
            if id.sequence == 0 {
                return Err(invalid("deed sequence is zero"));
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn snapshot(engine: &mut Sync, records: &OwnerData) -> Result<Vec<Change>, SyncError> {
    validate_install(&engine.install)?;
    validate_records(records)?;
    let mut snapshot = Vec::new();
    for deed in &records.deeds {
        let id = engine.wire_id(deed.id)?;
        snapshot.push(Change::Deed {
            id: id.clone(),
            row: Some(DeedRow {
                created_on: deed.created_on(),
                archived_from: deed.archived_from(),
                past_archives: deed.past_archives().to_vec(),
                name: records.names.get(&deed.id).cloned(),
                starter_key: records.starter_keys.get(&deed.id).cloned(),
            }),
        });
        snapshot.extend(deed.rule_versions().map(|(from_day, rule)| Change::Rule {
            id: id.clone(),
            from_day,
            rule: rule.clone(),
        }));
    }
    for local in &records.deleted_deeds {
        snapshot.push(Change::Deed {
            id: engine.wire_id(*local)?,
            row: None,
        });
    }
    for ((local, day), entry) in &records.marks {
        snapshot.push(Change::Mark {
            id: engine.wire_id(*local)?,
            day: *day,
            entry: Some(*entry),
        });
    }
    for (local, day) in records.cleared_marks.keys() {
        if !records.deleted_deeds.contains(local) {
            snapshot.push(Change::Mark {
                id: engine.wire_id(*local)?,
                day: *day,
                entry: None,
            });
        }
    }
    snapshot.extend(records.day_notes.iter().map(|(day, note)| Change::Note {
        day: *day,
        note: Some(note.clone()),
    }));
    snapshot.extend(records.cleared_notes.keys().map(|day| Change::Note {
        day: *day,
        note: None,
    }));
    snapshot.extend(settings_snapshot(&records.settings));
    snapshot.extend(records.chosen_sets.iter().map(|key| Change::ChosenSet {
        key: key.clone(),
        chosen: true,
    }));
    retain_removals(&mut snapshot, &engine.observed)?;
    Ok(snapshot)
}

pub(crate) fn settings_snapshot(settings: &Settings) -> Vec<Change> {
    [
        Setting::Language(settings.language.clone()),
        Setting::Theme(settings.theme),
        Setting::Digits(settings.digits),
        Setting::Calendar(settings.calendar.clone()),
    ]
    .into_iter()
    .map(Change::Setting)
    .collect()
}

fn validate_records(records: &OwnerData) -> Result<(), SyncError> {
    let mut ids = BTreeSet::new();
    for deed in &records.deeds {
        if !ids.insert(deed.id) || records.deleted_deeds.contains(&deed.id) {
            return Err(invalid("duplicate or deleted local deed"));
        }
    }
    if records
        .marks
        .keys()
        .any(|key| !ids.contains(&key.0) || records.cleared_marks.contains_key(key))
        || records
            .day_notes
            .keys()
            .any(|day| records.cleared_notes.contains_key(day))
    {
        return Err(invalid("orphan mark or conflicting tombstone"));
    }
    Ok(())
}

fn retain_removals(snapshot: &mut Vec<Change>, previous: &[Change]) -> Result<(), SyncError> {
    let keys: BTreeSet<_> = snapshot.iter().map(Change::key).collect();
    for old in previous.iter().filter(|old| !keys.contains(&old.key())) {
        let tombstone = match old {
            Change::Mark { id, day, .. } => Change::Mark {
                id: id.clone(),
                day: *day,
                entry: None,
            },
            Change::Note { day, .. } => Change::Note {
                day: *day,
                note: None,
            },
            Change::ChosenSet { key, .. } => Change::ChosenSet {
                key: key.clone(),
                chosen: false,
            },
            Change::Deed { id, .. } => Change::Deed {
                id: id.clone(),
                row: None,
            },
            Change::Rule { id, .. }
                if snapshot.iter().any(
                    |row| matches!(row, Change::Deed { id: deleted, row: None } if deleted == id),
                ) =>
            {
                continue;
            }
            Change::Rule { .. } => return Err(invalid("dated rule removal is not supported")),
            Change::Setting(_) => return Err(invalid("setting missing from snapshot")),
        };
        snapshot.push(tombstone);
    }
    Ok(())
}

pub(crate) fn project(engine: &mut Sync, records: &mut OwnerData) -> Result<(), SyncError> {
    let changes: Vec<_> = engine
        .accepted
        .iter()
        .map(|row| (row.pending.change.clone(), row.stamp.received_at))
        .collect();
    let mut ids = BTreeMap::new();
    for (change, _) in &changes {
        if let Change::Deed { id, row } = change {
            let local = engine.local_id(id, records)?;
            ids.insert(id.clone(), (local, row.is_some()));
            let replacement = row
                .as_ref()
                .map(|header| build_deed(local, id, header, &changes).map(|deed| (deed, header)))
                .transpose()?;
            replace_deed(records, local, replacement);
        }
    }
    for (change, time) in &changes {
        apply_row(records, change, *time, &ids)?;
    }
    records.deeds.sort_by_key(|deed| deed.id);
    validate_records(records)
}

fn replace_deed(records: &mut OwnerData, local: DeedId, replacement: Option<(Deed, &DeedRow)>) {
    records.deeds.retain(|deed| deed.id != local);
    records.names.remove(&local);
    records.starter_keys.remove(&local);
    if let Some((deed, header)) = replacement {
        records.deeds.push(deed);
        records.deleted_deeds.remove(&local);
        if let Some(name) = &header.name {
            records.names.insert(local, name.clone());
        }
        if let Some(key) = &header.starter_key {
            records.starter_keys.insert(local, key.clone());
        }
    } else {
        records.deleted_deeds.insert(local);
        records.marks.retain(|(id, _), _| *id != local);
        records.mark_revisions.retain(|(id, _), _| *id != local);
        records.cleared_marks.retain(|(id, _), _| *id != local);
    }
}

fn build_deed(
    local: DeedId,
    remote: &DeedKey,
    header: &DeedRow,
    changes: &[(Change, u64)],
) -> Result<Deed, SyncError> {
    let rules: BTreeMap<_, _> = changes
        .iter()
        .filter_map(|(row, _)| match row {
            Change::Rule { id, from_day, rule } if id == remote => Some((*from_day, rule.clone())),
            _ => None,
        })
        .collect();
    let mut versions = rules.into_iter();
    let (created_on, rule) = versions
        .next()
        .ok_or_else(|| invalid("deed has no initial rule"))?;
    if created_on != header.created_on {
        return Err(invalid("initial rule differs from creation day"));
    }
    let mut deed = Deed::new(local, created_on, rule);
    for (day, rule) in versions {
        deed.change_rule(day, rule)
            .map_err(|error| invalid(&error.to_string()))?;
    }
    for (start, end) in &header.past_archives {
        if end <= start {
            return Err(invalid("archive period is empty or reversed"));
        }
        deed.archive(*start)
            .map_err(|error| invalid(&error.to_string()))?;
        deed.restore(*end)
            .map_err(|error| invalid(&error.to_string()))?;
    }
    if let Some(start) = header.archived_from {
        deed.archive(start)
            .map_err(|error| invalid(&error.to_string()))?;
    }
    Ok(deed)
}

fn apply_row(
    records: &mut OwnerData,
    change: &Change,
    time: u64,
    ids: &BTreeMap<DeedKey, (DeedId, bool)>,
) -> Result<(), SyncError> {
    match change {
        Change::Mark { id, day, entry } => {
            let (local, live) = ids
                .get(id)
                .ok_or_else(|| invalid("mark references unknown deed"))?;
            if *live {
                let key = (*local, *day);
                records.marks.remove(&key);
                records.mark_revisions.remove(&key);
                records.cleared_marks.remove(&key);
                if let Some(entry) = entry {
                    records.marks.insert(key, *entry);
                    records.mark_revisions.insert(key, time);
                } else {
                    records.cleared_marks.insert(key, time);
                }
            }
        }
        Change::Rule { id, .. } if !ids.contains_key(id) => {
            return Err(invalid("rule references unknown deed"));
        }
        Change::Note { day, note } => {
            records.day_notes.remove(day);
            records.note_revisions.remove(day);
            records.cleared_notes.remove(day);
            if let Some(note) = note {
                records.day_notes.insert(*day, note.clone());
                records.note_revisions.insert(*day, time);
            } else {
                records.cleared_notes.insert(*day, time);
            }
        }
        Change::Setting(setting) => apply_setting(records, setting),
        Change::ChosenSet { key, chosen } => {
            if *chosen {
                records.chosen_sets.insert(key.clone());
            } else {
                records.chosen_sets.remove(key);
            }
        }
        _ => {}
    }
    Ok(())
}

fn apply_setting(records: &mut OwnerData, setting: &Setting) {
    match setting {
        Setting::Language(language) => records.settings.language = language.clone(),
        Setting::Theme(theme) => records.settings.theme = *theme,
        Setting::Digits(digits) => records.settings.digits = *digits,
        Setting::Calendar(calendar) => records.settings.calendar = calendar.clone(),
    }
}
