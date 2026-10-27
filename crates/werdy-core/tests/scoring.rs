use std::collections::{BTreeMap, BTreeSet};

use werdy_core::{
    Deed, DeedId, DeedRule, Error, Fraction, Mark, MarkEntry, Schedule, Score, Weekday, WerdyDay,
    score,
};

type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;

fn deed(id: u64, schedule: Schedule, created_on: WerdyDay) -> Deed {
    Deed::new(
        DeedId(id),
        created_on,
        DeedRule {
            schedule,
            target: None,
        },
    )
}

fn weekdays(days: &[Weekday]) -> Schedule {
    Schedule::Weekdays(days.iter().copied().collect::<BTreeSet<_>>())
}

#[test]
fn friday_deed_is_due_only_on_fridays() -> Result<(), Error> {
    let friday = WerdyDay::new(2026, 10, 23)?;
    let saturday = friday.next()?;
    assert_eq!(friday.weekday(), Weekday::Friday);
    assert_eq!(saturday.weekday(), Weekday::Saturday);
    let deeds = [deed(1, weekdays(&[Weekday::Friday]), friday)];
    let marks = Marks::new();

    assert_eq!(
        score(&deeds, &marks, &[friday, saturday], saturday.next()?).due,
        1
    );
    Ok(())
}

#[test]
fn every_day_deed_is_due_each_day() -> Result<(), Error> {
    let first = WerdyDay::new(2026, 10, 23)?;
    let deeds = [deed(1, Schedule::EveryDay, first)];

    assert_eq!(
        score(
            &deeds,
            &Marks::new(),
            &[first, first.next()?],
            first.next()?.next()?
        )
        .due,
        2
    );
    Ok(())
}

#[test]
fn unmarked_due_deed_counts_zero() -> Result<(), Error> {
    let day = WerdyDay::new(2026, 10, 23)?;
    let result = score(
        &[deed(1, Schedule::EveryDay, day)],
        &Marks::new(),
        &[day],
        day.next()?,
    );

    assert_eq!(result.earned, 0.0);
    assert_eq!(result.due, 1);
    assert_eq!(result.percent, Some(0));
    Ok(())
}

#[test]
fn partial_mark_counts_its_fraction() -> Result<(), Error> {
    let day = WerdyDay::new(2026, 10, 23)?;
    let mut marks = Marks::new();
    marks.insert(
        (DeedId(1), day),
        MarkEntry::new(Mark::partial_without_target(), day),
    );

    let result = score(
        &[deed(1, Schedule::EveryDay, day)],
        &marks,
        &[day],
        day.next()?,
    );
    assert_eq!(result.earned, 0.5);
    assert_eq!(result.due, 1);
    assert_eq!(result.percent, Some(50));
    Ok(())
}

#[test]
fn nothing_due_gives_no_percent() -> Result<(), Error> {
    let day = WerdyDay::new(2026, 10, 24)?;
    let result = score(
        &[deed(1, weekdays(&[Weekday::Friday]), day)],
        &Marks::new(),
        &[day],
        day.next()?,
    );

    assert_eq!(
        result,
        Score {
            earned: 0.0,
            due: 0,
            percent: None,
        }
    );
    Ok(())
}

#[test]
fn percent_rounds_never_truncates() -> Result<(), Error> {
    let day = WerdyDay::new(2026, 10, 23)?;
    let deeds = [deed(1, Schedule::EveryDay, day)];
    let mut marks = Marks::new();

    for (fraction, expected) in [
        (0.4261, 43),
        (0.425, 43),
        (0.4249, 42),
        (0.424_999_999_999, 42),
    ] {
        marks.insert(
            (DeedId(1), day),
            MarkEntry::new(Mark::Partial(Fraction::new(fraction)?), day),
        );
        assert_eq!(
            score(&deeds, &marks, &[day], day.next()?).percent,
            Some(expected)
        );
    }

    let eight_deeds: Vec<_> = (1..=8)
        .map(|id| deed(id, Schedule::EveryDay, day))
        .collect();
    marks.insert((DeedId(1), day), MarkEntry::new(Mark::Done, day));
    assert_eq!(
        score(&eight_deeds, &marks, &[day], day.next()?).percent,
        Some(13)
    );
    Ok(())
}

#[test]
fn thursday_lesson_month() -> Result<(), Error> {
    let lesson = deed(
        1,
        weekdays(&[Weekday::Thursday]),
        WerdyDay::new(2026, 10, 1)?,
    );
    let days: Vec<_> = (1..=27)
        .map(|day| WerdyDay::new(2026, 10, day))
        .collect::<Result<_, _>>()?;
    let mut marks = Marks::new();
    for day in [1, 8, 15] {
        let marked_day = WerdyDay::new(2026, 10, day)?;
        marks.insert(
            (lesson.id, marked_day),
            MarkEntry::new(Mark::Done, marked_day),
        );
    }

    let result = score(&[lesson], &marks, &days, WerdyDay::new(2026, 10, 28)?);
    assert_eq!(result.earned, 3.0);
    assert_eq!(result.due, 4);
    assert_eq!(result.percent, Some(75));
    Ok(())
}

#[test]
fn werd_week_counts_only_due_days() -> Result<(), Error> {
    let first = WerdyDay::new(2026, 10, 21)?;
    let deeds: Vec<_> = (1..=5)
        .map(|id| deed(id, Schedule::EveryDay, first))
        .chain([
            deed(6, weekdays(&[Weekday::Friday]), first),
            deed(7, weekdays(&[Weekday::Monday, Weekday::Thursday]), first),
        ])
        .collect();
    let days: Vec<_> = (21..=27)
        .map(|day| WerdyDay::new(2026, 10, day))
        .collect::<Result<_, _>>()?;
    let mut marks = Marks::new();
    for day in &days {
        for id in 1..=5 {
            marks.insert((DeedId(id), *day), MarkEntry::new(Mark::Done, *day));
        }
    }
    let friday = WerdyDay::new(2026, 10, 23)?;
    marks.insert((DeedId(6), friday), MarkEntry::new(Mark::Done, friday));
    for day in [22, 26] {
        let marked_day = WerdyDay::new(2026, 10, day)?;
        marks.insert(
            (DeedId(7), marked_day),
            MarkEntry::new(Mark::partial_without_target(), marked_day),
        );
    }

    // 35 daily + 1 Friday + 2 Monday and Thursday due; earned 35 + 1 + 0.5 + 0.5 = 37.
    let result = score(&deeds, &marks, &days, WerdyDay::new(2026, 10, 28)?);
    assert_eq!(result.earned, 37.0);
    assert_eq!(result.due, 38);
    assert_eq!(result.percent, Some(97));
    Ok(())
}

#[test]
fn invalid_date_is_an_error() {
    assert_eq!(WerdyDay::new(2025, 2, 29), Err(Error::InvalidDate));
    assert_eq!(WerdyDay::new(2026, 13, 1), Err(Error::InvalidDate));
    assert!(WerdyDay::new(2024, 2, 29).is_ok());
}

#[test]
fn fraction_above_one_is_an_error() {
    assert_eq!(Fraction::new(1.01), Err(Error::FractionOutOfRange));
    assert_eq!(Fraction::new(f64::NAN), Err(Error::FractionOutOfRange));
    assert!(Fraction::new(0.0).is_ok());
    assert!(Fraction::new(1.0).is_ok());
}
