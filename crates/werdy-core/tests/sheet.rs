use std::collections::{BTreeMap, BTreeSet};

use werdy_core::{
    CategoryKey, Deed, DeedId, DeedRule, Error, Mark, MarkEntry, Schedule, SlotKey, Target,
    Weekday, WerdyDay, daily_sheet,
};

type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;

fn day() -> Result<WerdyDay, Error> {
    // 2026-10-27 is a Tuesday.
    WerdyDay::new(2026, 10, 27)
}

fn slot_order() -> Vec<SlotKey> {
    [
        "fajr", "morning", "dhuhr", "asr", "maghrib", "isha", "night", "any_time",
    ]
    .into_iter()
    .map(SlotKey::new)
    .collect()
}

fn rule(slot: &str, schedule: Schedule, target: Option<Target>) -> DeedRule {
    DeedRule {
        schedule,
        target,
        category: CategoryKey::new("prayer"),
        slot: SlotKey::new(slot),
    }
}

fn deed(id: u64, slot: &str, created_on: WerdyDay) -> Deed {
    Deed::new(DeedId(id), created_on, rule(slot, Schedule::EveryDay, None))
}

fn slot_keys(deeds: &[Deed], marks: &Marks, on: WerdyDay) -> Vec<String> {
    daily_sheet(deeds, marks, on, &slot_order())
        .slots
        .into_iter()
        .map(|slot| slot.slot.0)
        .collect()
}

#[test]
fn slots_follow_the_given_order_and_empty_slots_are_hidden() -> Result<(), Error> {
    let today = day()?;
    let deeds = vec![
        deed(1, "isha", today),
        deed(2, "fajr", today),
        deed(3, "asr", today),
    ];

    assert_eq!(
        slot_keys(&deeds, &Marks::new(), today),
        ["fajr", "asr", "isha"]
    );
    Ok(())
}

#[test]
fn rows_keep_deed_order_and_carry_their_mark() -> Result<(), Error> {
    let today = day()?;
    let deeds = vec![deed(1, "fajr", today), deed(2, "fajr", today)];
    let marks = Marks::from([((DeedId(2), today), MarkEntry::new(Mark::Missed, today))]);

    let sheet = daily_sheet(&deeds, &marks, today, &slot_order());

    let rows: Vec<_> = sheet.slots[0]
        .rows
        .iter()
        .map(|row| (row.deed_id, row.mark))
        .collect();
    assert_eq!(rows, [(DeedId(1), None), (DeedId(2), Some(Mark::Missed))]);
    Ok(())
}

#[test]
fn deeds_not_due_that_day_are_left_out() -> Result<(), Error> {
    let today = day()?;
    let fridays = Schedule::Weekdays(BTreeSet::from([Weekday::Friday]));
    let deeds = vec![
        Deed::new(DeedId(1), today, rule("dhuhr", fridays, None)),
        deed(2, "night", today.next()?),
    ];

    assert!(slot_keys(&deeds, &Marks::new(), today).is_empty());
    Ok(())
}

#[test]
fn a_slot_folds_only_when_every_deed_in_it_is_done() -> Result<(), Error> {
    let today = day()?;
    let deeds = vec![
        deed(1, "fajr", today),
        deed(2, "fajr", today),
        deed(3, "asr", today),
    ];
    let marks = Marks::from([
        ((DeedId(1), today), MarkEntry::new(Mark::Done, today)),
        ((DeedId(2), today), MarkEntry::new(Mark::Done, today)),
        (
            (DeedId(3), today),
            MarkEntry::new(Mark::partial_without_target(), today),
        ),
    ]);

    let folded: Vec<_> = daily_sheet(&deeds, &marks, today, &slot_order())
        .slots
        .iter()
        .map(|slot| slot.folded)
        .collect();
    assert_eq!(folded, [true, false]);
    Ok(())
}

#[test]
fn a_count_at_its_target_counts_as_done_for_folding() -> Result<(), Error> {
    let today = day()?;
    let pages = Target::new(5, "page")?;
    let deeds = vec![Deed::new(
        DeedId(1),
        today,
        rule("morning", Schedule::EveryDay, Some(pages.clone())),
    )];
    let marks = Marks::from([((DeedId(1), today), MarkEntry::new(Mark::Count(5), today))]);

    let sheet = daily_sheet(&deeds, &marks, today, &slot_order());

    assert!(sheet.slots[0].folded);
    assert_eq!(sheet.slots[0].rows[0].target, Some(pages));
    Ok(())
}

#[test]
fn slots_missing_from_the_order_come_last_in_key_order() -> Result<(), Error> {
    let today = day()?;
    let deeds = vec![
        deed(1, "zuhr_typo", today),
        deed(2, "duha", today),
        deed(3, "isha", today),
    ];

    assert_eq!(
        slot_keys(&deeds, &Marks::new(), today),
        ["isha", "duha", "zuhr_typo"]
    );
    Ok(())
}

#[test]
fn the_slot_is_read_from_the_rule_in_force_that_day() -> Result<(), Error> {
    let today = day()?;
    let tomorrow = today.next()?;
    let mut moved = deed(1, "fajr", today);
    moved.change_rule(tomorrow, rule("night", Schedule::EveryDay, None))?;
    let deeds = vec![moved];

    assert_eq!(slot_keys(&deeds, &Marks::new(), today), ["fajr"]);
    assert_eq!(slot_keys(&deeds, &Marks::new(), tomorrow), ["night"]);
    Ok(())
}
