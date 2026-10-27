use std::collections::BTreeMap;

use werdy_core::{
    CategoryKey, Deed, DeedId, DeedRule, Error, Mark, MarkEntry, OwnerRecords, Schedule, Score,
    SlotKey, Target, TodaySummary, WerdyDay, score as score_records, today_summary,
};

type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;

fn score(deeds: &[Deed], marks: &Marks, days: &[WerdyDay], today: WerdyDay) -> Score {
    score_records(
        &OwnerRecords {
            deeds,
            marks,
            day_notes: &BTreeMap::new(),
        },
        days,
        today,
    )
}

fn day(number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, 10, number)
}

fn daily(target: Option<Target>) -> DeedRule {
    DeedRule {
        schedule: Schedule::EveryDay,
        target,
        category: CategoryKey::new("other"),
        slot: SlotKey::new("any_time"),
    }
}

#[test]
fn three_of_five_pages_is_worth_point_six() -> Result<(), Error> {
    let target = Target::new(5, "pages")?;
    let marked_day = day(23)?;
    let deed = Deed::new(DeedId(1), marked_day, daily(Some(target.clone())));
    let mark = Mark::Count(3);
    let marks = BTreeMap::from([((deed.id, marked_day), MarkEntry::new(mark, marked_day))]);

    assert_eq!(mark.value(&daily(Some(target))), 3.0 / 5.0);
    assert_eq!(
        score(&[deed], &marks, &[marked_day], day(24)?).earned,
        3.0 / 5.0
    );
    Ok(())
}

#[test]
fn count_at_target_is_done() -> Result<(), Error> {
    let target = Target::new(5, "pages")?;

    assert_eq!(Mark::Count(5).value(&daily(Some(target))), 1.0);
    Ok(())
}

#[test]
fn count_above_target_is_done_and_capped_at_one() -> Result<(), Error> {
    let target = Target::new(5, "pages")?;
    let mark = Mark::Count(6);

    assert_eq!(mark.value(&daily(Some(target))), 1.0);
    Ok(())
}

#[test]
fn count_of_zero_is_missed() -> Result<(), Error> {
    let target = Target::new(5, "pages")?;
    let mark = Mark::Count(0);

    assert_eq!(mark.value(&daily(Some(target))), 0.0);
    Ok(())
}

#[test]
fn partial_without_target_is_worth_a_half() -> Result<(), Error> {
    let marked_day = day(23)?;
    let deed = Deed::new(DeedId(1), marked_day, daily(None));
    let mark = Mark::partial_without_target();
    let marks = BTreeMap::from([((deed.id, marked_day), MarkEntry::new(mark, marked_day))]);

    assert_eq!(mark.value(&daily(None)), 0.5);
    assert_eq!(score(&[deed], &marks, &[marked_day], day(24)?).earned, 0.5);
    Ok(())
}

#[test]
fn target_change_does_not_move_past_scores() -> Result<(), Error> {
    let first = day(23)?;
    let second = day(24)?;
    let old_target = Target::new(5, "pages")?;
    let new_target = Target::new(10, "pages")?;
    let mut deed = Deed::new(DeedId(1), first, daily(Some(old_target.clone())));
    let marks = BTreeMap::from([((deed.id, first), MarkEntry::new(Mark::Count(3), day(25)?))]);
    let before = score(&[deed.clone()], &marks, &[first], day(25)?);
    assert_eq!(before.earned, 3.0 / 5.0);

    deed.change_rule(second, daily(Some(new_target.clone())))?;

    assert_eq!(deed.rule_on(first), Some(&daily(Some(old_target))));
    assert_eq!(deed.rule_on(second), Some(&daily(Some(new_target))));
    assert_eq!(score(&[deed.clone()], &marks, &[first], day(25)?), before);
    let later_marks = BTreeMap::from([((deed.id, second), MarkEntry::new(Mark::Count(3), second))]);
    assert_eq!(
        score(&[deed], &later_marks, &[second], day(25)?).earned,
        3.0 / 10.0
    );
    Ok(())
}

#[test]
fn count_is_kept_for_display() -> Result<(), Error> {
    let marked_day = day(23)?;
    let deed = Deed::new(DeedId(1), marked_day, daily(Some(Target::new(5, "pages")?)));
    let marks = BTreeMap::from([(
        (deed.id, marked_day),
        MarkEntry::new(Mark::Count(3), marked_day),
    )]);

    assert_eq!(marks[&(deed.id, marked_day)].mark, Mark::Count(3));
    assert_eq!(
        deed.rule_on(marked_day)
            .and_then(|rule| rule.target.as_ref())
            .map(Target::amount),
        Some(5)
    );
    Ok(())
}

#[test]
fn today_counts_zero_partial_and_full_counts() -> Result<(), Error> {
    let today = day(24)?;
    let target = Target::new(5, "pages")?;
    let deeds: Vec<_> = (1..=5)
        .map(|id| Deed::new(DeedId(id), today, daily(Some(target.clone()))))
        .collect();
    let marks = BTreeMap::from([
        ((DeedId(1), today), MarkEntry::new(Mark::Count(0), today)),
        ((DeedId(2), today), MarkEntry::new(Mark::Count(3), today)),
        ((DeedId(3), today), MarkEntry::new(Mark::Count(5), today)),
        ((DeedId(4), today), MarkEntry::new(Mark::Count(6), today)),
    ]);

    assert_eq!(
        today_summary(&deeds, &marks, today),
        TodaySummary {
            done: 2,
            partial: 1,
            open: 1,
            missed: 1
        }
    );
    Ok(())
}

#[test]
fn count_without_target_uses_binary_value() -> Result<(), Error> {
    let first = day(23)?;
    let second = day(24)?;
    let deed = Deed::new(DeedId(1), first, daily(None));
    let marks = BTreeMap::from([
        ((deed.id, first), MarkEntry::new(Mark::Count(0), first)),
        ((deed.id, second), MarkEntry::new(Mark::Count(3), second)),
    ]);

    assert_eq!(Mark::Count(0).value(&daily(None)), 0.0);
    assert_eq!(Mark::Count(3).value(&daily(None)), 1.0);
    assert_eq!(
        score(
            std::slice::from_ref(&deed),
            &marks,
            &[first, second],
            day(25)?
        )
        .earned,
        1.0
    );
    assert_eq!(
        today_summary(std::slice::from_ref(&deed), &marks, first).missed,
        1
    );
    assert_eq!(today_summary(&[deed], &marks, second).done, 1);
    Ok(())
}

#[test]
fn target_of_zero_is_an_error() {
    assert_eq!(Target::new(0, "pages"), Err(Error::InvalidTargetAmount));
}

#[test]
fn empty_target_unit_is_an_error() {
    assert_eq!(Target::new(5, ""), Err(Error::EmptyTargetUnit));
}
