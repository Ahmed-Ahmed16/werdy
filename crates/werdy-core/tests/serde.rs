//! Stored data is checked on load, so bad JSON can never build an invalid value.

#![cfg(feature = "serde")]

use werdy_core::{
    CalendarSettings, CategoryKey, DayNote, Deed, DeedId, DeedRule, Error, ExcuseCredit, Fraction,
    HijriOffset, HijriRange, Mark, MarkEntry, Schedule, ScoreValue, SlotKey, Target, WerdyDay,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn rule(target: Option<Target>) -> DeedRule {
    DeedRule {
        schedule: Schedule::EveryDay,
        target,
        category: CategoryKey::new("quran"),
        slot: SlotKey::new("morning"),
    }
}

#[test]
fn a_deed_with_rule_history_round_trips() -> TestResult {
    let created = WerdyDay::new(2026, 10, 1)?;
    let mut deed = Deed::new(DeedId(7), created, rule(None));
    deed.change_rule(
        WerdyDay::new(2026, 10, 9)?,
        rule(Some(Target::new(5, "page")?)),
    )?;
    deed.archive(WerdyDay::new(2026, 10, 20)?)?;

    let json = serde_json::to_string(&deed)?;

    assert_eq!(serde_json::from_str::<Deed>(&json)?, deed);
    Ok(())
}

#[test]
fn marks_and_notes_round_trip() -> TestResult {
    let day = WerdyDay::new(2026, 10, 27)?;
    let marks = [
        MarkEntry::new(Mark::Done, day),
        MarkEntry::new(Mark::partial_without_target(), day),
        MarkEntry::new(Mark::Missed, day),
        MarkEntry::new(Mark::Count(3), day),
    ];
    let note = DayNote::new("travelling", ExcuseCredit::Half, day)?;

    let json = serde_json::to_string(&(marks, &note))?;

    assert_eq!(
        serde_json::from_str::<([MarkEntry; 4], DayNote)>(&json)?,
        (marks, note)
    );
    Ok(())
}

#[test]
fn a_fraction_above_one_is_rejected() {
    let json = r#"{"mark":{"partial":1.5},"entered_on":20000}"#;

    assert!(serde_json::from_str::<MarkEntry>(json).is_err());
}

#[test]
fn a_zero_target_is_rejected() {
    assert!(serde_json::from_str::<Target>(r#"{"amount":0,"unit":"page"}"#).is_err());
}

#[test]
fn an_empty_note_without_credit_is_rejected() {
    let json = r#"{"text":"","credit":"none","entered_on":20000}"#;

    assert!(serde_json::from_str::<DayNote>(json).is_err());
}

#[test]
fn a_deed_whose_rules_go_back_in_time_is_rejected() -> TestResult {
    let created = WerdyDay::new(2026, 10, 10)?;
    let deed = Deed::new(DeedId(1), created, rule(None));
    let mut value = serde_json::to_value(&deed)?;
    let mut early = serde_json::to_value(&deed)?["first_rule"].clone();
    early["from_day"] = serde_json::to_value(WerdyDay::new(2026, 10, 5)?)?;
    value["later_rules"] = serde_json::Value::Array(vec![early]);

    assert!(serde_json::from_value::<Deed>(value).is_err());
    Ok(())
}

#[test]
fn an_archive_before_creation_is_rejected() -> TestResult {
    let deed = Deed::new(DeedId(1), WerdyDay::new(2026, 10, 10)?, rule(None));
    let mut value = serde_json::to_value(&deed)?;
    value["archived_from"] = serde_json::to_value(WerdyDay::new(2026, 10, 9)?)?;

    assert!(serde_json::from_value::<Deed>(value).is_err());
    Ok(())
}

#[test]
fn errors_have_readable_text() {
    assert_eq!(
        Error::FractionOutOfRange.to_string(),
        "fraction is not from zero to one"
    );
}

#[test]
fn exact_fraction_and_score_round_trip_without_float_conversion() -> TestResult {
    let fraction = Fraction::from_ratio(1, 3)?;
    let score = ScoreValue::from_ratio(7, 3)?;
    let json = serde_json::to_string(&(fraction, score))?;
    assert!(json.contains("\"numerator\":1,\"denominator\":3"));
    assert_eq!(
        serde_json::from_str::<(Fraction, ScoreValue)>(&json)?,
        (fraction, score)
    );
    assert_eq!(serde_json::from_str::<Fraction>("0.5")?, Fraction::HALF);
    assert!(serde_json::from_str::<Fraction>(r#"{"numerator":3,"denominator":2}"#).is_err());
    assert!(serde_json::from_str::<ScoreValue>(r#"{"numerator":1,"denominator":0}"#).is_err());
    Ok(())
}

#[test]
fn hijri_settings_and_range_are_checked_on_load() -> TestResult {
    let mut calendar = CalendarSettings::new(HijriOffset::PLUS_ONE);
    calendar.change_offset(WerdyDay::new(2026, 10, 27)?, HijriOffset::new(-1)?)?;
    let range = HijriRange::ramadan();
    let json = serde_json::to_string(&(&calendar, range))?;
    assert_eq!(
        serde_json::from_str::<(CalendarSettings, HijriRange)>(&json)?,
        (calendar, range)
    );
    assert!(serde_json::from_str::<HijriOffset>("2").is_err());
    assert!(
        serde_json::from_str::<CalendarSettings>(
            r#"{"first_offset":1,"later_offsets":[[20724,-1],[20724,0]]}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<HijriRange>(r#"{"month":13,"first_day":1,"last_day":10}"#).is_err()
    );
    Ok(())
}
