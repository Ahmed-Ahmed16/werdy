use std::collections::{BTreeMap, BTreeSet};

use werdy_core::{
    Deed, DeedId, DeedRule, Error, Mark, Schedule, Weekday, WerdyDay, delete_deed, score,
};

fn day(number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, 10, number)
}

fn daily() -> DeedRule {
    DeedRule {
        schedule: Schedule::EveryDay,
    }
}

#[test]
fn score_before_schedule_edit_is_unchanged_after_it() -> Result<(), Error> {
    let friday = day(23)?;
    let saturday = day(24)?;
    let mut deed = Deed::new(DeedId(1), friday, daily());
    let marks = BTreeMap::from([((deed.id, friday), Mark::Done)]);
    let before = score(&[deed.clone()], &marks, &[friday]);

    deed.change_rule(
        saturday,
        DeedRule {
            schedule: Schedule::Weekdays(BTreeSet::from([Weekday::Saturday])),
        },
    )?;

    assert_eq!(score(&[deed.clone()], &marks, &[friday]), before);
    assert_eq!(score(&[deed], &marks, &[friday, saturday]).due, 2);
    Ok(())
}

#[test]
fn deed_added_mid_week_is_due_from_that_day_only() -> Result<(), Error> {
    let deed = Deed::new(DeedId(1), day(24)?, daily());
    let days = (21..=27).map(day).collect::<Result<Vec<_>, _>>()?;

    assert_eq!(score(&[deed], &BTreeMap::new(), &days).due, 4);
    Ok(())
}

#[test]
fn archived_deed_counts_for_days_before_archiving() -> Result<(), Error> {
    let mut deed = Deed::new(DeedId(1), day(21)?, daily());
    let marks = BTreeMap::from([
        ((deed.id, day(22)?), Mark::Done),
        ((deed.id, day(24)?), Mark::Done),
    ]);
    deed.archive(day(24)?)?;

    let result = score(
        &[deed],
        &marks,
        &[day(21)?, day(22)?, day(23)?, day(24)?, day(25)?],
    );
    assert_eq!(result.due, 3);
    assert_eq!(result.earned, 1.0);
    Ok(())
}

#[test]
fn archived_deed_is_not_due_from_archive_day() -> Result<(), Error> {
    let mut deed = Deed::new(DeedId(1), day(21)?, daily());
    deed.archive(day(24)?)?;

    assert!(deed.is_due_on(day(23)?));
    assert!(!deed.is_due_on(day(24)?));
    assert!(!deed.is_due_on(day(25)?));
    Ok(())
}

#[test]
fn rule_change_before_creation_is_an_error() -> Result<(), Error> {
    let mut deed = Deed::new(DeedId(1), day(24)?, daily());

    assert_eq!(
        deed.change_rule(day(23)?, daily()),
        Err(Error::RuleBeforeCreation)
    );
    assert_eq!(deed.rule_on(day(23)?), None);
    assert!(!deed.is_due_on(day(23)?));
    Ok(())
}

#[test]
fn rule_change_before_latest_version_is_an_error() -> Result<(), Error> {
    let mut deed = Deed::new(DeedId(1), day(21)?, daily());
    deed.change_rule(
        day(24)?,
        DeedRule {
            schedule: Schedule::Weekdays(BTreeSet::new()),
        },
    )?;

    assert_eq!(
        deed.change_rule(day(23)?, daily()),
        Err(Error::RuleBeforeLatest)
    );
    assert!(!deed.is_due_on(day(24)?));
    Ok(())
}

#[test]
fn same_day_rule_change_replaces_latest_rule() -> Result<(), Error> {
    let mut deed = Deed::new(DeedId(1), day(21)?, daily());
    deed.change_rule(
        day(24)?,
        DeedRule {
            schedule: Schedule::Weekdays(BTreeSet::new()),
        },
    )?;
    deed.change_rule(day(24)?, daily())?;

    assert_eq!(deed.rule_on(day(24)?), Some(&daily()));
    assert!(deed.is_due_on(day(24)?));
    Ok(())
}

#[test]
fn archive_before_creation_is_an_error() -> Result<(), Error> {
    let mut deed = Deed::new(DeedId(1), day(24)?, daily());

    assert_eq!(deed.archive(day(23)?), Err(Error::ArchiveBeforeCreation));
    assert_eq!(deed.archived_from(), None);
    Ok(())
}

#[test]
fn archiving_twice_is_an_error() -> Result<(), Error> {
    let mut deed = Deed::new(DeedId(1), day(24)?, daily());
    deed.archive(day(25)?)?;
    assert_eq!(deed.archive(day(26)?), Err(Error::AlreadyArchived));
    assert_eq!(deed.archived_from(), Some(day(25)?));
    Ok(())
}

#[test]
fn deleting_a_deed_removes_its_marks() -> Result<(), Error> {
    let first = Deed::new(DeedId(1), day(21)?, daily());
    let second = Deed::new(DeedId(2), day(21)?, daily());
    let mut deeds = vec![first, second.clone()];
    let mut marks = BTreeMap::from([
        ((DeedId(1), day(21)?), Mark::Done),
        ((DeedId(1), day(22)?), Mark::Missed),
        ((DeedId(2), day(21)?), Mark::Done),
    ]);

    delete_deed(&mut deeds, &mut marks, DeedId(1));

    assert_eq!(deeds, vec![second]);
    assert_eq!(marks, BTreeMap::from([((DeedId(2), day(21)?), Mark::Done)]));
    Ok(())
}
