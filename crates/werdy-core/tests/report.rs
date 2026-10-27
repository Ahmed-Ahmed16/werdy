use std::collections::{BTreeMap, BTreeSet};

use werdy_core::{
    Band, CalendarSettings, CategoryKey, DayNote, Deed, DeedId, DeedRule, Error, ExcuseCredit,
    Fraction, Mark, MarkEntry, OwnerRecords, Period, ReportLead, Schedule, ScoreBands, SeriesEntry,
    SlotKey, Weekday, WerdyDay, build_report, report_lead,
};

fn day(month: u8, number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, month, number)
}

fn deed(id: u64, schedule: Schedule, category: &str, created_on: WerdyDay) -> Deed {
    Deed::new(
        DeedId(id),
        created_on,
        DeedRule {
            schedule,
            target: None,
            category: CategoryKey::new(category),
            slot: SlotKey::new("any_time"),
        },
    )
}

#[allow(clippy::expect_used)]
fn report(
    deeds: &[Deed],
    marks: &BTreeMap<(DeedId, WerdyDay), MarkEntry>,
    period: Period,
    steps_back: usize,
    today: WerdyDay,
    bands: ScoreBands,
) -> werdy_core::Report {
    build_report(
        &OwnerRecords {
            deeds,
            marks,
            day_notes: &BTreeMap::new(),
            calendar: CalendarSettings::default(),
        },
        period,
        steps_back,
        today,
        bands,
    )
    .expect("test report fits")
}

#[test]
fn week_report_covers_last_seven_finished_days() -> Result<(), Error> {
    let today = day(10, 27)?;
    let days = Period::Week.days(today, 0);
    assert_eq!(days.len(), 7);
    assert_eq!(days.first(), Some(&day(10, 20)?));
    assert_eq!(days.last(), Some(&day(10, 26)?));
    let result = report(
        &[],
        &BTreeMap::new(),
        Period::Week,
        0,
        today,
        ScoreBands::default(),
    );
    assert_eq!(result.days, days);
    assert_eq!(report_lead(&result), ReportLead::Empty);
    Ok(())
}

#[test]
fn stepping_back_one_week_returns_the_whole_previous_week() -> Result<(), Error> {
    let days = Period::Week.days(day(10, 27)?, 1);
    assert_eq!(days.len(), 7);
    assert_eq!(days.first(), Some(&day(10, 13)?));
    assert_eq!(days.last(), Some(&day(10, 19)?));
    Ok(())
}

#[test]
fn today_is_never_in_a_report() -> Result<(), Error> {
    let today = day(10, 27)?;
    let deeds = [deed(1, Schedule::EveryDay, "prayer", day(10, 24)?)];
    let marks = BTreeMap::from([((DeedId(1), today), MarkEntry::new(Mark::Done, today))]);
    let result = report(
        &deeds,
        &marks,
        Period::ThreeDays,
        0,
        today,
        ScoreBands::default(),
    );
    assert!(!result.days.contains(&today));
    assert_eq!(result.overall.due, 3);
    assert_eq!(result.overall.earned.as_f64(), 0.0);
    Ok(())
}

#[test]
fn short_period_has_a_day_series_long_period_has_month_buckets() -> Result<(), Error> {
    let today = day(10, 27)?;
    let short = report(
        &[],
        &BTreeMap::new(),
        Period::Month,
        0,
        today,
        ScoreBands::default(),
    );
    assert_eq!(short.series.len(), 30);
    assert!(
        short
            .series
            .iter()
            .all(|entry| matches!(entry, SeriesEntry::Day { band: None, .. }))
    );

    let long = report(
        &[],
        &BTreeMap::new(),
        Period::SixMonths,
        0,
        today,
        ScoreBands::default(),
    );
    assert!(long.series.len() >= 6);
    assert!(
        long.series
            .iter()
            .all(|entry| matches!(entry, SeriesEntry::Month { .. }))
    );
    assert!(matches!(
        long.series.first(),
        Some(SeriesEntry::Month {
            year: 2026,
            month: 4,
            ..
        })
    ));
    assert!(matches!(
        long.series.last(),
        Some(SeriesEntry::Month {
            year: 2026,
            month: 10,
            ..
        })
    ));
    Ok(())
}

#[test]
fn score_is_split_by_category() -> Result<(), Error> {
    let today = day(10, 27)?;
    let first = day(10, 24)?;
    let prayer = deed(1, Schedule::EveryDay, "prayer", first);
    let mut study = deed(2, Schedule::EveryDay, "study", first);
    study.change_rule(
        day(10, 26)?,
        DeedRule {
            schedule: Schedule::EveryDay,
            target: None,
            category: CategoryKey::new("work"),
            slot: SlotKey::new("any_time"),
        },
    )?;
    let marks = BTreeMap::from([
        ((DeedId(1), first), MarkEntry::new(Mark::Done, first)),
        ((DeedId(2), first), MarkEntry::new(Mark::Done, first)),
    ]);
    let result = report(
        &[prayer, study],
        &marks,
        Period::ThreeDays,
        0,
        today,
        ScoreBands::default(),
    );
    assert_eq!(result.by_category[&CategoryKey::new("prayer")].due, 3);
    assert_eq!(
        result.by_category[&CategoryKey::new("prayer")]
            .earned
            .as_f64(),
        1.0
    );
    assert_eq!(result.by_category[&CategoryKey::new("study")].due, 2);
    assert_eq!(
        result.by_category[&CategoryKey::new("study")]
            .earned
            .as_f64(),
        1.0
    );
    assert_eq!(result.by_category[&CategoryKey::new("work")].due, 1);
    assert_eq!(result.overall.due, 6);
    Ok(())
}

#[test]
fn what_did_i_miss_lists_worst_first() -> Result<(), Error> {
    let today = day(10, 27)?;
    let first = day(10, 24)?;
    let deeds = [
        deed(4, Schedule::EveryDay, "other", first),
        deed(3, Schedule::EveryDay, "other", first),
        deed(2, Schedule::EveryDay, "other", first),
        deed(1, Schedule::EveryDay, "other", first),
    ];
    let mut marks = BTreeMap::new();
    for number in 24..=26 {
        let marked_day = day(10, number)?;
        marks.insert(
            (DeedId(1), marked_day),
            MarkEntry::new(Mark::Done, marked_day),
        );
        marks.insert(
            (DeedId(2), marked_day),
            MarkEntry::new(Mark::Missed, marked_day),
        );
        marks.insert(
            (DeedId(3), marked_day),
            MarkEntry::new(Mark::partial_without_target(), marked_day),
        );
        marks.insert(
            (DeedId(4), marked_day),
            MarkEntry::new(Mark::partial_without_target(), marked_day),
        );
    }
    let result = report(
        &deeds,
        &marks,
        Period::ThreeDays,
        0,
        today,
        ScoreBands::default(),
    );
    assert_eq!(
        result
            .missed
            .iter()
            .map(|item| item.deed_id)
            .collect::<Vec<_>>(),
        vec![DeedId(2), DeedId(3), DeedId(4)]
    );
    assert_eq!(result.missed[0].days_not_done, 3);
    assert_eq!(result.missed[0].lost.as_f64(), 3.0);
    Ok(())
}

#[test]
fn per_deed_history_counts_due_days_only() -> Result<(), Error> {
    let today = day(10, 27)?;
    let first = Period::Month.days(today, 0)[0];
    let fridays = Schedule::Weekdays(BTreeSet::from([Weekday::Friday]));
    let deeds = [deed(1, fridays, "other", first)];
    let marks = BTreeMap::from([(
        (DeedId(1), day(10, 23)?),
        MarkEntry::new(Mark::Done, day(10, 23)?),
    )]);
    let result = report(
        &deeds,
        &marks,
        Period::Month,
        0,
        today,
        ScoreBands::default(),
    );
    let friday_count = result
        .days
        .iter()
        .filter(|day| day.weekday() == Weekday::Friday)
        .count();
    assert_eq!(result.per_deed[&DeedId(1)].due, friday_count);
    assert_eq!(result.per_deed[&DeedId(1)].earned.as_f64(), 1.0);
    Ok(())
}

#[test]
fn day_band_uses_the_given_bands() -> Result<(), Error> {
    let bands = ScoreBands::default();
    assert_eq!(bands.band(80), Band::Good);
    assert_eq!(bands.band(79), Band::Mid);
    assert_eq!(bands.band(50), Band::Mid);
    assert_eq!(bands.band(49), Band::Low);
    assert_eq!(ScoreBands { good: 90, mid: 60 }.band(80), Band::Mid);
    let today = day(10, 27)?;
    let finished = day(10, 26)?;
    let deeds: Vec<_> = (1..=5)
        .map(|id| deed(id, Schedule::EveryDay, "other", finished))
        .collect();
    let marks = (1..=4)
        .map(|id| ((DeedId(id), finished), MarkEntry::new(Mark::Done, finished)))
        .collect();
    let result = report(
        &deeds,
        &marks,
        Period::ThreeDays,
        0,
        today,
        ScoreBands { good: 90, mid: 60 },
    );
    assert!(matches!(
        result.series.last(),
        Some(SeriesEntry::Day {
            band: Some(Band::Mid),
            ..
        })
    ));
    Ok(())
}

#[test]
fn full_day_credit_removes_deed_from_missed_list() -> Result<(), Error> {
    let today = day(10, 27)?;
    let finished = day(10, 26)?;
    let deeds = [deed(1, Schedule::EveryDay, "other", finished)];
    let notes = BTreeMap::from([(finished, DayNote::new("Ill", ExcuseCredit::Full, finished)?)]);
    let result = build_report(
        &OwnerRecords {
            deeds: &deeds,
            marks: &BTreeMap::new(),
            day_notes: &notes,
            calendar: CalendarSettings::default(),
        },
        Period::ThreeDays,
        0,
        today,
        ScoreBands::default(),
    )?;
    assert_eq!(result.overall.earned.as_f64(), 1.0);
    assert_eq!(
        result.by_category[&CategoryKey::new("other")]
            .earned
            .as_f64(),
        1.0
    );
    assert!(result.missed.is_empty());
    assert_eq!(report_lead(&result), ReportLead::AllDone);
    assert!(result.noted_days.contains(&finished));
    assert_eq!(result.per_deed_days[&DeedId(1)][0].value, Fraction::ONE);
    Ok(())
}

#[test]
fn month_buckets_cross_year_and_leap_day() -> Result<(), Error> {
    assert_eq!(WerdyDay::new(2024, 2, 29)?.year_month(), (2024, 2));
    let today = WerdyDay::new(2025, 1, 2)?;
    let result = report(
        &[],
        &BTreeMap::new(),
        Period::SixMonths,
        0,
        today,
        ScoreBands::default(),
    );
    assert!(matches!(
        result.series.first(),
        Some(SeriesEntry::Month {
            year: 2024,
            month: 7,
            ..
        })
    ));
    assert!(matches!(
        result.series.last(),
        Some(SeriesEntry::Month {
            year: 2025,
            month: 1,
            ..
        })
    ));
    Ok(())
}

#[test]
fn report_lead_names_deed_with_greatest_lost_value() -> Result<(), Error> {
    let today = day(10, 27)?;
    let first = day(10, 24)?;
    let deeds = [
        deed(1, Schedule::EveryDay, "prayer", first),
        deed(2, Schedule::EveryDay, "prayer", first),
    ];
    let marks = BTreeMap::from([
        (
            (DeedId(1), day(10, 25)?),
            MarkEntry::new(Mark::Done, day(10, 25)?),
        ),
        (
            (DeedId(1), day(10, 26)?),
            MarkEntry::new(Mark::Done, day(10, 26)?),
        ),
        (
            (DeedId(2), day(10, 26)?),
            MarkEntry::new(Mark::Done, day(10, 26)?),
        ),
    ]);
    let missed = report(
        &deeds,
        &marks,
        Period::ThreeDays,
        0,
        today,
        ScoreBands::default(),
    );
    assert_eq!(report_lead(&missed), ReportLead::MostMissed(DeedId(2)));
    Ok(())
}

#[test]
fn deed_history_contains_due_days_only_and_lists_not_due_deeds() -> Result<(), Error> {
    let today = day(10, 27)?;
    let friday = day(10, 23)?;
    let deeds = [
        deed(
            1,
            Schedule::Weekdays(BTreeSet::from([Weekday::Friday])),
            "other",
            day(10, 20)?,
        ),
        deed(
            2,
            Schedule::Weekdays(BTreeSet::from([Weekday::Sunday])),
            "other",
            today,
        ),
    ];
    let marks = BTreeMap::from([((DeedId(1), friday), MarkEntry::new(Mark::Done, today))]);
    let result = report(
        &deeds,
        &marks,
        Period::Week,
        0,
        today,
        ScoreBands::default(),
    );
    assert_eq!(result.per_deed_days[&DeedId(1)].len(), 1);
    assert_eq!(result.per_deed_days[&DeedId(1)][0].day, friday);
    assert_eq!(result.per_deed_days[&DeedId(1)][0].value, Fraction::ONE);
    assert!(result.per_deed_days[&DeedId(1)][0].added_later);
    assert_eq!(result.not_due, vec![DeedId(2)]);
    Ok(())
}
