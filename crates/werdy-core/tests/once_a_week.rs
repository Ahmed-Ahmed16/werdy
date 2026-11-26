//! Once a week deeds (decision 0047). The week runs Saturday to Friday.
//! The week used here is Saturday 2026-10-24 to Friday 2026-10-30.

use std::collections::BTreeMap;

use werdy_core::{
    CalendarSettings, CategoryKey, DayNote, Deed, DeedId, DeedRule, Error, ExcuseCredit, Mark,
    MarkEntry, OwnerRecords, RowTag, Schedule, Score, SlotKey, TodaySummary, Weekday, WerdyDay,
    consistency, daily_sheet, finished_day_summary, score, today_summary,
};

type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;
type Notes = BTreeMap<WerdyDay, DayNote>;

fn day(month: u8, number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, month, number)
}

/// Saturday to Friday of the test week, in order.
fn week() -> Result<Vec<WerdyDay>, Error> {
    (24..=30).map(|number| day(10, number)).collect()
}

fn weekly_deed(id: u64, created_on: WerdyDay) -> Deed {
    Deed::new(
        DeedId(id),
        created_on,
        DeedRule {
            schedule: Schedule::OnceAWeek,
            target: None,
            category: CategoryKey::new("other"),
            slot: SlotKey::new("any_time"),
        },
    )
}

fn marked(marks: &mut Marks, id: u64, day: WerdyDay, mark: Mark) {
    marks.insert((DeedId(id), day), MarkEntry::new(mark, day));
}

#[allow(clippy::expect_used)]
fn score_days(deeds: &[Deed], marks: &Marks, notes: &Notes, days: &[WerdyDay]) -> Score {
    let records = OwnerRecords {
        deeds,
        marks,
        day_notes: notes,
        calendar: CalendarSettings::default(),
    };
    score(&records, days, day(11, 1).expect("valid day")).expect("test score fits")
}

#[test]
fn test_week_runs_saturday_to_friday() -> Result<(), Error> {
    let week = week()?;
    assert_eq!(week[0].weekday(), Weekday::Saturday);
    assert_eq!(week[6].weekday(), Weekday::Friday);
    Ok(())
}

#[test]
fn done_on_tuesday_is_due_only_on_tuesday() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 1)?)];
    let mut marks = Marks::new();
    let tuesday = day(10, 27)?;
    marked(&mut marks, 1, tuesday, Mark::Done);

    for other in week()?.into_iter().filter(|other| *other != tuesday) {
        assert_eq!(score_days(&deeds, &marks, &Notes::new(), &[other]).due, 0);
    }
    let whole_week = score_days(&deeds, &marks, &Notes::new(), &week()?);
    assert_eq!((whole_week.due, whole_week.percent), (1, Some(100)));
    Ok(())
}

#[test]
fn no_mark_all_week_is_due_on_friday_and_missed() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 1)?)];
    let marks = Marks::new();
    let friday = day(10, 30)?;

    for other in week()?.into_iter().filter(|other| *other != friday) {
        assert_eq!(score_days(&deeds, &marks, &Notes::new(), &[other]).due, 0);
    }
    let on_friday = score_days(&deeds, &marks, &Notes::new(), &[friday]);
    assert_eq!((on_friday.due, on_friday.percent), (1, Some(0)));
    Ok(())
}

#[test]
fn missed_on_monday_is_due_on_monday_and_not_again_that_week() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 1)?)];
    let mut marks = Marks::new();
    let monday = day(10, 26)?;
    marked(&mut marks, 1, monday, Mark::Missed);

    let on_monday = score_days(&deeds, &marks, &Notes::new(), &[monday]);
    assert_eq!((on_monday.due, on_monday.percent), (1, Some(0)));
    let whole_week = score_days(&deeds, &marks, &Notes::new(), &week()?);
    assert_eq!(whole_week.due, 1);
    Ok(())
}

#[test]
fn no_mark_is_open_every_day_until_friday_ends() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 1)?)];
    let marks = Marks::new();
    let calendar = CalendarSettings::default();
    let open = TodaySummary {
        done: 0,
        partial: 0,
        open: 1,
        missed: 0,
    };
    let nothing = TodaySummary { open: 0, ..open };

    for today in week()? {
        assert_eq!(today_summary(&deeds, &marks, today, &calendar), open);
    }
    for finished in week()?.into_iter().take(6) {
        assert_eq!(
            finished_day_summary(&deeds, &marks, finished, &calendar),
            nothing
        );
    }
    Ok(())
}

fn full_credit(notes: &mut Notes, on: WerdyDay) -> Result<(), Error> {
    notes.insert(on, DayNote::new("ill", ExcuseCredit::Full, on)?);
    Ok(())
}

#[test]
fn excuse_credit_lifts_a_weekly_deed_only_on_its_due_day() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 1)?)];
    let mut marks = Marks::new();
    let mut notes = Notes::new();
    full_credit(&mut notes, day(10, 26)?)?;

    let unmarked = score_days(&deeds, &marks, &notes, &week()?);
    assert_eq!((unmarked.due, unmarked.percent), (1, Some(0)));

    full_credit(&mut notes, day(10, 30)?)?;
    let credit_on_friday = score_days(&deeds, &marks, &notes, &week()?);
    assert_eq!(
        (credit_on_friday.due, credit_on_friday.percent),
        (1, Some(100))
    );

    marked(&mut marks, 1, day(10, 27)?, Mark::partial_without_target());
    let partial_on_tuesday = score_days(&deeds, &marks, &notes, &week()?);
    assert_eq!(
        (partial_on_tuesday.due, partial_on_tuesday.percent),
        (1, Some(50))
    );
    Ok(())
}

/// Tags of the rows on one day's sheet, in order.
fn sheet_tags(deeds: &[Deed], marks: &Marks, on: WerdyDay) -> Vec<Option<RowTag>> {
    daily_sheet(deeds, marks, on, &[], CalendarSettings::default())
        .slots
        .into_iter()
        .flat_map(|slot| slot.rows)
        .map(|row| row.tag)
        .collect()
}

#[test]
fn sheet_shows_an_unmarked_weekly_deed_every_day_with_a_this_week_tag() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 1)?)];
    for on in week()? {
        assert_eq!(
            sheet_tags(&deeds, &Marks::new(), on),
            [Some(RowTag::ThisWeek)]
        );
    }
    Ok(())
}

#[test]
fn a_marked_weekly_deed_shows_only_on_its_marked_day() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 1)?)];
    let mut marks = Marks::new();
    let tuesday = day(10, 27)?;
    marked(&mut marks, 1, tuesday, Mark::Done);

    for on in week()? {
        let expected = if on == tuesday { vec![None] } else { vec![] };
        assert_eq!(sheet_tags(&deeds, &marks, on), expected, "{on:?}");
    }
    Ok(())
}

#[test]
fn consistency_counts_one_due_day_per_week() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 10)?)];
    let mut marks = Marks::new();
    marked(&mut marks, 1, day(10, 13)?, Mark::Done);
    marked(&mut marks, 1, day(10, 26)?, Mark::Done);
    let notes = Notes::new();
    let records = OwnerRecords {
        deeds: &deeds,
        marks: &marks,
        day_notes: &notes,
        calendar: CalendarSettings::default(),
    };

    // Done in the week of 10 October, unmarked in the week of 17 October (due
    // on Friday 23 October), done in the week of 24 October, and still open in
    // the week of 31 October on Wednesday 4 November.
    let tally = consistency(&records, DeedId(1), day(11, 4)?, 7);
    assert_eq!((tally.done, tally.of), (2, 3));
    Ok(())
}

#[test]
fn a_deed_made_on_wednesday_is_due_that_friday_when_unmarked() -> Result<(), Error> {
    let deeds = [weekly_deed(1, day(10, 28)?)];
    let whole_week = score_days(&deeds, &Marks::new(), &Notes::new(), &week()?);
    assert_eq!((whole_week.due, whole_week.percent), (1, Some(0)));
    Ok(())
}

#[test]
fn an_archived_weekly_deed_is_not_due_on_friday() -> Result<(), Error> {
    let mut deed = weekly_deed(1, day(10, 1)?);
    deed.archive(day(10, 29)?)?;
    let whole_week = score_days(&[deed], &Marks::new(), &Notes::new(), &week()?);
    assert_eq!(whole_week.due, 0);
    Ok(())
}
