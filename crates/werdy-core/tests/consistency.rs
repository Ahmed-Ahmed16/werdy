use std::collections::{BTreeMap, BTreeSet};

use werdy_core::{
    CalendarSettings, CategoryKey, Consistency, DEFAULT_CONSISTENCY_WINDOW, DayNote, Deed, DeedId,
    DeedRule, Error, ExcuseCredit, Mark, MarkEntry, OwnerRecords, Schedule, SlotKey, Target,
    Weekday, WerdyDay, consistency, kept_days,
};

type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;

fn day(month: u8, number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, month, number)
}

fn deed(id: u64, created_on: WerdyDay, schedule: Schedule) -> Deed {
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

fn records<'a>(
    deeds: &'a [Deed],
    marks: &'a Marks,
    notes: &'a BTreeMap<WerdyDay, DayNote>,
) -> OwnerRecords<'a> {
    OwnerRecords {
        deeds,
        marks,
        day_notes: notes,
        calendar: CalendarSettings::default(),
    }
}

#[test]
fn friday_deed_looks_back_over_its_last_seven_fridays() -> Result<(), Error> {
    let fridays = Schedule::Weekdays(BTreeSet::from([Weekday::Friday]));
    let deeds = [deed(1, day(9, 4)?, fridays)];
    let mut marks = Marks::new();
    marked(&mut marks, 1, day(9, 4)?, Mark::Done);
    for (month, number) in [(9, 11), (9, 18), (9, 25), (10, 2), (10, 9), (10, 16)] {
        marked(&mut marks, 1, day(month, number)?, Mark::Done);
    }
    marked(&mut marks, 1, day(10, 23)?, Mark::Missed);
    marked(&mut marks, 1, day(10, 30)?, Mark::Done);

    assert_eq!(
        consistency(
            &records(&deeds, &marks, &BTreeMap::new()),
            DeedId(1),
            day(10, 31)?,
            DEFAULT_CONSISTENCY_WINDOW,
        ),
        Consistency { done: 6, of: 7 }
    );
    Ok(())
}

#[test]
fn one_missed_day_lowers_consistency_by_one_not_to_zero() -> Result<(), Error> {
    let deeds = [deed(1, day(10, 20)?, Schedule::EveryDay)];
    let mut marks = Marks::new();
    for number in 20..=26 {
        marked(
            &mut marks,
            1,
            day(10, number)?,
            if number == 23 {
                Mark::Missed
            } else {
                Mark::Done
            },
        );
    }

    assert_eq!(
        consistency(
            &records(&deeds, &marks, &BTreeMap::new()),
            DeedId(1),
            day(10, 27)?,
            7
        ),
        Consistency { done: 6, of: 7 }
    );
    Ok(())
}

#[test]
fn new_deed_has_fewer_than_seven_due_days() -> Result<(), Error> {
    let deeds = [deed(1, day(10, 24)?, Schedule::EveryDay)];
    let mut marks = Marks::new();
    marked(&mut marks, 1, day(10, 24)?, Mark::Done);
    marked(&mut marks, 1, day(10, 25)?, Mark::Done);

    assert_eq!(
        consistency(
            &records(&deeds, &marks, &BTreeMap::new()),
            DeedId(1),
            day(10, 27)?,
            7
        ),
        Consistency { done: 2, of: 3 }
    );
    Ok(())
}

#[test]
fn today_never_counts_in_consistency() -> Result<(), Error> {
    let deeds = [deed(1, day(10, 25)?, Schedule::EveryDay)];
    let mut marks = Marks::new();
    marked(&mut marks, 1, day(10, 25)?, Mark::Done);
    marked(&mut marks, 1, day(10, 27)?, Mark::Done);

    assert_eq!(
        consistency(
            &records(&deeds, &marks, &BTreeMap::new()),
            DeedId(1),
            day(10, 27)?,
            7
        ),
        Consistency { done: 1, of: 2 }
    );
    Ok(())
}

#[test]
fn kept_days_count_missed_marks_too() -> Result<(), Error> {
    let deeds = [deed(1, day(10, 24)?, Schedule::EveryDay)];
    let mut marks = Marks::new();
    marked(&mut marks, 1, day(10, 24)?, Mark::Missed);
    marked(&mut marks, 1, day(10, 25)?, Mark::Done);

    assert_eq!(
        kept_days(&records(&deeds, &marks, &BTreeMap::new()), day(10, 27)?),
        2
    );
    Ok(())
}

#[test]
fn kept_days_skip_days_with_an_unmarked_due_deed() -> Result<(), Error> {
    let deeds = [
        deed(1, day(10, 24)?, Schedule::EveryDay),
        deed(2, day(10, 25)?, Schedule::EveryDay),
    ];
    let mut marks = Marks::new();
    marked(&mut marks, 1, day(10, 24)?, Mark::Done);
    marked(&mut marks, 1, day(10, 25)?, Mark::Done);
    marked(&mut marks, 1, day(10, 26)?, Mark::Missed);
    marked(&mut marks, 2, day(10, 26)?, Mark::Missed);

    assert_eq!(
        kept_days(&records(&deeds, &marks, &BTreeMap::new()), day(10, 27)?),
        2
    );
    Ok(())
}

#[test]
fn day_with_excuse_credit_counts_as_kept() -> Result<(), Error> {
    let deeds = [deed(1, day(10, 24)?, Schedule::EveryDay)];
    let marks = Marks::new();
    let notes = BTreeMap::from([
        (
            day(10, 24)?,
            DayNote::new("Ill", ExcuseCredit::Half, day(10, 24)?)?,
        ),
        (
            day(10, 25)?,
            DayNote::new("Rest", ExcuseCredit::None, day(10, 25)?)?,
        ),
    ]);

    assert_eq!(kept_days(&records(&deeds, &marks, &notes), day(10, 27)?), 1);
    Ok(())
}

#[test]
fn consistency_uses_target_values_and_full_day_credit() -> Result<(), Error> {
    let deeds = [Deed::new(
        DeedId(1),
        day(10, 24)?,
        DeedRule {
            schedule: Schedule::EveryDay,
            target: Some(Target::new(5, "pages")?),
            category: CategoryKey::new("other"),
            slot: SlotKey::new("any_time"),
        },
    )];
    let mut marks = Marks::new();
    marked(&mut marks, 1, day(10, 24)?, Mark::Count(5));
    marked(&mut marks, 1, day(10, 25)?, Mark::Count(4));
    let notes = BTreeMap::from([(
        day(10, 26)?,
        DayNote::new("Ill", ExcuseCredit::Full, day(10, 26)?)?,
    )]);

    assert_eq!(
        consistency(&records(&deeds, &marks, &notes), DeedId(1), day(10, 27)?, 7),
        Consistency { done: 2, of: 3 }
    );
    Ok(())
}

#[test]
fn days_with_nothing_due_do_not_count_as_kept() -> Result<(), Error> {
    let deeds = [deed(
        1,
        day(10, 23)?,
        Schedule::Weekdays(BTreeSet::from([Weekday::Friday])),
    )];
    let mut marks = Marks::new();
    marked(&mut marks, 1, day(10, 23)?, Mark::Missed);
    let notes = BTreeMap::from([(
        day(10, 24)?,
        DayNote::new("Ill", ExcuseCredit::Full, day(10, 24)?)?,
    )]);

    assert_eq!(kept_days(&records(&deeds, &marks, &notes), day(10, 26)?), 1);
    Ok(())
}
