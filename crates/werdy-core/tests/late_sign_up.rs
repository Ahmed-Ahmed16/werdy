//! Late sign up week (#170): a person may mark the 7 days before the day
//! they sign up. Days before that week never count, not even as Missed.

use std::collections::BTreeMap;

use werdy_core::{
    CalendarSettings, CategoryKey, Deed, DeedId, DeedRule, Error, LATE_SIGN_UP_DAYS, Mark,
    MarkEntry, OwnerRecords, Schedule, SlotKey, WerdyDay, first_markable_day, score,
};

fn november(day: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, 11, day)
}

fn daily() -> DeedRule {
    DeedRule {
        schedule: Schedule::EveryDay,
        target: None,
        category: CategoryKey::new("prayer"),
        slot: SlotKey::new("fajr"),
    }
}

#[test]
fn the_week_before_sign_up_is_markable() -> Result<(), Error> {
    assert_eq!(LATE_SIGN_UP_DAYS, 7_u8);
    assert_eq!(first_markable_day(november(26)?), november(19)?);
    assert_eq!(
        first_markable_day(november(3)?),
        WerdyDay::new(2026, 10, 27)?
    );
    Ok(())
}

#[test]
fn the_first_markable_day_stops_at_the_earliest_day() -> Result<(), Error> {
    let earliest = WerdyDay::from_unix_days(i64::from(i32::MIN))?;
    assert_eq!(first_markable_day(earliest), earliest);
    let near = WerdyDay::from_unix_days(i64::from(i32::MIN) + 3)?;
    assert_eq!(first_markable_day(near), earliest);
    Ok(())
}

/// A person signs up on 26 November. Setup deeds start on 19 November.
/// Marks in that week count; days before it are not due at all.
#[test]
fn days_before_the_late_sign_up_week_are_not_missed() -> Result<(), Error> {
    let sign_up = november(26)?;
    let deed = Deed::new(DeedId(1), first_markable_day(sign_up), daily());
    let marks = BTreeMap::from([(
        (deed.id, november(20)?),
        MarkEntry::new(Mark::Done, sign_up),
    )]);
    let records = OwnerRecords {
        deeds: std::slice::from_ref(&deed),
        marks: &marks,
        day_notes: &BTreeMap::new(),
        calendar: CalendarSettings::default(),
    };

    let before_week = [november(10)?, november(18)?];
    let before = score(&records, &before_week, sign_up)?;
    assert_eq!((before.due, before.percent), (0, None));

    let week = [november(19)?, november(20)?];
    let in_week = score(&records, &week, sign_up)?;
    assert_eq!((in_week.due, in_week.percent), (2, Some(50)));
    Ok(())
}
