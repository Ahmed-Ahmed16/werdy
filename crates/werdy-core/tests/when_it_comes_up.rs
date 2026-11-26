//! When it comes up deeds (decision 0047): due only on a day they have a mark.

use std::collections::BTreeMap;

use werdy_core::{
    CalendarSettings, CategoryKey, DayNote, Deed, DeedId, DeedRule, Error, Mark, MarkEntry,
    OwnerRecords, RowTag, Schedule, Score, SlotKey, TodaySummary, WerdyDay, daily_sheet,
    finished_day_summary, score, today_summary,
};

type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;

fn day(month: u8, number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, month, number)
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

fn marked(marks: &mut Marks, id: u64, day: WerdyDay, mark: Mark) {
    marks.insert((DeedId(id), day), MarkEntry::new(mark, day));
}

#[allow(clippy::expect_used)]
fn score_day(deeds: &[Deed], marks: &Marks, on: WerdyDay) -> Score {
    let notes: BTreeMap<WerdyDay, DayNote> = BTreeMap::new();
    let records = OwnerRecords {
        deeds,
        marks,
        day_notes: &notes,
        calendar: CalendarSettings::default(),
    };
    score(&records, &[on], day(11, 1).expect("valid day")).expect("test score fits")
}

#[test]
fn an_unmarked_deed_is_due_on_no_day_and_scores_as_absent() -> Result<(), Error> {
    let on = day(10, 27)?;
    let only_optional = [deed(1, Schedule::WhenItComesUp, day(10, 1)?)];
    let alone = score_day(&only_optional, &Marks::new(), on);
    assert_eq!((alone.due, alone.percent), (0, None));

    let with_daily = [
        deed(1, Schedule::WhenItComesUp, day(10, 1)?),
        deed(2, Schedule::EveryDay, day(10, 1)?),
    ];
    let mut marks = Marks::new();
    marked(&mut marks, 2, on, Mark::Done);
    let beside = score_day(&with_daily, &marks, on);
    assert_eq!((beside.due, beside.percent), (1, Some(100)));
    Ok(())
}

#[test]
fn a_partial_or_missed_mark_counts_that_fraction_on_that_day() -> Result<(), Error> {
    let on = day(10, 27)?;
    let deeds = [deed(1, Schedule::WhenItComesUp, day(10, 1)?)];
    let mut marks = Marks::new();

    marked(&mut marks, 1, on, Mark::partial_without_target());
    let partial = score_day(&deeds, &marks, on);
    assert_eq!((partial.due, partial.percent), (1, Some(50)));

    marked(&mut marks, 1, on, Mark::Missed);
    let missed = score_day(&deeds, &marks, on);
    assert_eq!((missed.due, missed.percent), (1, Some(0)));
    Ok(())
}

#[test]
fn todays_line_leaves_out_an_unmarked_deed_and_counts_a_marked_one() -> Result<(), Error> {
    let on = day(10, 27)?;
    let calendar = CalendarSettings::default();
    let deeds = [
        deed(1, Schedule::WhenItComesUp, day(10, 1)?),
        deed(2, Schedule::EveryDay, day(10, 1)?),
    ];
    let mut marks = Marks::new();
    let one_open = TodaySummary {
        done: 0,
        partial: 0,
        open: 1,
        missed: 0,
    };
    assert_eq!(today_summary(&deeds, &marks, on, &calendar), one_open);
    let one_missed = TodaySummary {
        open: 0,
        missed: 1,
        ..one_open
    };
    assert_eq!(
        finished_day_summary(&deeds, &marks, on, &calendar),
        one_missed
    );

    marked(&mut marks, 1, on, Mark::Done);
    assert_eq!(
        today_summary(&deeds, &marks, on, &calendar),
        TodaySummary {
            done: 1,
            ..one_open
        }
    );
    Ok(())
}

#[test]
fn sheet_shows_it_with_a_quiet_tag_and_its_section_ignores_it_until_marked() -> Result<(), Error> {
    let on = day(10, 27)?;
    let deeds = [
        deed(1, Schedule::WhenItComesUp, day(10, 1)?),
        deed(2, Schedule::EveryDay, day(10, 1)?),
    ];
    let mut marks = Marks::new();
    marked(&mut marks, 2, on, Mark::Done);

    let sheet = daily_sheet(&deeds, &marks, on, &[], CalendarSettings::default());
    let slot = &sheet.slots[0];
    let tags: Vec<_> = slot.rows.iter().map(|row| row.tag).collect();
    assert_eq!(tags, [Some(RowTag::WhenItComesUp), None]);
    let counted: Vec<_> = slot
        .rows
        .iter()
        .map(|row| row.counts_in_section())
        .collect();
    assert_eq!(counted, [false, true]);
    // Only the daily deed counts, and it is Done, so the section can fold.
    assert!(slot.all_entered);
    assert!(slot.complete);

    marked(&mut marks, 1, on, Mark::Missed);
    let sheet = daily_sheet(&deeds, &marks, on, &[], CalendarSettings::default());
    let slot = &sheet.slots[0];
    assert!(slot.rows[0].counts_in_section());
    assert!(slot.all_entered);
    assert!(!slot.complete);
    Ok(())
}

#[test]
fn a_section_of_only_unmarked_such_deeds_folds_but_is_not_complete() -> Result<(), Error> {
    let on = day(10, 27)?;
    let deeds = [deed(1, Schedule::WhenItComesUp, day(10, 1)?)];
    let sheet = daily_sheet(&deeds, &Marks::new(), on, &[], CalendarSettings::default());
    assert!(sheet.slots[0].all_entered);
    assert!(!sheet.slots[0].complete);
    assert!(sheet.unmarked_only().slots.is_empty());
    Ok(())
}
