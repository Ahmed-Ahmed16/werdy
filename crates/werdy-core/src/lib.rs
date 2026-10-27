//! Pure deed scoring for finished werdy days.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Error for an invalid calendar day or mark fraction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Calendar day is invalid or outside the supported day count.
    InvalidDate,
    /// Fraction is not a finite value from zero to one.
    FractionOutOfRange,
}

/// Calendar label for a werdy day, stored as days since 1970-01-01.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WerdyDay(i32);

impl WerdyDay {
    /// Creates a day from a checked Gregorian year, month, and day.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, Error> {
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap_year(year) => 29,
            2 => 28,
            _ => return Err(Error::InvalidDate),
        };
        if day == 0 || day > days_in_month {
            return Err(Error::InvalidDate);
        }

        // Days from civil date, after Howard Hinnant's public domain algorithm:
        // years start in March, so the leap day falls at the end of the year.
        let year = i64::from(year) - i64::from(month <= 2);
        let era = year.div_euclid(400);
        let year_in_era = year - era * 400;
        let month_from_march = i64::from(month) + if month > 2 { -3 } else { 9 };
        let day_in_year = (153 * month_from_march + 2) / 5 + i64::from(day) - 1;
        let day_in_era = year_in_era * 365 + year_in_era / 4 - year_in_era / 100 + day_in_year;
        let days = era * 146_097 + day_in_era - 719_468;
        i32::try_from(days)
            .map(Self)
            .map_err(|_| Error::InvalidDate)
    }

    /// Returns weekday in Monday through Sunday order.
    pub fn weekday(self) -> Weekday {
        match (i64::from(self.0) + 3).rem_euclid(7) {
            0 => Weekday::Monday,
            1 => Weekday::Tuesday,
            2 => Weekday::Wednesday,
            3 => Weekday::Thursday,
            4 => Weekday::Friday,
            5 => Weekday::Saturday,
            _ => Weekday::Sunday,
        }
    }

    /// Returns the next day, or an error at the day count limit.
    pub fn next(self) -> Result<Self, Error> {
        self.0.checked_add(1).map(Self).ok_or(Error::InvalidDate)
    }
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Weekday in Monday through Sunday order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

/// Rule that says which calendar days a deed is due.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Schedule {
    /// Due every day.
    EveryDay,
    /// Due on the listed weekdays.
    Weekdays(BTreeSet<Weekday>),
}

impl Schedule {
    /// Says whether the schedule includes this day.
    pub fn includes(&self, day: WerdyDay) -> bool {
        match self {
            Self::EveryDay => true,
            Self::Weekdays(weekdays) => weekdays.contains(&day.weekday()),
        }
    }
}

// Hijri date ranges belong to a later ticket.

/// Value from zero to one inclusive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fraction(f64);

impl Fraction {
    /// Creates a checked fraction.
    pub fn new(value: f64) -> Result<Self, Error> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(Error::FractionOutOfRange);
        }
        Ok(Self(value))
    }

    /// Returns the stored fraction.
    pub fn value(self) -> f64 {
        self.0
    }
}

/// Owner's record for one deed on one werdy day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    /// Deed was done in full.
    Done,
    /// Deed was done in part.
    Partial(Fraction),
    /// Deed was missed.
    Missed,
}

impl Mark {
    /// Returns the fraction earned by this mark.
    pub fn value(self) -> f64 {
        match self {
            Self::Done => 1.0,
            Self::Partial(fraction) => fraction.value(),
            Self::Missed => 0.0,
        }
    }
}

/// Stable identity for a deed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeedId(pub u64);

/// Deed with its current schedule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deed {
    /// Stable deed identity.
    pub id: DeedId,
    /// Days when this deed is due.
    pub schedule: Schedule,
}

/// Total mark fractions, due deed count, and rounded percent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Score {
    /// Sum of mark fractions for due deeds.
    pub earned: f64,
    /// Number of due deeds across the given days.
    pub due: usize,
    /// Rounded percent, or none when no deed was due.
    pub percent: Option<u8>,
}

/// Scores deeds over the supplied finished werdy days.
pub fn score(
    deeds: &[Deed],
    marks: &BTreeMap<(DeedId, WerdyDay), Mark>,
    days: &[WerdyDay],
) -> Score {
    let mut earned = 0.0;
    let mut due = 0;
    for day in days {
        for deed in deeds {
            if deed.schedule.includes(*day) {
                due += 1;
                earned += marks.get(&(deed.id, *day)).map_or(0.0, |mark| mark.value());
            }
        }
    }

    Score {
        earned,
        due,
        percent: (due > 0).then(|| rounded_percent(earned, due)),
    }
}

fn rounded_percent(earned: f64, due: usize) -> u8 {
    let percent = earned / due as f64 * 100.0;
    // A few float steps protect exact halves without changing nearby values.
    let tolerance = 4.0 * f64::EPSILON * percent;
    (percent + tolerance).round() as u8
}
