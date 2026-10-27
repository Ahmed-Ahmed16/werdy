use std::collections::BTreeMap;

use werdy_core::{
    CalendarSettings, CategoryKey, Deed, DeedId, DeedRule, Error, HijriDate, HijriOffset,
    HijriRange, OwnerRecords, Schedule, WerdyDay, score,
};

fn day(year: i32, month: u8, number: u8) -> Result<WerdyDay, Error> {
    WerdyDay::new(year, month, number)
}

fn hijri(year: i32, month: u8, day: u8) -> HijriDate {
    HijriDate { year, month, day }
}

fn offset(days: i8) -> Result<CalendarSettings, Error> {
    Ok(CalendarSettings {
        hijri_offset: HijriOffset::new(days)?,
    })
}

fn hijri_deed(range: HijriRange, created_on: WerdyDay) -> Deed {
    Deed::new(
        DeedId(1),
        created_on,
        DeedRule {
            schedule: Schedule::Hijri(range),
            target: None,
            category: CategoryKey::new("fasting"),
        },
    )
}

/// Returns every day from `first` for `count` days.
fn days_from(first: WerdyDay, count: usize) -> Result<Vec<WerdyDay>, Error> {
    let mut days = Vec::with_capacity(count);
    let mut current = first;
    for _ in 0..count {
        days.push(current);
        current = current.next()?;
    }
    Ok(days)
}

/// Returns the days in this list on which the deed is due.
fn due_days(
    deed: &Deed,
    days: &[WerdyDay],
    calendar: CalendarSettings,
) -> Result<Vec<WerdyDay>, Error> {
    Ok(days
        .iter()
        .copied()
        .filter(|day| deed.is_due_on(*day, calendar))
        .collect())
}

#[test]
fn owner_reference_day_with_offset_plus_one() -> Result<(), Error> {
    let reference = day(2026, 10, 27)?;

    assert_eq!(reference.hijri(HijriOffset::new(1)?)?, hijri(1448, 5, 16));
    assert_eq!(reference.hijri(HijriOffset::default())?, hijri(1448, 5, 15));
    Ok(())
}

// Reference month from hijri-calendar.com, shared by the owner on 2026-10-27:
// Jumada al Ula 1448 runs from 2026-10-12 to 2026-11-10, 30 days.
#[test]
fn owner_reference_month_with_offset_plus_one() -> Result<(), Error> {
    let plus_one = HijriOffset::new(1)?;

    assert_eq!(day(2026, 10, 12)?.hijri(plus_one)?, hijri(1448, 5, 1));
    assert_eq!(day(2026, 10, 23)?.hijri(plus_one)?, hijri(1448, 5, 12));
    assert_eq!(day(2026, 11, 10)?.hijri(plus_one)?, hijri(1448, 5, 30));
    assert_eq!(day(2026, 11, 11)?.hijri(plus_one)?, hijri(1448, 6, 1));
    Ok(())
}

#[test]
fn offset_moves_every_date_by_one_day() -> Result<(), Error> {
    for current in days_from(day(2026, 1, 1)?, 400)? {
        let next = current.next()?;
        assert_eq!(
            current.hijri(HijriOffset::new(1)?)?,
            next.hijri(HijriOffset::default())?
        );
        assert_eq!(
            next.hijri(HijriOffset::new(-1)?)?,
            current.hijri(HijriOffset::default())?
        );
    }
    Ok(())
}

#[test]
fn ramadan_deed_is_due_only_in_ramadan() -> Result<(), Error> {
    let deed = hijri_deed(HijriRange::ramadan(), day(2026, 1, 1)?);
    let year = days_from(day(2026, 6, 1)?, 400)?;

    let due = due_days(&deed, &year, CalendarSettings::default())?;

    assert_eq!(due, days_from(day(2027, 2, 8)?, 30)?);
    Ok(())
}

#[test]
fn ramadan_deed_counts_only_ramadan_days_in_a_score() -> Result<(), Error> {
    let deeds = [hijri_deed(HijriRange::ramadan(), day(2026, 1, 1)?)];
    let records = OwnerRecords {
        deeds: &deeds,
        marks: &BTreeMap::new(),
        day_notes: &BTreeMap::new(),
        calendar: CalendarSettings::default(),
    };
    let days = days_from(day(2027, 1, 1)?, 120)?;

    let result = score(&records, &days, day(2027, 6, 1)?);

    assert_eq!(result.due, 30);
    assert_eq!(result.percent, Some(0));
    Ok(())
}

#[test]
fn last_ten_nights_are_days_21_to_30() -> Result<(), Error> {
    let deed = hijri_deed(HijriRange::last_ten_nights(), day(2026, 1, 1)?);
    let ramadan = days_from(day(2027, 2, 1)?, 50)?;

    let due = due_days(&deed, &ramadan, CalendarSettings::default())?;

    assert_eq!(due, days_from(day(2027, 2, 28)?, 10)?);
    Ok(())
}

#[test]
fn ayyam_al_bid_is_13_to_15_every_month() -> Result<(), Error> {
    let deed = hijri_deed(HijriRange::ayyam_al_bid(), day(2026, 1, 1)?);
    let calendar = CalendarSettings::default();
    let days = days_from(day(2026, 6, 17)?, 354)?;

    let due = due_days(&deed, &days, calendar)?;

    assert_eq!(due.len(), 36);
    assert!(due.contains(&day(2026, 10, 25)?));
    assert!(due.contains(&day(2026, 10, 27)?));
    assert!(!due.contains(&day(2026, 10, 28)?));
    for due_day in due {
        let date = due_day.hijri(calendar.hijri_offset)?;
        assert!((13..=15).contains(&date.day));
    }
    Ok(())
}

#[test]
fn ashura_range_is_9_and_10_muharram() -> Result<(), Error> {
    let deed = hijri_deed(HijriRange::ashura(), day(2026, 1, 1)?);
    let days = days_from(day(2026, 6, 1)?, 400)?;

    let due = due_days(&deed, &days, CalendarSettings::default())?;

    assert_eq!(
        due,
        vec![
            day(2026, 6, 25)?,
            day(2026, 6, 26)?,
            day(2027, 6, 14)?,
            day(2027, 6, 15)?,
        ]
    );
    Ok(())
}

#[test]
fn other_named_ranges_cover_dhul_hijjah() -> Result<(), Error> {
    let first_ten = hijri_deed(HijriRange::first_ten_dhul_hijjah(), day(2026, 1, 1)?);
    let arafah = hijri_deed(HijriRange::day_of_arafah(), day(2026, 1, 1)?);
    let calendar = CalendarSettings::default();
    let days = days_from(day(2026, 6, 17)?, 354)?;

    let first_ten_days = due_days(&first_ten, &days, calendar)?;
    let arafah_days = due_days(&arafah, &days, calendar)?;

    assert_eq!(first_ten_days.len(), 10);
    assert_eq!(arafah_days.len(), 1);
    assert_eq!(first_ten_days.get(8), arafah_days.first());
    for due_day in first_ten_days {
        assert_eq!(due_day.hijri(calendar.hijri_offset)?.month, 12);
    }
    Ok(())
}

#[test]
fn offset_moves_a_hijri_schedule_by_one_day() -> Result<(), Error> {
    let deed = hijri_deed(HijriRange::ramadan(), day(2026, 1, 1)?);
    let days = days_from(day(2027, 1, 1)?, 120)?;

    let due = due_days(&deed, &days, offset(1)?)?;

    assert_eq!(due, days_from(day(2027, 2, 7)?, 30)?);
    Ok(())
}

#[test]
fn new_range_needs_data_only() -> Result<(), Error> {
    let first_ten_muharram = HijriRange::new(Some(1), 1, 10)?;
    let deed = hijri_deed(first_ten_muharram, day(2026, 1, 1)?);
    let days = days_from(day(2026, 6, 1)?, 60)?;

    let due = due_days(&deed, &days, CalendarSettings::default())?;

    assert_eq!(due, days_from(day(2026, 6, 17)?, 10)?);
    Ok(())
}

#[test]
fn invalid_range_is_an_error() {
    assert_eq!(
        HijriRange::new(Some(13), 1, 10),
        Err(Error::InvalidHijriRange)
    );
    assert_eq!(
        HijriRange::new(Some(0), 1, 10),
        Err(Error::InvalidHijriRange)
    );
    assert_eq!(HijriRange::new(None, 0, 10), Err(Error::InvalidHijriRange));
    assert_eq!(HijriRange::new(None, 10, 31), Err(Error::InvalidHijriRange));
    assert_eq!(HijriRange::new(None, 15, 13), Err(Error::InvalidHijriRange));
}

#[test]
fn invalid_offset_is_an_error() {
    assert_eq!(HijriOffset::new(2), Err(Error::InvalidHijriOffset));
    assert_eq!(HijriOffset::new(-2), Err(Error::InvalidHijriOffset));
    assert_eq!(HijriOffset::new(-1).map(HijriOffset::days), Ok(-1));
    assert_eq!(HijriOffset::default().days(), 0);
}

#[test]
fn hijri_days_advance_by_one_and_months_roll_over() -> Result<(), Error> {
    let no_offset = HijriOffset::default();
    for current in days_from(day(2025, 1, 1)?, 800)? {
        let today = current.hijri(no_offset)?;
        let tomorrow = current.next()?.hijri(no_offset)?;
        let length = HijriDate::month_length(today.year, today.month)?;

        if today.day < length {
            assert_eq!(tomorrow, hijri(today.year, today.month, today.day + 1));
        } else if today.month < 12 {
            assert_eq!(tomorrow, hijri(today.year, today.month + 1, 1));
        } else {
            assert_eq!(tomorrow, hijri(today.year + 1, 1, 1));
        }
    }
    Ok(())
}

#[test]
fn month_lengths_follow_the_tabular_calendar() -> Result<(), Error> {
    assert_eq!(HijriDate::month_length(1448, 1)?, 30);
    assert_eq!(HijriDate::month_length(1448, 2)?, 29);
    assert_eq!(HijriDate::month_length(1448, 9)?, 30);
    assert_eq!(HijriDate::month_length(1448, 12)?, 29);
    assert_eq!(HijriDate::month_length(1447, 12)?, 30);
    assert_eq!(HijriDate::month_length(1448, 13), Err(Error::InvalidDate));
    Ok(())
}
