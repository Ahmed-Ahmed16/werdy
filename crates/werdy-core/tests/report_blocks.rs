//! The glance blocks of the reports page (#112): ranked deeds, the prayer
//! grid, the change from the previous period and categories weakest first.

use std::collections::BTreeMap;

use werdy_core::{
    CalendarSettings, CategoryKey, DayDot, DayNote, Deed, DeedId, DeedRule, Error, ExcuseCredit,
    Mark, MarkEntry, OwnerRecords, Period, PrayerRow, PrayerSource, PrayerTally, Report, Schedule,
    Score, ScoreBands, ScoreValue, SlotKey, Weekday, WerdyDay, build_report,
    categories_weakest_first, missed_prayers, percent_change, prayer_grid, ranked_deeds,
};

fn day(month: u8, number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(2026, month, number)
}

fn deed(id: u64, category: &str, created_on: WerdyDay) -> Deed {
    Deed::new(
        DeedId(id),
        created_on,
        DeedRule {
            schedule: Schedule::EveryDay,
            target: None,
            category: CategoryKey::new(category),
            slot: SlotKey::new("any_time"),
        },
    )
}

/// A week ending on 26 October with every deed created a month earlier.
struct Week {
    deeds: Vec<Deed>,
    marks: BTreeMap<(DeedId, WerdyDay), MarkEntry>,
    notes: BTreeMap<WerdyDay, DayNote>,
}

impl Week {
    fn new(categories: &[&str]) -> Result<Self, Error> {
        let created = day(9, 1)?;
        let deeds = categories
            .iter()
            .zip(1..)
            .map(|(category, id)| deed(id, category, created))
            .collect();
        Ok(Self {
            deeds,
            marks: BTreeMap::new(),
            notes: BTreeMap::new(),
        })
    }

    /// Marks deed `id` Done on the first `done` days of the week.
    fn done_on_first(&mut self, id: u64, done: usize) -> Result<(), Error> {
        for marked in Period::Week.days(day(10, 27)?, 0).into_iter().take(done) {
            self.marks
                .insert((DeedId(id), marked), MarkEntry::new(Mark::Done, marked));
        }
        Ok(())
    }

    fn mark(&mut self, id: u64, marked: WerdyDay, mark: Mark, entered: WerdyDay) {
        self.marks
            .insert((DeedId(id), marked), MarkEntry::new(mark, entered));
    }

    fn report(&self) -> Result<Report, Error> {
        build_report(
            &OwnerRecords {
                deeds: &self.deeds,
                marks: &self.marks,
                day_notes: &self.notes,
                calendar: CalendarSettings::default(),
            },
            Period::Week,
            0,
            day(10, 27)?,
            ScoreBands::default(),
        )
    }
}

fn ids(ranks: &[werdy_core::DeedRank]) -> Vec<u64> {
    ranks.iter().map(|rank| rank.deed_id.0).collect()
}

fn score(percent: Option<u8>) -> Result<Score, Error> {
    Ok(Score {
        earned: ScoreValue::from_ratio(u128::from(percent.unwrap_or(0)), 100)?,
        due: usize::from(percent.is_some()),
        percent,
    })
}

#[test]
fn ranked_deeds_take_the_bottom_and_top_of_many() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"; 7])?;
    for id in 1..=7 {
        week.done_on_first(id, usize::try_from(id - 1).unwrap_or(0))?;
    }
    let ranked = ranked_deeds(&week.report()?, 3);
    assert_eq!(ids(&ranked.weakest), [1, 2, 3]);
    assert_eq!(ids(&ranked.strongest), [7, 6, 5]);
    assert_eq!(ranked.weakest[0].score.percent, Some(0));
    Ok(())
}

#[test]
fn ranked_deeds_split_a_short_list_without_repeating_a_deed() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"; 4])?;
    for (id, done) in [(1, 7), (2, 6), (3, 3), (4, 1)] {
        week.done_on_first(id, done)?;
    }
    let ranked = ranked_deeds(&week.report()?, 3);
    assert_eq!(ids(&ranked.weakest), [4, 3]);
    assert_eq!(ids(&ranked.strongest), [1, 2]);
    Ok(())
}

#[test]
fn a_full_deed_is_never_weakest_and_an_empty_deed_never_strongest() -> Result<(), Error> {
    let mut full = Week::new(&["prayer"; 2])?;
    full.done_on_first(1, 7)?;
    full.done_on_first(2, 7)?;
    let ranked = ranked_deeds(&full.report()?, 3);
    assert!(ranked.weakest.is_empty());
    assert_eq!(ids(&ranked.strongest), [1, 2]);

    let empty = Week::new(&["prayer"; 2])?;
    let ranked = ranked_deeds(&empty.report()?, 3);
    assert_eq!(ids(&ranked.weakest), [1]);
    assert!(ranked.strongest.is_empty());
    Ok(())
}

#[test]
fn equal_percents_rank_by_deed_id() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"; 6])?;
    for id in 1..=6 {
        week.done_on_first(id, if id > 3 { 5 } else { 2 })?;
    }
    let ranked = ranked_deeds(&week.report()?, 2);
    assert_eq!(ids(&ranked.weakest), [1, 2]);
    assert_eq!(ids(&ranked.strongest), [4, 5]);
    Ok(())
}

#[test]
fn ranked_deeds_with_no_due_deed_are_empty() -> Result<(), Error> {
    let ranked = ranked_deeds(&Week::new(&[])?.report()?, 3);
    assert!(ranked.weakest.is_empty());
    assert!(ranked.strongest.is_empty());
    Ok(())
}

/// A prayer marked only through its on time deed.
fn on_time(id: u64) -> PrayerSource {
    PrayerSource {
        prayer: SlotKey::new("fajr"),
        on_time: Some(DeedId(id)),
        congregation: Vec::new(),
    }
}

fn sources(grid: &[PrayerRow]) -> Vec<Option<u64>> {
    grid[0]
        .days
        .iter()
        .map(|cell| cell.deed_id.map(|id| id.0))
        .collect()
}

#[test]
fn prayer_grid_shows_each_day_as_a_dot() -> Result<(), Error> {
    let mut week = Week::new(&["prayer", "prayer"])?;
    let days = Period::Week.days(day(10, 27)?, 0);
    week.mark(1, days[4], Mark::Done, days[4]);
    week.mark(1, days[5], Mark::partial_without_target(), days[5]);
    week.mark(1, days[6], Mark::Missed, days[6]);
    let grid = prayer_grid(&week.report()?, &[on_time(1)], &days[4..])?;
    assert_eq!(grid.len(), 1);
    assert_eq!(grid[0].prayer, SlotKey::new("fajr"));
    let dots: Vec<_> = grid[0].days.iter().map(|cell| cell.dot).collect();
    assert_eq!(dots, [DayDot::Done, DayDot::Partial, DayDot::Missed]);
    assert_eq!(grid[0].days[0].day, days[4]);
    Ok(())
}

#[test]
fn prayer_grid_scores_only_the_days_it_shows() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"])?;
    week.done_on_first(1, 5)?;
    let days = Period::Week.days(day(10, 27)?, 0);
    let grid = prayer_grid(&week.report()?, &[on_time(1)], &days)?;
    assert_eq!(grid[0].score.due, 7);
    assert_eq!(grid[0].score.percent, Some(71));
    let grid = prayer_grid(&week.report()?, &[on_time(1)], &days[..5])?;
    assert_eq!(grid[0].score.percent, Some(100));
    Ok(())
}

#[test]
fn prayer_grid_counts_unmarked_days_among_misses() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"])?;
    let days = [day(10, 24)?, day(10, 25)?, day(10, 26)?];
    for marked in &days[..2] {
        week.mark(1, *marked, Mark::Missed, *marked);
    }

    let grid = prayer_grid(&week.report()?, &[on_time(1)], &days)?;
    let row = &grid[0];
    assert_eq!(
        row.tally(),
        PrayerTally {
            missed: 3,
            unmarked: 1,
            due: 3,
            ..PrayerTally::default()
        }
    );
    let unmarked: Vec<_> = row.days.iter().map(|cell| cell.unmarked).collect();
    assert_eq!(unmarked, [false, false, true]);
    Ok(())
}

#[test]
fn missed_prayers_sums_misses_across_rows() -> Result<(), Error> {
    let mut week = Week::new(&["prayer", "prayer"])?;
    let days = [day(10, 25)?, day(10, 26)?];
    week.mark(1, days[0], Mark::Done, days[0]);
    week.mark(2, days[0], Mark::Missed, days[0]);
    let prayers = [
        on_time(1),
        PrayerSource {
            prayer: SlotKey::new("dhuhr"),
            ..on_time(2)
        },
    ];
    let grid = prayer_grid(&week.report()?, &prayers, &days)?;
    assert_eq!(missed_prayers(&grid), 3);
    assert_eq!(missed_prayers(&[]), 0);
    Ok(())
}

#[test]
fn prayer_grid_keeps_the_caller_order_and_skips_unknown_deeds() -> Result<(), Error> {
    let week = Week::new(&["prayer", "prayer"])?;
    let days = Period::Week.days(day(10, 27)?, 0);
    let prayers = [
        PrayerSource {
            prayer: SlotKey::new("dhuhr"),
            ..on_time(2)
        },
        PrayerSource {
            prayer: SlotKey::new("asr"),
            ..on_time(9)
        },
        on_time(1),
    ];
    let grid = prayer_grid(&week.report()?, &prayers, &days)?;
    let order: Vec<_> = grid.iter().map(|row| row.prayer.clone()).collect();
    assert_eq!(order, [SlotKey::new("dhuhr"), SlotKey::new("fajr")]);
    Ok(())
}

#[test]
fn prayer_grid_tries_congregation_deeds_in_order() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"])?;
    week.done_on_first(1, 7)?;
    let days = Period::Week.days(day(10, 27)?, 0);
    let fajr = PrayerSource {
        prayer: SlotKey::new("fajr"),
        on_time: None,
        congregation: vec![DeedId(9), DeedId(1)],
    };
    let grid = prayer_grid(&week.report()?, &[fajr], &days)?;
    assert_eq!(sources(&grid), [Some(1); 7]);
    assert!(grid[0].days.iter().all(|cell| cell.dot == DayDot::Done));
    assert_eq!(grid[0].score.percent, Some(100));
    Ok(())
}

#[test]
fn prayer_grid_counts_praying_alone_separately_from_missing_a_prayer() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"])?;
    let days = [day(10, 24)?, day(10, 25)?, day(10, 26)?];
    week.mark(1, days[0], Mark::Missed, days[0]);
    week.mark(1, days[2], Mark::Done, days[2]);
    let fajr = PrayerSource {
        prayer: SlotKey::new("fajr"),
        on_time: None,
        congregation: vec![DeedId(1)],
    };

    let grid = prayer_grid(&week.report()?, &[fajr], &days)?;
    assert_eq!(missed_prayers(&grid), 1);
    assert_eq!(
        grid[0].tally(),
        PrayerTally {
            done: 1,
            partial: 0,
            alone: 1,
            missed: 1,
            unmarked: 1,
            due: 3,
        }
    );
    Ok(())
}

#[test]
fn prayer_grid_prefers_the_congregation_deed_when_both_are_due() -> Result<(), Error> {
    let mut week = Week::new(&["prayer", "prayer"])?;
    week.done_on_first(1, 7)?;
    let days = Period::Week.days(day(10, 27)?, 0);
    let fajr = PrayerSource {
        congregation: vec![DeedId(2)],
        ..on_time(1)
    };
    let grid = prayer_grid(&week.report()?, &[fajr], &days)?;
    assert_eq!(sources(&grid), [Some(2); 7]);
    assert_eq!(grid[0].score.percent, Some(0));
    Ok(())
}

#[test]
fn prayer_grid_uses_the_on_time_deed_on_days_the_congregation_deed_is_archived() -> Result<(), Error>
{
    let mut week = Week::new(&["prayer", "prayer"])?;
    let days = Period::Week.days(day(10, 27)?, 0);
    week.deeds[1].archive(days[3])?;
    let fajr = PrayerSource {
        congregation: vec![DeedId(2)],
        ..on_time(1)
    };
    let grid = prayer_grid(&week.report()?, &[fajr], &days)?;
    let expected: Vec<_> = (0..7).map(|at| Some(if at < 3 { 2 } else { 1 })).collect();
    assert_eq!(sources(&grid), expected);
    let congregation: Vec<_> = grid[0].days.iter().map(|cell| cell.congregation).collect();
    assert_eq!(congregation, [true, true, true, false, false, false, false]);
    Ok(())
}

#[test]
fn prayer_grid_hides_a_prayer_with_no_deed_of_either_kind() -> Result<(), Error> {
    let week = Week::new(&["prayer"])?;
    let days = Period::Week.days(day(10, 27)?, 0);
    let fajr = PrayerSource {
        prayer: SlotKey::new("fajr"),
        on_time: Some(DeedId(8)),
        congregation: vec![DeedId(9)],
    };
    assert!(prayer_grid(&week.report()?, &[fajr], &days)?.is_empty());
    Ok(())
}

#[test]
fn prayer_grid_marks_days_a_prayer_was_not_due() -> Result<(), Error> {
    let mut week = Week::new(&[])?;
    week.deeds.push(Deed::new(
        DeedId(1),
        day(9, 1)?,
        DeedRule {
            schedule: Schedule::Weekdays([Weekday::Friday].into()),
            target: None,
            category: CategoryKey::new("prayer"),
            slot: SlotKey::new("dhuhr"),
        },
    ));
    let days = Period::Week.days(day(10, 27)?, 0);
    let grid = prayer_grid(&week.report()?, &[on_time(1)], &days)?;
    let due: Vec<_> = grid[0]
        .days
        .iter()
        .filter(|cell| cell.dot != DayDot::NotDue)
        .map(|cell| cell.day.weekday())
        .collect();
    assert_eq!(due, [Weekday::Friday]);
    assert_eq!(grid[0].score.due, 1);

    let mut never = Week::new(&[])?;
    never.deeds.push(Deed::new(
        DeedId(1),
        day(10, 27)?,
        DeedRule {
            schedule: Schedule::EveryDay,
            target: None,
            category: CategoryKey::new("prayer"),
            slot: SlotKey::new("fajr"),
        },
    ));
    let grid = prayer_grid(&never.report()?, &[on_time(1)], &days)?;
    assert!(grid[0].days.iter().all(|cell| cell.dot == DayDot::NotDue));
    assert_eq!(grid[0].score.percent, None);
    assert_eq!(grid[0].tally(), PrayerTally::default());
    Ok(())
}

#[test]
fn prayer_grid_shows_late_marks_and_noted_days() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"])?;
    let days = Period::Week.days(day(10, 27)?, 0);
    week.mark(1, days[0], Mark::Done, day(10, 26)?);
    week.notes
        .insert(days[1], DayNote::new("Ill", ExcuseCredit::Full, days[1])?);
    let grid = prayer_grid(&week.report()?, &[on_time(1)], &days[..3])?;
    let cells = &grid[0].days;
    assert!(cells[0].added_later && !cells[0].noted);
    assert!(cells[1].noted && !cells[1].added_later);
    assert_eq!(cells[1].dot, DayDot::Done);
    assert!(!cells[2].noted && !cells[2].added_later);
    Ok(())
}

#[test]
fn prayer_grid_counts_an_excused_day_without_a_mark_as_done() -> Result<(), Error> {
    let mut week = Week::new(&["prayer"])?;
    let excused = day(10, 26)?;
    week.notes
        .insert(excused, DayNote::new("Ill", ExcuseCredit::Full, excused)?);

    let grid = prayer_grid(&week.report()?, &[on_time(1)], &[excused])?;
    let row = &grid[0];
    assert!(!row.days[0].unmarked);
    assert_eq!(
        row.tally(),
        PrayerTally {
            done: 1,
            due: 1,
            ..PrayerTally::default()
        }
    );
    Ok(())
}

#[test]
fn percent_change_is_the_difference_in_points() -> Result<(), Error> {
    assert_eq!(
        percent_change(&score(Some(72))?, &score(Some(66))?),
        Some(6)
    );
    assert_eq!(
        percent_change(&score(Some(40))?, &score(Some(55))?),
        Some(-15)
    );
    assert_eq!(
        percent_change(&score(Some(0))?, &score(Some(100))?),
        Some(-100)
    );
    assert_eq!(
        percent_change(&score(Some(50))?, &score(Some(50))?),
        Some(0)
    );
    Ok(())
}

#[test]
fn percent_change_needs_a_percent_in_both_periods() -> Result<(), Error> {
    assert_eq!(percent_change(&score(Some(72))?, &score(None)?), None);
    assert_eq!(percent_change(&score(None)?, &score(Some(66))?), None);
    Ok(())
}

#[test]
fn categories_are_listed_weakest_first() -> Result<(), Error> {
    let mut week = Week::new(&["quran", "prayer", "adhkar", "family"])?;
    week.done_on_first(1, 2)?;
    week.done_on_first(2, 7)?;
    week.done_on_first(3, 2)?;
    week.done_on_first(4, 5)?;
    let order: Vec<_> = categories_weakest_first(&week.report()?)
        .into_iter()
        .map(|(key, score)| (key.0, score.percent))
        .collect();
    assert_eq!(
        order,
        [
            ("adhkar".to_owned(), Some(29)),
            ("quran".to_owned(), Some(29)),
            ("family".to_owned(), Some(71)),
            ("prayer".to_owned(), Some(100)),
        ]
    );
    Ok(())
}
