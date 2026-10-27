//! Hijri dates from the tabular Islamic calendar, with the owner's one day offset.

use crate::{Error, WerdyDay};

// Days from the Islamic civil epoch, 1 Muharram 1 (Friday 16 July 622, Julian),
// to 1970-01-01. Julian day numbers: 2440588 minus 1948440.
const EPOCH_TO_UNIX_DAYS: i64 = 492_148;
const DAYS_IN_CYCLE: i64 = 10_631;
const YEARS_IN_CYCLE: i64 = 30;
const MONTHS: u8 = 12;
const LONGEST_MONTH: u8 = 30;

/// Date in the tabular Islamic calendar.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HijriDate {
    /// Hijri year.
    pub year: i32,
    /// Month from 1 (Muharram) to 12 (Dhul Hijjah).
    pub month: u8,
    /// Day of the month from 1 to 30.
    pub day: u8,
}

impl HijriDate {
    /// Returns the number of days in a month: 29 or 30.
    /// Month 12 has 30 days only in a leap year.
    pub fn month_length(year: i32, month: u8) -> Result<u8, Error> {
        match month {
            12 if is_leap_year(i64::from(year)) => Ok(30),
            12 => Ok(29),
            1..=11 if month % 2 == 1 => Ok(30),
            1..=11 => Ok(29),
            _ => Err(Error::InvalidDate),
        }
    }

    /// Converts a werdy day to its tabular Hijri date, moved by the offset.
    pub(crate) fn from_day(day: WerdyDay, offset: HijriOffset) -> Result<Self, Error> {
        let shifted = day
            .checked_sub_days(-i64::from(offset.days()))
            .ok_or(Error::InvalidDate)?;
        from_epoch_days(shifted.days_since_unix_epoch() + EPOCH_TO_UNIX_DAYS)
    }
}

// Arithmetic (tabular) Islamic calendar, the variant Microsoft calls the Kuwaiti
// algorithm (type II). Source: Wikipedia, "Tabular Islamic calendar", and Dershowitz and
// Reingold, "Calendrical Calculations". A 30 year cycle has 10631 days, and
// years 2, 5, 7, 10, 13, 16, 18, 21, 24, 26 and 29 of each cycle are leap years.
// Months alternate 30 and 29 days; a leap year adds day 30 to month 12.
fn from_epoch_days(days: i64) -> Result<HijriDate, Error> {
    let cycle = days.div_euclid(DAYS_IN_CYCLE);
    let day_in_cycle = days.rem_euclid(DAYS_IN_CYCLE);
    let year_in_cycle = (1..=YEARS_IN_CYCLE)
        .rev()
        .find(|year| days_before_year(*year) <= day_in_cycle)
        .unwrap_or(1);
    let day_in_year = day_in_cycle - days_before_year(year_in_cycle);
    let month = (1..=MONTHS)
        .rev()
        .find(|month| days_before_month(*month) <= day_in_year)
        .unwrap_or(1);
    let day = day_in_year - days_before_month(month) + 1;
    let year = cycle * YEARS_IN_CYCLE + year_in_cycle;
    Ok(HijriDate {
        year: i32::try_from(year).map_err(|_| Error::InvalidDate)?,
        month,
        day: u8::try_from(day).map_err(|_| Error::InvalidDate)?,
    })
}

/// Days in the cycle before this year, for a year from 1 to 30.
fn days_before_year(year_in_cycle: i64) -> i64 {
    354 * (year_in_cycle - 1) + (3 + 11 * year_in_cycle) / 30
}

/// Days in the year before this month, for a month from 1 to 12.
fn days_before_month(month: u8) -> i64 {
    (59 * (i64::from(month) - 1) + 1) / 2
}

fn is_leap_year(year: i64) -> bool {
    (14 + 11 * year).rem_euclid(YEARS_IN_CYCLE) < 11
}

/// Owner's correction for local moon sighting: minus one, zero or plus one day.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HijriOffset(i8);

impl HijriOffset {
    /// Creates a checked offset from minus one to plus one day.
    pub fn new(days: i8) -> Result<Self, Error> {
        if !(-1..=1).contains(&days) {
            return Err(Error::InvalidHijriOffset);
        }
        Ok(Self(days))
    }

    /// Returns the offset in days.
    pub fn days(self) -> i8 {
        self.0
    }
}

/// Owner settings that decide which calendar days a schedule includes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CalendarSettings {
    /// Days added to the tabular Hijri date. Zero by default.
    pub hijri_offset: HijriOffset,
}

/// Hijri days a deed is due: some days of one month, or of every month.
/// New ranges are data, made with `HijriRange::new`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HijriRange {
    month: Option<u8>,
    first_day: u8,
    last_day: u8,
}

impl HijriRange {
    /// Creates a checked range. `None` means every month.
    /// Day 30 only matches in months that have it.
    pub fn new(month: Option<u8>, first_day: u8, last_day: u8) -> Result<Self, Error> {
        let month_ok = month.is_none_or(|month| (1..=MONTHS).contains(&month));
        let days_ok = first_day >= 1 && first_day <= last_day && last_day <= LONGEST_MONTH;
        if !month_ok || !days_ok {
            return Err(Error::InvalidHijriRange);
        }
        Ok(Self {
            month,
            first_day,
            last_day,
        })
    }

    /// Every day of Ramadan, month 9.
    pub fn ramadan() -> Self {
        Self::fixed(Some(9), 1, LONGEST_MONTH)
    }

    /// Ramadan days 21 to 30.
    pub fn last_ten_nights() -> Self {
        Self::fixed(Some(9), 21, LONGEST_MONTH)
    }

    /// Dhul Hijjah days 1 to 10.
    pub fn first_ten_dhul_hijjah() -> Self {
        Self::fixed(Some(12), 1, 10)
    }

    /// The Day of Arafah, 9 Dhul Hijjah.
    pub fn day_of_arafah() -> Self {
        Self::fixed(Some(12), 9, 9)
    }

    /// Ashura with the day before: 9 and 10 Muharram.
    pub fn ashura() -> Self {
        Self::fixed(Some(1), 9, 10)
    }

    /// Ayyam al Bid: days 13 to 15 of every month.
    pub fn ayyam_al_bid() -> Self {
        Self::fixed(None, 13, 15)
    }

    /// Builds a range the named constructors know is valid.
    fn fixed(month: Option<u8>, first_day: u8, last_day: u8) -> Self {
        Self {
            month,
            first_day,
            last_day,
        }
    }

    /// Returns the month, or `None` for every month.
    pub fn month(self) -> Option<u8> {
        self.month
    }

    /// Returns the first and last day of the range.
    pub fn days(self) -> (u8, u8) {
        (self.first_day, self.last_day)
    }

    /// Says whether the range includes this Hijri date.
    pub fn contains(self, date: HijriDate) -> bool {
        let month_matches = self.month.is_none_or(|month| month == date.month);
        month_matches && (self.first_day..=self.last_day).contains(&date.day)
    }
}
