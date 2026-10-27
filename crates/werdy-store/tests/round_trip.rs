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
        day_notes,
        settings: Settings {
            language: "ar".to_owned(),
            theme: Theme::Dark,
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
    let json = to_json(&sample()?)?.replace("\"partial\":0.5", "\"partial\":7.0");

    assert!(matches!(from_json(&json), Err(StoreError::Corrupt(_))));
    Ok(())
}

#[test]
fn a_newer_schema_version_is_refused() -> TestResult {
    let json = to_json(&sample()?)?.replace("\"schema_version\":1", "\"schema_version\":99");

    assert_eq!(from_json(&json), Err(StoreError::UnsupportedVersion(99)));
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
fn the_next_deed_id_follows_the_highest_id() -> TestResult {
    assert_eq!(sample()?.next_deed_id(), DeedId(3));
    assert_eq!(OwnerData::default().next_deed_id(), DeedId(1));
    Ok(())
}
