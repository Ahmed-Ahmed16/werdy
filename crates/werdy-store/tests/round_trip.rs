use std::collections::BTreeMap;

use werdy_core::{
    CalendarSettings, CategoryKey, DayNote, Deed, DeedId, DeedRule, ExcuseCredit, HijriOffset,
    Mark, MarkEntry, Period, Schedule, ScoreBands, SlotKey, Target, WerdyDay, build_report,
};
use werdy_store::{
    DeedName, Digits, MemoryStore, OwnerData, SCHEMA_VERSION, Settings, Store, StoreError, Theme,
    from_json, to_json,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn rule(slot: &str, target: Option<Target>) -> DeedRule {
    DeedRule {
        schedule: Schedule::EveryDay,
        target,
        category: CategoryKey::new("prayer"),
        slot: SlotKey::new(slot),
    }
}

fn sample() -> Result<OwnerData, Box<dyn std::error::Error>> {
    let start = WerdyDay::new(2026, 10, 1)?;
    let later = WerdyDay::new(2026, 10, 15)?;
    let mut quran = Deed::new(DeedId(2), start, rule("morning", None));
    quran.change_rule(later, rule("morning", Some(Target::new(5, "page")?)))?;
    let fajr = Deed::new(DeedId(1), start, rule("fajr", None));

    let mut marks = BTreeMap::new();
    let mut day = start;
    for step in 0..20u32 {
        let mark = match step % 4 {
            0 => Mark::Done,
            1 => Mark::partial_without_target(),
            2 => Mark::Missed,
            _ => Mark::Count(3),
        };
        marks.insert((DeedId(1), day), MarkEntry::new(mark, day));
        marks.insert((DeedId(2), day), MarkEntry::new(Mark::Count(step % 6), day));
        day = day.next()?;
    }
    let day_notes = BTreeMap::from([(
        WerdyDay::new(2026, 10, 5)?,
        DayNote::new("ill", ExcuseCredit::Full, WerdyDay::new(2026, 10, 9)?)?,
    )]);

    Ok(OwnerData {
        first_run_complete: true,
        chosen_sets: ["five_prayers".to_owned()].into(),
        starter_keys: BTreeMap::from([(DeedId(1), "fajr_on_time".to_owned())]),
        deeds: vec![fajr, quran],
        last_deed_id: 2,
        deleted_deeds: Default::default(),
        names: BTreeMap::from([
            (DeedId(1), DeedName::Key("starter-fajr-on-time".to_owned())),
            (DeedId(2), DeedName::Text("ورد القرآن".to_owned())),
        ]),
        marks,
        mark_revisions: BTreeMap::new(),
        cleared_marks: BTreeMap::new(),
        day_notes,
        note_revisions: BTreeMap::new(),
        cleared_notes: BTreeMap::new(),
        dismissed_milestones: [7, 30].into(),
        last_welcome_day: Some(WerdyDay::new(2026, 10, 20)?),
        settings: Settings {
            language: "ar".to_owned(),
            theme: Theme::Dark,
            digits: Digits::ArabicIndic,
            calendar: CalendarSettings::new(HijriOffset::PLUS_ONE),
            location: werdy_store::Location::new(
                Some("cairo".to_owned()),
                Some("EG".to_owned()),
                30.04,
                31.24,
            ),
            prayer_method: werdy_store::PrayerMethod::EgyptianGeneralAuthority,
        },
    })
}

#[test]
fn save_then_load_returns_identical_data() -> TestResult {
    let data = sample()?;
    let store = MemoryStore::default();

    store.save(&data)?;

    assert_eq!(store.load()?, Some(data));
    Ok(())
}

#[test]
fn umm_al_qura_round_trips_in_schema_eleven() -> TestResult {
    let mut data = sample()?;
    data.settings.prayer_method = werdy_store::PrayerMethod::UmmAlQura;
    let json = to_json(&data)?;
    let value: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(value["schema_version"], 11);
    assert_eq!(value["settings"]["prayer_method"], "umm_al_qura");
    assert_eq!(from_json(&json)?, data);
    Ok(())
}

#[test]
fn schema_ten_method_migrates_without_change() -> TestResult {
    let data = sample()?;
    let mut old: serde_json::Value = serde_json::from_str(&to_json(&data)?)?;
    old["schema_version"] = 10.into();
    assert_eq!(from_json(&old.to_string())?, data);
    Ok(())
}

#[test]
fn older_file_loads_without_notice_history() -> TestResult {
    let mut old: serde_json::Value = serde_json::from_str(&to_json(&sample()?)?)?;
    old["schema_version"] = 9.into();
    let object = old.as_object_mut().ok_or("expected object")?;
    object.remove("dismissed_milestones");
    object.remove("last_welcome_day");
    let loaded = from_json(&old.to_string())?;
    assert!(loaded.dismissed_milestones.is_empty());
    assert_eq!(loaded.last_welcome_day, None);
    Ok(())
}

#[test]
fn notice_history_merges_from_stale_tabs() -> TestResult {
    let mut first = sample()?;
    let mut stale = first.clone();
    first.dismissed_milestones.insert(100);
    first.last_welcome_day = Some(WerdyDay::new(2026, 10, 22)?);
    stale.dismissed_milestones.insert(7);
    stale.last_welcome_day = Some(WerdyDay::new(2026, 10, 21)?);
    let merged = werdy_store::merge_records(&first, &stale);
    assert_eq!(merged.dismissed_milestones, [7, 30, 100].into());
    assert_eq!(merged.last_welcome_day, first.last_welcome_day);
    Ok(())
}

#[test]
fn version_eight_loads_western_digits() -> TestResult {
    let mut old: serde_json::Value = serde_json::from_str(&to_json(&sample()?)?)?;
    old["schema_version"] = 8.into();
    old["settings"]
        .as_object_mut()
        .ok_or("settings missing")?
        .remove("digits");
    let loaded = from_json(&old.to_string())?;
    assert_eq!(loaded.settings.digits, Digits::Western);
    Ok(())
}

#[test]
fn note_edits_and_clears_merge_across_tabs() -> TestResult {
    let store = MemoryStore::default();
    let mut first = sample()?;
    store.save(&first)?;
    let mut stale = store.load()?.ok_or("missing data")?;
    let day = WerdyDay::new(2026, 10, 6)?;
    first.set_day_note(day, Some(DayNote::new("Ill", ExcuseCredit::Half, day)?), 10);
    store.save(&first)?;
    stale.set_mark(DeedId(1), day, Some(MarkEntry::new(Mark::Done, day)), 11);
    store.save(&werdy_store::merge_records(&first, &stale))?;
    let merged = store.load()?.ok_or("missing merged data")?;
    assert_eq!(
        merged.day_notes.get(&day).map(|note| note.credit()),
        Some(ExcuseCredit::Half)
    );
    let mut deleting = merged.clone();
    deleting.set_day_note(day, None, 12);
    let restored = werdy_store::merge_records(&deleting, &stale);
    assert!(!restored.day_notes.contains_key(&day));
    assert_eq!(from_json(&to_json(&restored)?)?, restored);
    Ok(())
}

#[test]
fn version_six_notes_load_without_revisions() -> TestResult {
    let original = sample()?;
    let mut old: serde_json::Value = serde_json::from_str(&to_json(&original)?)?;
    old["schema_version"] = 6.into();
    old.as_object_mut()
        .ok_or("expected object")?
        .remove("cleared_notes");
    for note in old["day_notes"].as_array_mut().ok_or("expected notes")? {
        note.as_object_mut()
            .ok_or("expected note")?
            .remove("changed_at");
    }
    assert_eq!(from_json(&serde_json::to_string(&old)?)?, original);
    Ok(())
}

#[test]
fn both_version_seven_formats_still_load() -> TestResult {
    let original = sample()?;
    let current: serde_json::Value = serde_json::from_str(&to_json(&original)?)?;

    let mut main = current.clone();
    main["schema_version"] = 7.into();
    let main_fields = main.as_object_mut().ok_or("expected object")?;
    main_fields.remove("day_notes");
    main_fields.remove("cleared_notes");
    let mut without_notes = original.clone();
    without_notes.day_notes.clear();
    assert_eq!(from_json(&main.to_string())?, without_notes);

    let mut branch = current;
    branch["schema_version"] = 7.into();
    let branch_fields = branch.as_object_mut().ok_or("expected object")?;
    branch_fields.remove("first_run_complete");
    branch_fields.remove("chosen_sets");
    branch_fields.remove("starter_keys");
    let settings = branch_fields["settings"]
        .as_object_mut()
        .ok_or("expected settings")?;
    settings.remove("location");
    settings.remove("prayer_method");
    let mut without_first_run = original;
    without_first_run.chosen_sets.clear();
    without_first_run.starter_keys.clear();
    without_first_run.settings.location = None;
    without_first_run.settings.prayer_method = werdy_store::PrayerMethod::default();
    assert_eq!(from_json(&branch.to_string())?, without_first_run);
    Ok(())
}

#[test]
fn later_note_edit_wins_for_same_day() -> TestResult {
    let day = WerdyDay::new(2026, 10, 6)?;
    let mut earlier = sample()?;
    let mut later = earlier.clone();
    earlier.set_day_note(day, Some(DayNote::new("Ill", ExcuseCredit::Half, day)?), 10);
    later.set_day_note(
        day,
        Some(DayNote::new("Travelled", ExcuseCredit::Full, day)?),
        11,
    );
    let merged = werdy_store::merge_records(&earlier, &later);
    assert_eq!(
        merged.day_notes.get(&day).map(|note| note.credit()),
        Some(ExcuseCredit::Full)
    );
    Ok(())
}

#[test]
fn restored_archive_periods_survive_round_trip_and_version_four_still_loads() -> TestResult {
    let mut data = sample()?;
    let archived = WerdyDay::new(2026, 10, 6)?;
    let restored = WerdyDay::new(2026, 10, 9)?;
    data.deeds[0].archive(archived)?;
    data.deeds[0].restore(restored)?;
    let json = to_json(&data)?;
    assert_eq!(from_json(&json)?, data);

    let older_data = sample()?;
    let mut old: serde_json::Value = serde_json::from_str(&to_json(&older_data)?)?;
    old["schema_version"] = 4.into();
    if let Some(deeds) = old["deeds"].as_array_mut() {
        for deed in deeds {
            if let Some(fields) = deed.as_object_mut() {
                fields.remove("past_archives");
            }
        }
    }
    assert_eq!(from_json(&serde_json::to_string(&old)?)?, older_data);
    Ok(())
}

#[test]
fn an_empty_store_loads_nothing() -> TestResult {
    assert_eq!(MemoryStore::default().load()?, None);
    Ok(())
}

#[test]
fn a_report_is_the_same_before_and_after_a_round_trip() -> TestResult {
    let data = sample()?;
    let loaded = from_json(&to_json(&data)?)?;
    let today = WerdyDay::new(2026, 10, 21)?;

    for period in [Period::ThreeDays, Period::Week, Period::Month] {
        assert_eq!(
            build_report(&loaded.records(), period, 0, today, ScoreBands::default()),
            build_report(&data.records(), period, 0, today, ScoreBands::default()),
        );
    }
    Ok(())
}

#[test]
fn corrupt_json_is_an_error() {
    for text in ["", "{", "not json", "[1, 2]", r#"{"schema_version": 1}"#] {
        assert!(
            matches!(from_json(text), Err(StoreError::Corrupt(_))),
            "{text:?} should be corrupt"
        );
    }
}

#[test]
fn invalid_values_inside_valid_json_are_an_error() -> TestResult {
    let original = to_json(&sample()?)?;
    let json = original.replace(
        "\"numerator\":1,\"denominator\":2",
        "\"numerator\":7,\"denominator\":2",
    );
    assert_ne!(json, original);

    assert!(matches!(from_json(&json), Err(StoreError::Corrupt(_))));
    Ok(())
}

#[test]
fn a_newer_schema_version_is_refused() -> TestResult {
    let json = to_json(&sample()?)?.replace(
        &format!("\"schema_version\":{SCHEMA_VERSION}"),
        "\"schema_version\":99",
    );

    assert_eq!(from_json(&json), Err(StoreError::UnsupportedVersion(99)));
    Ok(())
}

#[test]
fn location_outside_earth_is_rejected_on_load() -> TestResult {
    let json = to_json(&sample()?)?.replace(
        "\"latitude_hundredths\":3004",
        "\"latitude_hundredths\":9001",
    );
    assert!(matches!(from_json(&json), Err(StoreError::Corrupt(_))));
    Ok(())
}

#[test]
fn merge_keeps_marks_from_both_tabs() -> TestResult {
    let mut left = OwnerData::default();
    let mut right = OwnerData::default();
    let day = WerdyDay::new(2026, 10, 27)?;
    left.set_mark(DeedId(1), day, Some(MarkEntry::new(Mark::Done, day)), 10);
    right.set_mark(DeedId(2), day, Some(MarkEntry::new(Mark::Missed, day)), 11);

    let merged = werdy_store::merge_records(&left, &right);
    assert_eq!(merged.marks.len(), 2);
    Ok(())
}

#[test]
fn later_change_wins_for_same_mark() -> TestResult {
    let mut first = OwnerData::default();
    let mut later = OwnerData::default();
    let day = WerdyDay::new(2026, 10, 27)?;
    first.set_mark(DeedId(1), day, Some(MarkEntry::new(Mark::Done, day)), 10);
    later.set_mark(DeedId(1), day, Some(MarkEntry::new(Mark::Missed, day)), 11);

    let merged = werdy_store::merge_records(&later, &first);
    assert_eq!(merged.marks[&(DeedId(1), day)].mark, Mark::Missed);
    Ok(())
}

#[test]
fn clear_survives_round_trip_and_stale_merge() -> TestResult {
    let mut stale = OwnerData::default();
    let day = WerdyDay::new(2026, 10, 27)?;
    stale.set_mark(DeedId(1), day, Some(MarkEntry::new(Mark::Done, day)), 10);
    let mut cleared = stale.clone();
    cleared.set_mark(DeedId(1), day, None, 11);
    let loaded = from_json(&to_json(&cleared)?)?;

    let merged = werdy_store::merge_records(&loaded, &stale);
    assert!(!merged.marks.contains_key(&(DeedId(1), day)));
    assert_eq!(merged.cleared_marks.get(&(DeedId(1), day)), Some(&11));
    Ok(())
}

#[test]
fn version_one_marks_load_with_zero_revision() -> TestResult {
    let mut json: serde_json::Value = serde_json::from_str(&to_json(&sample()?)?)?;
    json["schema_version"] = 1.into();
    json["settings"]["hijri_offset"] = 1.into();
    json["settings"]
        .as_object_mut()
        .ok_or("settings is not an object")?
        .remove("calendar");
    let object = json.as_object_mut().ok_or("file is not an object")?;
    object.remove("cleared_marks");
    for mark in object["marks"]
        .as_array_mut()
        .ok_or("marks is not a list")?
    {
        mark.as_object_mut()
            .ok_or("mark is not an object")?
            .remove("changed_at");
    }

    let loaded = from_json(&json.to_string())?;
    assert_eq!(loaded.mark_revisions, BTreeMap::new());
    assert_eq!(loaded.marks, sample()?.marks);
    Ok(())
}

#[test]
fn equal_change_times_keep_stored_mark() -> TestResult {
    let day = WerdyDay::new(2026, 10, 27)?;
    let mut stored = OwnerData::default();
    let mut local = OwnerData::default();
    stored.set_mark(DeedId(1), day, Some(MarkEntry::new(Mark::Done, day)), 10);
    local.set_mark(DeedId(1), day, Some(MarkEntry::new(Mark::Missed, day)), 10);

    let merged = werdy_store::merge_records(&stored, &local);
    assert_eq!(merged.marks[&(DeedId(1), day)].mark, Mark::Done);
    Ok(())
}

#[test]
fn two_marks_for_the_same_deed_and_day_are_an_error() -> TestResult {
    let mut value: serde_json::Value = serde_json::from_str(&to_json(&sample()?)?)?;
    let marks = value["marks"].as_array_mut().ok_or("marks is not a list")?;
    let first = marks.first().cloned().ok_or("no marks")?;
    marks.push(first);

    assert_eq!(
        from_json(&value.to_string()),
        Err(StoreError::DuplicateEntry)
    );
    Ok(())
}

#[test]
fn two_deeds_with_the_same_id_are_an_error_on_load() -> TestResult {
    let mut value: serde_json::Value = serde_json::from_str(&to_json(&sample()?)?)?;
    let deeds = value["deeds"].as_array_mut().ok_or("deeds is not a list")?;
    let first = deeds.first().cloned().ok_or("no deeds")?;
    deeds.push(first);

    assert_eq!(
        from_json(&value.to_string()),
        Err(StoreError::DuplicateEntry)
    );
    Ok(())
}

#[test]
fn the_next_deed_id_follows_the_highest_id() -> TestResult {
    assert_eq!(sample()?.next_deed_id(), DeedId(3));
    assert_eq!(OwnerData::default().next_deed_id(), DeedId(1));
    Ok(())
}

#[test]
fn deleted_highest_id_stays_reserved_after_round_trip_and_merge() -> TestResult {
    let mut data = sample()?;
    data.deleted_deeds.insert(DeedId(2));
    data.deeds.retain(|deed| deed.id != DeedId(2));
    let loaded = from_json(&to_json(&data)?)?;
    assert_eq!(loaded.next_deed_id(), DeedId(3));
    let merged = werdy_store::merge_records(&loaded, &sample()?);
    assert_eq!(merged.next_deed_id(), DeedId(3));
    assert!(!merged.deeds.iter().any(|deed| deed.id == DeedId(2)));
    Ok(())
}

#[test]
fn old_version_derives_issued_id_from_live_deeds() -> TestResult {
    let mut old: serde_json::Value = serde_json::from_str(&to_json(&sample()?)?)?;
    old["schema_version"] = 5.into();
    old.as_object_mut()
        .ok_or("file is not an object")?
        .remove("last_deed_id");
    old.as_object_mut()
        .ok_or("file is not an object")?
        .remove("deleted_deeds");
    assert_eq!(from_json(&old.to_string())?.next_deed_id(), DeedId(3));
    Ok(())
}

#[test]
fn hijri_offset_history_is_saved_and_old_settings_migrate() -> TestResult {
    let mut data = sample()?;
    let change_day = WerdyDay::new(2026, 10, 27)?;
    let past_day = change_day.previous()?;
    data.settings
        .calendar
        .change_offset(change_day, HijriOffset::new(-1)?)?;
    let saved = to_json(&data)?;
    let loaded = from_json(&saved)?;
    assert_eq!(
        loaded.settings.calendar.offset_on(past_day),
        HijriOffset::PLUS_ONE
    );
    assert_eq!(loaded.settings.calendar.offset_on(change_day).days(), -1);
    assert_eq!(loaded.settings.calendar, data.settings.calendar);

    let mut old: serde_json::Value = serde_json::from_str(&saved)?;
    old["schema_version"] = 3.into();
    old["settings"]
        .as_object_mut()
        .ok_or("settings is not an object")?
        .remove("calendar");
    old["settings"]["hijri_offset"] = (-1).into();
    let loaded = from_json(&old.to_string())?;
    assert_eq!(loaded.settings.calendar.offset_on(past_day).days(), -1);
    assert_eq!(loaded.settings.calendar.offset_on(change_day).days(), -1);

    old["settings"]
        .as_object_mut()
        .ok_or("settings is not an object")?
        .remove("hijri_offset");
    let loaded = from_json(&old.to_string())?;
    assert_eq!(
        loaded.settings.calendar.offset_on(past_day),
        HijriOffset::PLUS_ONE
    );
    Ok(())
}

#[test]
fn version_two_decimal_marks_still_load() -> TestResult {
    let mut old: serde_json::Value = serde_json::from_str(&to_json(&sample()?)?)?;
    old["schema_version"] = 2.into();
    old["settings"]
        .as_object_mut()
        .ok_or("settings is not an object")?
        .remove("calendar");
    let marks = old["marks"].as_array_mut().ok_or("marks is not a list")?;
    let partial = marks
        .iter_mut()
        .find_map(|record| record["entry"]["mark"].get_mut("partial"))
        .ok_or("no partial mark")?;
    *partial = serde_json::json!(0.5);
    let loaded = from_json(&old.to_string())?;
    assert_eq!(loaded.marks, sample()?.marks);
    assert_eq!(
        loaded
            .settings
            .calendar
            .offset_on(WerdyDay::new(2026, 10, 1)?),
        HijriOffset::PLUS_ONE
    );
    Ok(())
}
