use crate::Error;

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
