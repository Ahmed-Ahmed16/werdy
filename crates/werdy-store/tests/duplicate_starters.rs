use std::collections::BTreeSet;

use werdy_core::Target;
use werdy_core::{
    CategoryKey, Deed, DeedId, DeedRule, Mark, MarkEntry, Schedule, SlotKey, WerdyDay,
};
use werdy_store::{ClockTime, DeedKey, DeedName, OwnerData};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn day(date: u8) -> Result<WerdyDay, werdy_core::Error> {
    WerdyDay::new(2026, 10, date)
}

fn rule() -> DeedRule {
    DeedRule {
        schedule: Schedule::EveryDay,
        target: None,
        category: CategoryKey::new("prayer"),
        slot: SlotKey::new("fajr"),
    }
}

/// Adds a starter deed made by `install`, created on `created`.
fn add(records: &mut OwnerData, id: u64, created: WerdyDay, install: &str, key: &str) {
    records.deeds.push(Deed::new(DeedId(id), created, rule()));
    records.starter_keys.insert(DeedId(id), key.to_owned());
    records.deed_keys.insert(
        DeedId(id),
        DeedKey {
            install: install.to_owned(),
            sequence: id,
        },
    );
    records.last_deed_id = records.last_deed_id.max(id);
}

fn mark(records: &mut OwnerData, id: u64, on: WerdyDay, mark: Mark, now: u64) {
    records.set_mark(DeedId(id), on, Some(MarkEntry::new(mark, on)), now);
}

#[test]
fn oldest_copy_is_kept_and_takes_the_other_copys_marks() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(30)?, "phone", "fajr_on_time");
    add(&mut records, 2, day(20)?, "web", "fajr_on_time");
    mark(&mut records, 1, day(30)?, Mark::Done, 500);
    mark(&mut records, 2, day(25)?, Mark::Missed, 400);

    assert!(records.merge_duplicate_starters(&BTreeSet::new()));

    let ids: Vec<DeedId> = records.deeds.iter().map(|deed| deed.id).collect();
    assert_eq!(ids, [DeedId(2)]);
    assert_eq!(records.marks[&(DeedId(2), day(30)?)].mark, Mark::Done);
    assert_eq!(records.marks[&(DeedId(2), day(25)?)].mark, Mark::Missed);
    assert_eq!(records.marks.len(), 2);
    assert!(records.deleted_deeds.contains(&DeedId(1)));
    assert!(!records.starter_keys.contains_key(&DeedId(1)));
    Ok(())
}

#[test]
fn on_a_day_both_copies_marked_the_later_change_wins() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(30)?, "phone", "fajr_on_time");
    let today = day(30)?;
    mark(&mut records, 1, today, Mark::Missed, 100);
    mark(&mut records, 2, today, Mark::Done, 200);
    records.merge_duplicate_starters(&BTreeSet::new());
    assert_eq!(records.marks[&(DeedId(1), today)].mark, Mark::Done);

    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(30)?, "phone", "fajr_on_time");
    mark(&mut records, 2, today, Mark::Done, 100);
    // The kept copy was cleared after the other copy was marked.
    mark(&mut records, 1, today, Mark::Missed, 50);
    records.set_mark(DeedId(1), today, None, 300);
    records.merge_duplicate_starters(&BTreeSet::new());
    assert!(!records.marks.contains_key(&(DeedId(1), today)));
    Ok(())
}

#[test]
fn every_device_keeps_the_same_copy_whatever_its_local_ids() -> TestResult {
    let mut here = OwnerData::default();
    add(&mut here, 1, day(20)?, "b_install", "fajr_on_time");
    add(&mut here, 2, day(20)?, "a_install", "fajr_on_time");
    let mut there = OwnerData::default();
    add(&mut there, 7, day(20)?, "a_install", "fajr_on_time");
    add(&mut there, 9, day(20)?, "b_install", "fajr_on_time");
    // Same lasting keys, different local ids.
    there.deed_keys.insert(
        DeedId(7),
        DeedKey {
            install: "a_install".into(),
            sequence: 2,
        },
    );
    there.deed_keys.insert(
        DeedId(9),
        DeedKey {
            install: "b_install".into(),
            sequence: 1,
        },
    );

    here.merge_duplicate_starters(&BTreeSet::new());
    there.merge_duplicate_starters(&BTreeSet::new());

    assert_eq!(
        here.deed_keys[&here.deeds[0].id],
        there.deed_keys[&there.deeds[0].id]
    );
    Ok(())
}

#[test]
fn an_older_archived_copy_is_kept_with_its_history_and_comes_back_live() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(30)?, "phone", "fajr_on_time");
    mark(&mut records, 1, day(21)?, Mark::Done, 100);
    records.deeds[0].archive(day(25)?)?;
    records.merge_duplicate_starters(&BTreeSet::new());

    let kept = &records.deeds[0];
    assert_eq!(records.deeds.len(), 1);
    assert_eq!(kept.id, DeedId(1));
    // The day marked before the live copy existed still has a due rule.
    assert!(kept.rule_on(day(21)?).is_some());
    assert!(!kept.is_archived_on(day(21)?));
    assert!(kept.is_archived_on(day(27)?));
    assert!(!kept.is_archived_on(day(30)?));
    Ok(())
}

#[test]
fn an_unsent_mark_on_the_copy_beats_a_synced_one_on_the_kept_deed() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(30)?, "phone", "fajr_on_time");
    let today = day(30)?;
    // A synced mark carries the server time; an unsent one carries zero.
    mark(&mut records, 1, today, Mark::Missed, 1_800_000_000_000);
    records
        .marks
        .insert((DeedId(2), today), MarkEntry::new(Mark::Done, today));
    records.mark_revisions.insert((DeedId(2), today), 0);
    let unsent = BTreeSet::from([(DeedId(2), today)]);
    records.merge_duplicate_starters(&unsent);
    assert_eq!(records.marks[&(DeedId(1), today)].mark, Mark::Done);
    Ok(())
}

#[test]
fn a_reminder_on_the_removed_copy_moves_to_the_kept_one() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(30)?, "phone", "fajr_on_time");
    let time = ClockTime::new(5, 30)?;
    records.settings.reminders.deeds.insert(DeedId(2), time);
    records.merge_duplicate_starters(&BTreeSet::new());
    assert_eq!(
        records.settings.reminders.deeds.get(&DeedId(1)),
        Some(&time)
    );
    assert!(!records.settings.reminders.deeds.contains_key(&DeedId(2)));
    Ok(())
}

#[test]
fn a_later_clear_on_the_removed_copy_clears_the_kept_deed() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(30)?, "phone", "fajr_on_time");
    let today = day(30)?;
    mark(&mut records, 1, today, Mark::Done, 100);
    mark(&mut records, 2, today, Mark::Missed, 150);
    records.set_mark(DeedId(2), today, None, 300);
    records.merge_duplicate_starters(&BTreeSet::new());
    assert!(!records.marks.contains_key(&(DeedId(1), today)));
    assert!(records.cleared_marks.contains_key(&(DeedId(1), today)));
    Ok(())
}

#[test]
fn a_rule_edit_on_the_removed_copy_moves_to_the_kept_deed() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(25)?, "phone", "fajr_on_time");
    let edited = DeedRule {
        target: Some(Target::new(2, "rakah")?),
        ..rule()
    };
    records.deeds[1].change_rule(day(28)?, edited.clone())?;
    records.merge_duplicate_starters(&BTreeSet::new());
    let kept = &records.deeds[0];
    assert_eq!(kept.id, DeedId(1));
    assert_eq!(kept.rule_on(day(27)?), Some(&rule()));
    assert_eq!(kept.rule_on(day(28)?), Some(&edited));
    Ok(())
}

#[test]
fn a_typed_name_on_the_removed_copy_is_kept() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(30)?, "phone", "fajr_on_time");
    records
        .names
        .insert(DeedId(1), DeedName::Key("starter-fajr-on-time".into()));
    records
        .names
        .insert(DeedId(2), DeedName::Text("Fajr at the mosque".into()));
    records.merge_duplicate_starters(&BTreeSet::new());
    assert_eq!(
        records.names.get(&DeedId(1)),
        Some(&DeedName::Text("Fajr at the mosque".into()))
    );
    assert!(!records.names.contains_key(&DeedId(2)));
    Ok(())
}

#[test]
fn distinct_starters_and_own_deeds_are_left_alone() -> TestResult {
    let mut records = OwnerData::default();
    add(&mut records, 1, day(20)?, "web", "fajr_on_time");
    add(&mut records, 2, day(20)?, "web", "fajr_sunnah");
    records.deeds.push(Deed::new(DeedId(3), day(20)?, rule()));
    records.deeds.push(Deed::new(DeedId(4), day(20)?, rule()));
    let before = records.clone();
    assert!(!records.merge_duplicate_starters(&BTreeSet::new()));
    assert_eq!(records, before);
    Ok(())
}
