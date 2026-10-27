use std::collections::{BTreeMap, BTreeSet};

use werdy_core::{
    CalendarSettings, CategoryKey, Deed, DeedId, DeedRule, Error, Fraction, HijriRange, Mark,
    MarkEntry, OwnerRecords, Period, Schedule, Score, ScoreValue, SlotKey, Target, Weekday,
    WerdyDay, score as score_records,
};

type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;

#[allow(clippy::expect_used)]
fn score(deeds: &[Deed], marks: &Marks, days: &[WerdyDay], today: WerdyDay) -> Score {
    score_records(
        &OwnerRecords {
            deeds,
            marks,
            day_notes: &BTreeMap::new(),
            calendar: CalendarSettings::default(),
        },
        days,
        today,
    )
    .expect("test score fits")
}

fn deed(id: u64, schedule: Schedule, created_on: WerdyDay) -> Deed {
    Deed::new(
        DeedId(id),
        created_on,
        DeedRule {
            schedule,
            target: None,
            category: CategoryKey::new("other"),
            slot: SlotKey::new("any_time"),
        },
    )
}

fn targeted_deed(id: u64, target: u32, day: WerdyDay) -> Result<Deed, Error> {
    Ok(Deed::new(
        DeedId(id),
        day,
        DeedRule {
            schedule: Schedule::EveryDay,
            target: Some(Target::new(target, "pages")?),
            category: CategoryKey::new("other"),
            slot: SlotKey::new("any_time"),
        },
    ))
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

    assert_eq!(result.earned.as_f64(), 0.0);
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
    assert_eq!(result.earned.as_f64(), 0.5);
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
            earned: ScoreValue::ZERO,
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

    for (numerator, denominator, expected) in [
        (4261, 10_000, 43),
        (425, 1000, 43),
        (4249, 10_000, 42),
        (424_999_999_999, 1_000_000_000_000, 42),
    ] {
        marks.insert(
            (DeedId(1), day),
            MarkEntry::new(
                Mark::Partial(Fraction::from_ratio(numerator, denominator)?),
                day,
            ),
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
fn many_partials_summing_to_a_half_round_up() -> Result<(), Error> {
    let first = WerdyDay::new(2026, 10, 1)?;
    let days = (1..=10)
        .map(|number| WerdyDay::new(2026, 10, number))
        .collect::<Result<Vec<_>, _>>()?;
    let deeds: Vec<_> = (1..=16)
        .map(|id| deed(id, Schedule::EveryDay, first))
        .collect();
    let mut marks = Marks::new();
    for (index, (day, deed)) in days
        .iter()
        .flat_map(|day| deeds.iter().map(move |deed| (day, deed)))
        .enumerate()
    {
        let numerator = if index < 104 { 6 } else { 1 };
        marks.insert(
            (deed.id, *day),
            MarkEntry::new(Mark::Partial(Fraction::from_ratio(numerator, 10)?), *day),
        );
    }

    let result = score(&deeds, &marks, &days, WerdyDay::new(2026, 10, 11)?);
    assert_eq!(result.due, 160);
    assert_eq!(result.earned.numerator(), 68);
    assert_eq!(result.earned.denominator(), 1);
    assert_eq!(result.percent, Some(43));
    Ok(())
}

#[test]
fn exact_values_have_no_float_drift_over_a_year() -> Result<(), Error> {
    let today = WerdyDay::new(2027, 1, 1)?;
    let days = Period::Year.days(today, 0);
    let first = days[0];
    let count_deed = Deed::new(
        DeedId(1),
        first,
        DeedRule {
            schedule: Schedule::EveryDay,
            target: Some(Target::new(5, "pages")?),
            category: CategoryKey::new("other"),
            slot: SlotKey::new("any_time"),
        },
    );
    let partial_deed = deed(2, Schedule::EveryDay, first);
    let mut marks = Marks::new();
    for day in &days {
        marks.insert((count_deed.id, *day), MarkEntry::new(Mark::Count(3), *day));
        marks.insert(
            (partial_deed.id, *day),
            MarkEntry::new(Mark::Partial(Fraction::from_ratio(1, 10)?), *day),
        );
    }

    let result = score(&[count_deed, partial_deed], &marks, &days, today);
    assert_eq!(result.due, 730);
    assert_eq!(result.earned.numerator(), 511);
    assert_eq!(result.earned.denominator(), 2);
    assert_eq!(result.percent, Some(35));
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
    assert_eq!(result.earned.as_f64(), 3.0);
    assert_eq!(result.due, 4);
    assert_eq!(result.percent, Some(75));
    Ok(())
}

#[test]
fn werd_week_rebuilt_with_werdy_rules() -> Result<(), Error> {
    let first = WerdyDay::new(2026, 10, 21)?;
    let deeds: Vec<_> = (1..=5)
        .map(|id| deed(id, Schedule::EveryDay, first))
        .chain([
            deed(6, weekdays(&[Weekday::Friday]), first),
            deed(7, weekdays(&[Weekday::Thursday]), first),
            deed(8, weekdays(&[Weekday::Monday, Weekday::Thursday]), first),
            deed(9, Schedule::Hijri(HijriRange::ramadan()), first),
            deed(10, Schedule::Hijri(HijriRange::last_ten_nights()), first),
        ])
        .collect();
    let days: Vec<_> = (21..=27)
        .map(|day| WerdyDay::new(2026, 10, day))
        .collect::<Result<_, _>>()?;
    let today = WerdyDay::new(2026, 10, 27)?;
    let mut marks = Marks::new();
    for day in days.iter().filter(|day| **day < today) {
        for id in 1..=4 {
            marks.insert((DeedId(id), *day), MarkEntry::new(Mark::Done, *day));
        }
        marks.insert(
            (DeedId(5), *day),
            MarkEntry::new(Mark::partial_without_target(), *day),
        );
    }
    let friday = WerdyDay::new(2026, 10, 23)?;
    marks.insert((DeedId(6), friday), MarkEntry::new(Mark::Done, friday));
    let thursday = WerdyDay::new(2026, 10, 22)?;
    marks.insert((DeedId(7), thursday), MarkEntry::new(Mark::Done, thursday));
    for day in [22, 26] {
        let marked_day = WerdyDay::new(2026, 10, day)?;
        marks.insert(
            (DeedId(8), marked_day),
            MarkEntry::new(Mark::partial_without_target(), marked_day),
        );
    }
    marks.insert((DeedId(1), today), MarkEntry::new(Mark::Done, today));

    assert_eq!(score(&deeds[8..], &marks, &days, today).due, 0);
    assert_eq!(score(&deeds[5..6], &marks, &days, today).due, 1);
    assert_eq!(score(&deeds[6..7], &marks, &days, today).due, 1);
    assert_eq!(score(&deeds[7..8], &marks, &days, today).due, 2);

    // Six finished days: 5 daily deeds give 30 due. Friday gives 1,
    // Thursday gives 1, fasting gives 2, and both Hijri deeds give 0.
    // Earned: 24 daily Done + 6 daily halves worth 3 + 1 Friday
    // + 1 Thursday + 2 fasting halves worth 1 = 30.
    // Due = 34. 100 * 30 / 34 rounds to 88.
    let result = score(&deeds, &marks, &days, today);
    assert_eq!(result.earned.as_f64(), 30.0);
    assert_eq!(result.due, 34);
    assert_eq!(result.percent, Some(88));
    Ok(())
}

#[test]
fn invalid_date_is_an_error() {
    assert_eq!(WerdyDay::new(2025, 2, 29), Err(Error::InvalidDate));
    assert_eq!(WerdyDay::new(2026, 13, 1), Err(Error::InvalidDate));
    assert!(WerdyDay::new(2024, 2, 29).is_ok());
}

#[test]
fn fraction_input_is_checked_and_converted_once() -> Result<(), Error> {
    assert_eq!(Fraction::new(1.01), Err(Error::FractionOutOfRange));
    assert_eq!(Fraction::new(f64::NAN), Err(Error::FractionOutOfRange));
    assert_eq!(Fraction::from_ratio(1, 0), Err(Error::FractionOutOfRange));
    assert!(Fraction::new(0.0).is_ok());
    assert!(Fraction::new(1.0).is_ok());
    assert_eq!(Fraction::new(0.425)?, Fraction::from_ratio(17, 40)?);
    Ok(())
}

#[test]
fn thirds_sum_to_one_deed_exactly() -> Result<(), Error> {
    let day = WerdyDay::new(2026, 10, 23)?;
    let deeds: Vec<_> = (1..=3)
        .map(|id| targeted_deed(id, 3, day))
        .collect::<Result<_, _>>()?;
    let marks = deeds
        .iter()
        .map(|deed| ((deed.id, day), MarkEntry::new(Mark::Count(1), day)))
        .collect();
    let result = score(&deeds, &marks, &[day], day.next()?);
    assert_eq!(result.earned.as_f64(), 1.0);
    assert_eq!(result.earned.numerator(), 1);
    assert_eq!(result.earned.denominator(), 1);
    Ok(())
}

#[test]
fn percent_built_from_thirds_rounds_half_up() -> Result<(), Error> {
    let day = WerdyDay::new(2026, 10, 23)?;
    let deeds: Vec<_> = (1..=20)
        .map(|id| targeted_deed(id, 3, day))
        .collect::<Result<_, _>>()?;
    let marks = deeds
        .iter()
        .map(|deed| {
            let mark = match deed.id.0 {
                1..=7 => Mark::Done,
                8..=10 => Mark::Count(1),
                11 => Mark::partial_without_target(),
                _ => Mark::Missed,
            };
            ((deed.id, day), MarkEntry::new(mark, day))
        })
        .collect();
    let result = score(&deeds, &marks, &[day], day.next()?);
    assert_eq!(result.earned.numerator(), 17);
    assert_eq!(result.earned.denominator(), 2);
    assert_eq!(result.percent, Some(43));
    Ok(())
}

#[test]
fn mixed_targets_sum_exactly() -> Result<(), Error> {
    let day = WerdyDay::new(2026, 10, 23)?;
    let counts = [(3, 1), (7, 2), (100, 38)];
    let deeds: Vec<_> = counts
        .iter()
        .enumerate()
        .map(|(index, (target, _))| targeted_deed(index as u64 + 1, *target, day))
        .collect::<Result<_, _>>()?;
    let marks = deeds
        .iter()
        .zip(counts)
        .map(|(deed, (_, count))| ((deed.id, day), MarkEntry::new(Mark::Count(count), day)))
        .collect();
    let result = score(&deeds, &marks, &[day], day.next()?);
    assert_eq!(result.earned.numerator(), 1049);
    assert_eq!(result.earned.denominator(), 1050);
    Ok(())
}

#[test]
fn score_returns_error_when_exact_sum_exceeds_u128() -> Result<(), Error> {
    let day = WerdyDay::new(2026, 10, 23)?;
    let targets = [
        1_000_000_007,
        1_000_000_009,
        1_000_000_021,
        1_000_000_033,
        1_000_000_087,
    ];
    let deeds = targets
        .iter()
        .enumerate()
        .map(|(index, target)| targeted_deed(index as u64 + 1, *target, day))
        .collect::<Result<Vec<_>, _>>()?;
    let marks = deeds
        .iter()
        .map(|deed| ((deed.id, day), MarkEntry::new(Mark::Count(1), day)))
        .collect();
    let records = OwnerRecords {
        deeds: &deeds,
        marks: &marks,
        day_notes: &BTreeMap::new(),
        calendar: CalendarSettings::default(),
    };
    assert_eq!(
        score_records(&records, &[day], day.next()?),
        Err(Error::ScoreOverflow)
    );
    Ok(())
}

#[test]
fn large_matching_partials_reduce_to_a_fitting_score() -> Result<(), Error> {
    let day = WerdyDay::new(2027, 2, 7)?;
    let denominator = u128::MAX - 1;
    let partial = Mark::Partial(Fraction::from_ratio(denominator - 1, denominator)?);
    let deeds = [
        deed(1, Schedule::EveryDay, day),
        deed(2, Schedule::EveryDay, day),
    ];
    let marks = Marks::from([
        ((DeedId(1), day), MarkEntry::new(partial, day)),
        ((DeedId(2), day), MarkEntry::new(partial, day)),
    ]);
    let result = score_records(
        &OwnerRecords {
            deeds: &deeds,
            marks: &marks,
            day_notes: &BTreeMap::new(),
            calendar: CalendarSettings::default(),
        },
        &[day],
        day.next()?,
    )?;
    assert_eq!(result.earned.numerator(), denominator - 1);
    assert_eq!(result.earned.denominator(), denominator / 2);
    assert_eq!(result.due, 2);
    Ok(())
}
