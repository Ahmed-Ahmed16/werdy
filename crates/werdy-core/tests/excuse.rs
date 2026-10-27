use std::collections::BTreeMap;

use werdy_core::{
    CategoryKey, DayNote, Deed, DeedId, DeedRule, Error, ExcuseCredit, Mark, MarkEntry,
    OwnerRecords, Schedule, Score, SlotKey, Target, WerdyDay, score,
};

fn day(number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, 10, number)
}

fn deed(id: u64, created_on: WerdyDay, target: Option<Target>) -> Deed {
    Deed::new(
        DeedId(id),
        created_on,
        DeedRule {
            schedule: Schedule::EveryDay,
            target,
            category: CategoryKey::new("other"),
            slot: SlotKey::new("any_time"),
        },
    )
}

type Marks = BTreeMap<(DeedId, WerdyDay), MarkEntry>;
type DayNotes = BTreeMap<WerdyDay, DayNote>;

fn score_day(
    deeds: &[Deed],
    marks: &Marks,
    day_notes: &DayNotes,
    scored_day: WerdyDay,
    today: WerdyDay,
) -> Score {
    score(
        &OwnerRecords {
            deeds,
            marks,
            day_notes,
        },
        &[scored_day],
        today,
    )
}

#[test]
fn day_without_credit_scores_as_usual() -> Result<(), Error> {
    let scored_day = day(23)?;
    let deeds = [deed(1, scored_day, None), deed(2, scored_day, None)];
    let marks = Marks::from([(
        (DeedId(1), scored_day),
        MarkEntry::new(Mark::Done, scored_day),
    )]);
    let day_notes = DayNotes::from([(
        scored_day,
        DayNote::new("Travelled", ExcuseCredit::None, scored_day)?,
    )]);

    assert_eq!(
        score_day(&deeds, &marks, &day_notes, scored_day, day(24)?).earned,
        1.0
    );
    Ok(())
}

#[test]
fn half_credit_lifts_missed_to_a_half() -> Result<(), Error> {
    let scored_day = day(23)?;
    let deeds = [deed(1, scored_day, None), deed(2, scored_day, None)];
    let marks = Marks::from([(
        (DeedId(1), scored_day),
        MarkEntry::new(Mark::Missed, scored_day),
    )]);
    let day_notes = DayNotes::from([(
        scored_day,
        DayNote::new("Ill", ExcuseCredit::Half, scored_day)?,
    )]);

    let result = score_day(&deeds, &marks, &day_notes, scored_day, day(24)?);
    assert_eq!(result.earned, 1.0);
    assert_eq!(result.due, 2);
    assert_eq!(result.percent, Some(50));
    Ok(())
}

#[test]
fn half_credit_keeps_a_higher_partial() -> Result<(), Error> {
    let scored_day = day(23)?;
    let deeds = [deed(1, scored_day, Some(Target::new(5, "pages")?))];
    let marks = Marks::from([(
        (DeedId(1), scored_day),
        MarkEntry::new(Mark::Count(3), scored_day),
    )]);
    let day_notes = DayNotes::from([(
        scored_day,
        DayNote::new("Ill", ExcuseCredit::Half, scored_day)?,
    )]);

    let result = score_day(&deeds, &marks, &day_notes, scored_day, day(24)?);
    assert_eq!(result.earned, 0.6);
    assert_eq!(result.percent, Some(60));
    Ok(())
}

#[test]
fn full_credit_makes_every_due_deed_worth_one() -> Result<(), Error> {
    let scored_day = day(23)?;
    let deeds = [
        deed(1, scored_day, None),
        deed(2, scored_day, None),
        deed(3, scored_day, None),
    ];
    let marks = Marks::from([
        (
            (DeedId(1), scored_day),
            MarkEntry::new(Mark::Done, scored_day),
        ),
        (
            (DeedId(2), scored_day),
            MarkEntry::new(Mark::Missed, scored_day),
        ),
    ]);
    let day_notes = DayNotes::from([(
        scored_day,
        DayNote::new("Ill", ExcuseCredit::Full, scored_day)?,
    )]);

    let result = score_day(&deeds, &marks, &day_notes, scored_day, day(24)?);
    assert_eq!(result.earned, 3.0);
    assert_eq!(result.due, 3);
    assert_eq!(result.percent, Some(100));
    Ok(())
}

#[test]
fn credit_never_changes_the_stored_marks() -> Result<(), Error> {
    let scored_day = day(23)?;
    let deed = deed(1, scored_day, None);
    let marks = Marks::from([(
        (deed.id, scored_day),
        MarkEntry::new(Mark::Missed, scored_day),
    )]);
    let day_notes = DayNotes::from([(
        scored_day,
        DayNote::new("Ill", ExcuseCredit::Full, scored_day)?,
    )]);

    assert_eq!(
        score_day(&[deed], &marks, &day_notes, scored_day, day(24)?).earned,
        1.0
    );
    assert_eq!(marks[&(DeedId(1), scored_day)].mark, Mark::Missed);
    Ok(())
}

#[test]
fn credit_affects_only_its_werdy_day() -> Result<(), Error> {
    let credited_day = day(23)?;
    let next_day = day(24)?;
    let deeds = [deed(1, credited_day, None)];
    let day_notes = DayNotes::from([(
        credited_day,
        DayNote::new("Ill", ExcuseCredit::Full, credited_day)?,
    )]);

    assert_eq!(
        score_day(&deeds, &Marks::new(), &day_notes, credited_day, day(25)?).earned,
        1.0
    );
    assert_eq!(
        score_day(&deeds, &Marks::new(), &day_notes, next_day, day(25)?).earned,
        0.0
    );
    Ok(())
}

#[test]
fn note_added_three_days_late_has_late_flag() -> Result<(), Error> {
    let note = DayNote::new("Ill", ExcuseCredit::None, day(26)?)?;

    assert!(note.is_late_for(day(23)?));
    assert!(!note.is_late_for(day(24)?));
    Ok(())
}

#[test]
fn empty_note_without_credit_is_an_error() -> Result<(), Error> {
    let entered_on = day(23)?;

    assert_eq!(
        DayNote::new("", ExcuseCredit::None, entered_on),
        Err(Error::EmptyDayNote)
    );
    assert!(DayNote::new("", ExcuseCredit::Half, entered_on).is_ok());
    Ok(())
}
