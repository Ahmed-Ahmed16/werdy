use std::collections::BTreeMap;

use werdy_core::{
    CategoryKey, DayNote, Deed, DeedId, DeedRule, ExcuseCredit, Mark, MarkEntry, Period, Schedule,
    ScoreBands, SlotKey, Target, WerdyDay, build_report,
};
use werdy_store::{
    DeedName, MemoryStore, OwnerData, Settings, Store, StoreError, Theme, from_json, to_json,
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
        deeds: vec![fajr, quran],
        names: BTreeMap::from([
            (DeedId(1), DeedName::Key("starter-fajr-on-time".to_owned())),
            (DeedId(2), DeedName::Text("ورد القرآن".to_owned())),
        ]),
        marks,
        mark_revisions: BTreeMap::new(),
        cleared_marks: BTreeMap::new(),
        day_notes,
        settings: Settings {
            language: "ar".to_owned(),
            theme: Theme::Dark,
            hijri_offset: werdy_core::HijriOffset::PLUS_ONE,
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
    let json = to_json(&sample()?)?.replace("\"schema_version\":3", "\"schema_version\":99");

    assert_eq!(from_json(&json), Err(StoreError::UnsupportedVersion(99)));
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
fn hijri_offset_is_saved_and_old_settings_default_to_plus_one() -> TestResult {
    let mut data = sample()?;
    data.settings.hijri_offset = werdy_core::HijriOffset::new(-1)?;
    let saved = to_json(&data)?;
    assert_eq!(
        from_json(&saved)?.records().calendar.hijri_offset.days(),
        -1
    );

    let mut old: serde_json::Value = serde_json::from_str(&saved)?;
    old["settings"]
        .as_object_mut()
        .ok_or("settings is not an object")?
        .remove("hijri_offset");
    let loaded = from_json(&old.to_string())?;
    assert_eq!(
        loaded.records().calendar.hijri_offset,
        werdy_core::HijriOffset::PLUS_ONE
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
        .remove("hijri_offset");
    let marks = old["marks"].as_array_mut().ok_or("marks is not a list")?;
    let partial = marks
        .iter_mut()
        .find_map(|record| record["entry"]["mark"].get_mut("partial"))
        .ok_or("no partial mark")?;
    *partial = serde_json::json!(0.5);
    let loaded = from_json(&old.to_string())?;
    assert_eq!(loaded.marks, sample()?.marks);
    assert_eq!(
        loaded.settings.hijri_offset,
        werdy_core::HijriOffset::PLUS_ONE
    );
    Ok(())
}
