use std::collections::BTreeMap;

use werdy_core::{
    CalendarSettings, CategoryKey, Deed, DeedId, DeedRule, Error, Fraction, Mark, MarkEntry,
    OwnerRecords, Schedule, Score, ScoreValue, TodaySummary, WerdyDay, score as score_records,
    today_summary,
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

fn day(number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, 10, number)
}

fn deed(id: u64, created_on: WerdyDay) -> Deed {
    Deed::new(
        DeedId(id),
        created_on,
        DeedRule {
            schedule: Schedule::EveryDay,
            target: None,
            category: CategoryKey::new("other"),
        },
    )
}

#[test]
fn today_returns_counts_and_no_percent() -> Result<(), Error> {
    let today = day(24)?;
    let mut deeds: Vec<_> = (1..=4).map(|id| deed(id, today)).collect();
    deeds.push(deed(5, today.next()?));
    let marks = Marks::from([
        ((DeedId(1), today), MarkEntry::new(Mark::Done, today)),
        (
            (DeedId(2), today),
            MarkEntry::new(Mark::partial_without_target(), today),
        ),
        ((DeedId(3), today), MarkEntry::new(Mark::Missed, today)),
    ]);

    assert_eq!(
        today_summary(&deeds, &marks, today, &CalendarSettings::default()),
        TodaySummary {
            done: 1,
            partial: 1,
            open: 1,
            missed: 1,
        }
    );
    Ok(())
}

#[test]
fn partial_marks_at_zero_and_one_use_exact_value() -> Result<(), Error> {
    let today = day(24)?;
    let deeds = [deed(1, today), deed(2, today)];
    let marks = Marks::from([
        (
            (DeedId(1), today),
            MarkEntry::new(Mark::Partial(Fraction::ONE), today),
        ),
        (
            (DeedId(2), today),
            MarkEntry::new(Mark::Partial(Fraction::ZERO), today),
        ),
    ]);
    assert_eq!(
        today_summary(&deeds, &marks, today, &CalendarSettings::default()),
        TodaySummary {
            done: 1,
            partial: 0,
            open: 0,
            missed: 1
        }
    );
    Ok(())
}

#[test]
fn open_deed_today_is_not_missed() -> Result<(), Error> {
    let today = day(24)?;
    let summary = today_summary(
        &[deed(1, today)],
        &Marks::new(),
        today,
        &CalendarSettings::default(),
    );

    assert_eq!(summary.open, 1);
    assert_eq!(summary.missed, 0);
    Ok(())
}

#[test]
fn unmarked_due_deed_on_finished_day_is_worth_zero() -> Result<(), Error> {
    let finished = day(23)?;
    let result = score(&[deed(1, finished)], &Marks::new(), &[finished], day(24)?);

    assert_eq!(
        result,
        Score {
            earned: ScoreValue::ZERO,
            due: 1,
            percent: Some(0),
        }
    );
    Ok(())
}

#[test]
fn mark_entered_two_days_late_has_no_flag() -> Result<(), Error> {
    let entry = MarkEntry::new(Mark::Done, day(25)?);

    assert!(!entry.is_late_for(day(23)?));
    Ok(())
}

#[test]
fn mark_entered_three_days_late_has_flag() -> Result<(), Error> {
    let entry = MarkEntry::new(Mark::Done, day(26)?);

    assert!(entry.is_late_for(day(23)?));
    Ok(())
}

#[test]
fn late_mark_counts_like_any_mark() -> Result<(), Error> {
    let finished = day(23)?;
    let marks = Marks::from([(
        (DeedId(1), finished),
        MarkEntry::new(Mark::partial_without_target(), day(26)?),
    )]);

    assert!(marks[&(DeedId(1), finished)].is_late_for(finished));
    assert_eq!(
        score(&[deed(1, finished)], &marks, &[finished], day(27)?).percent,
        Some(50)
    );
    Ok(())
}

#[test]
fn today_never_enters_a_period_score() -> Result<(), Error> {
    let finished = day(23)?;
    let today = day(24)?;
    let future = day(25)?;
    let marks = Marks::from([
        ((DeedId(1), finished), MarkEntry::new(Mark::Done, finished)),
        ((DeedId(1), today), MarkEntry::new(Mark::Done, today)),
        ((DeedId(1), future), MarkEntry::new(Mark::Done, future)),
    ]);
    let deeds = [deed(1, finished)];

    let finished_only = score(&deeds, &marks, &[finished], today);
    assert_eq!(
        score(&deeds, &marks, &[finished, today], today),
        finished_only
    );
    assert_eq!(
        score(&deeds, &marks, &[finished, today, future], today),
        finished_only
    );
    Ok(())
}
