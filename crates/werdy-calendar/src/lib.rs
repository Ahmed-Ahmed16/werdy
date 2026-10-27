//! Fajr time and werdy day boundaries from local calendar data.

#![forbid(unsafe_code)]

use core::f64::consts::{PI, TAU};

/// A Gregorian date with a checked year, month and day.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

impl Date {
    /// Checks a Gregorian date from year 1 through year 9999.
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, Error> {
        if !(1..=9999).contains(&year) || day == 0 || day > days_in_month(year, month) {
            return Err(Error::InvalidDate);
        }
        Ok(Self { year, month, day })
    }

    /// Returns the Gregorian year.
    pub const fn year(self) -> u16 {
        self.year
    }

    /// Returns the month, from 1 through 12.
    pub const fn month(self) -> u8 {
        self.month
    }

    /// Returns the day of the month.
    pub const fn day(self) -> u8 {
        self.day
    }

    fn previous(self) -> Result<Self, Error> {
        if self.day > 1 {
            return Ok(Self {
                day: self.day - 1,
                ..self
            });
        }
        if self.month > 1 {
            let month = self.month - 1;
            return Ok(Self {
                month,
                day: days_in_month(self.year, month),
                ..self
            });
        }
        let year = self
            .year
            .checked_sub(1)
            .filter(|year| *year > 0)
            .ok_or(Error::DateOutOfRange)?;
        Ok(Self {
            year,
            month: 12,
            day: 31,
        })
    }

    fn day_of_year(self) -> u16 {
        let mut total = u16::from(self.day);
        for month in 1..self.month {
            total += u16::from(days_in_month(self.year, month));
        }
        total
    }
}

fn leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn days_in_year(year: u16) -> u16 {
    if leap_year(year) { 366 } else { 365 }
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// A place and its offset from UTC for the given date.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    latitude: f64,
    longitude: f64,
    utc_offset_minutes: i32,
}

impl Place {
    /// Checks coordinates and an offset from minus 12 to plus 14 hours.
    pub fn new(latitude: f64, longitude: f64, utc_offset_minutes: i32) -> Result<Self, Error> {
        if !latitude.is_finite() || !(-90.0..=90.0).contains(&latitude) {
            return Err(Error::InvalidLatitude);
        }
        if !longitude.is_finite() || !(-180.0..=180.0).contains(&longitude) {
            return Err(Error::InvalidLongitude);
        }
        if !(-720..=840).contains(&utc_offset_minutes) {
            return Err(Error::InvalidUtcOffset);
        }
        Ok(Self {
            latitude,
            longitude,
            utc_offset_minutes,
        })
    }

    /// Returns latitude in degrees north.
    pub const fn latitude(self) -> f64 {
        self.latitude
    }

    /// Returns longitude in degrees east.
    pub const fn longitude(self) -> f64 {
        self.longitude
    }

    /// Returns the local offset from UTC in minutes.
    pub const fn utc_offset_minutes(self) -> i32 {
        self.utc_offset_minutes
    }
}

/// A prayer calculation method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    /// Egyptian General Authority of Survey.
    EgyptianGeneralAuthority,
    /// Muslim World League.
    MuslimWorldLeague,
}

struct MethodParameters {
    fajr_angle: f64,
    isha_angle: f64,
}

const METHOD_PARAMETERS: [MethodParameters; 2] = [
    MethodParameters {
        fajr_angle: 19.5,
        isha_angle: 17.5,
    },
    MethodParameters {
        fajr_angle: 18.0,
        isha_angle: 17.0,
    },
];

impl Method {
    /// Returns Fajr and Isha angles below the horizon, in degrees.
    pub fn angles(self) -> (f64, f64) {
        let parameters = match self {
            Self::EgyptianGeneralAuthority => &METHOD_PARAMETERS[0],
            Self::MuslimWorldLeague => &METHOD_PARAMETERS[1],
        };
        (parameters.fajr_angle, parameters.isha_angle)
    }
}

/// A time represented as minutes since local midnight.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LocalTime {
    minutes_since_midnight: u16,
}

impl LocalTime {
    /// Checks an hour and minute within one local day.
    pub fn new(hour: u8, minute: u8) -> Result<Self, Error> {
        if hour > 23 || minute > 59 {
            return Err(Error::InvalidTime);
        }
        Ok(Self {
            minutes_since_midnight: u16::from(hour) * 60 + u16::from(minute),
        })
    }

    /// Returns whole minutes since local midnight.
    pub const fn minutes_since_midnight(self) -> u16 {
        self.minutes_since_midnight
    }
}

/// A date and time already expressed at the given place.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Moment {
    date: Date,
    time: LocalTime,
}

impl Moment {
    /// Joins a local date and time.
    pub const fn new(date: Date, time: LocalTime) -> Self {
        Self { date, time }
    }

    /// Returns the local Gregorian date.
    pub const fn date(self) -> Date {
        self.date
    }

    /// Returns the local time.
    pub const fn time(self) -> LocalTime {
        self.time
    }
}

/// A bad input or a Fajr time that cannot be returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Latitude is outside its range or is not finite.
    InvalidLatitude,
    /// Longitude is outside its range or is not finite.
    InvalidLongitude,
    /// UTC offset is outside its range.
    InvalidUtcOffset,
    /// Gregorian date is invalid.
    InvalidDate,
    /// Local time is invalid.
    InvalidTime,
    /// Previous Gregorian date is outside the supported range.
    DateOutOfRange,
    /// The sun does not cross the Fajr angle that day.
    FajrDoesNotOccur,
}

/// Computes Fajr for a Gregorian date and a known local UTC offset.
pub fn fajr(date: Date, place: Place, method: Method) -> Result<LocalTime, Error> {
    let latitude = place.latitude * PI / 180.0;
    let fajr_altitude = -method.angles().0 * PI / 180.0;
    // Cases: prior, same, and next solar date can map to the requested local date.
    for day_offset in -1..=1 {
        let (equation_of_time, declination) = solar_position(date, day_offset);
        let cosine_hour_angle = (fajr_altitude.sin() - latitude.sin() * declination.sin())
            / (latitude.cos() * declination.cos());
        if !(-1.0..=1.0).contains(&cosine_hour_angle) {
            continue;
        }
        let hour_angle = cosine_hour_angle.acos() * 180.0 / PI;
        let utc_minutes = 720.0 - 4.0 * (place.longitude + hour_angle) - equation_of_time;
        let local_minutes =
            f64::from(day_offset) * 1440.0 + utc_minutes + f64::from(place.utc_offset_minutes);
        if (0.0..1440.0).contains(&local_minutes) {
            return Ok(LocalTime {
                minutes_since_midnight: local_minutes.round().rem_euclid(1440.0) as u16,
            });
        }
    }
    Err(Error::FajrDoesNotOccur)
}

fn solar_position(date: Date, day_offset: i32) -> (f64, f64) {
    // NOAA solar position equations: https://gml.noaa.gov/grad/solcalc/solareqns.PDF
    let mut year = date.year;
    let mut day_of_year = i32::from(date.day_of_year()) + day_offset;
    if day_of_year < 1 {
        year -= 1;
        day_of_year += i32::from(days_in_year(year));
    } else if day_of_year > i32::from(days_in_year(year)) {
        year += 1;
        day_of_year -= i32::from(days_in_year(date.year));
    }
    let gamma = TAU / f64::from(days_in_year(year)) * f64::from(day_of_year - 1);
    let equation_of_time = 229.18
        * (0.000075 + 0.001868 * gamma.cos()
            - 0.032077 * gamma.sin()
            - 0.014615 * (2.0 * gamma).cos()
            - 0.040849 * (2.0 * gamma).sin());
    let declination = 0.006918 - 0.399912 * gamma.cos() + 0.070257 * gamma.sin()
        - 0.006758 * (2.0 * gamma).cos()
        + 0.000907 * (2.0 * gamma).sin()
        - 0.002697 * (3.0 * gamma).cos()
        + 0.00148 * (3.0 * gamma).sin();
    (equation_of_time, declination)
}

/// Returns the Gregorian date that owns a local moment.
pub fn werdy_day_for(moment: Moment, place: Place, method: Method) -> Result<Date, Error> {
    let fajr_time = fajr(moment.date, place, method)?;
    if moment.time < fajr_time {
        moment.date.previous()
    } else {
        Ok(moment.date)
    }
}

/// Picks the default method from a two letter country code.
pub fn default_method_for_country(iso_code: &str) -> Method {
    if iso_code.eq_ignore_ascii_case("EG") {
        Method::EgyptianGeneralAuthority
    } else {
        Method::MuslimWorldLeague
    }
}
