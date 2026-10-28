//! Prayer times and werdy day boundaries from local calendar data.

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
    /// Umm al Qura.
    UmmAlQura,
}

/// Rule for Isha in a prayer calculation method.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IshaRule {
    /// Angle below the horizon, in degrees.
    Angle(f64),
    /// Minutes after Maghrib, outside and during Ramadan.
    MinutesAfterMaghrib { usual: u16, ramadan: u16 },
}

/// The shadow rule for Asr, in addition to the shadow at solar noon.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AsrSchool {
    /// Shadow factor 1, also used by Maliki and Hanbali.
    #[default]
    Shafi,
    /// Shadow factor 2.
    Hanafi,
}

impl AsrSchool {
    fn altitude(self, latitude: f64, declination: f64) -> Option<f64> {
        let distance = (latitude - declination).abs();
        if distance >= PI / 2.0 {
            return None;
        }
        let factor = match self {
            Self::Shafi => 1.0,
            Self::Hanafi => 2.0,
        };
        Some((1.0 / (factor + distance.tan())).atan())
    }
}

struct MethodParameters {
    fajr_angle: f64,
    isha: IshaRule,
}

const METHOD_PARAMETERS: [MethodParameters; 3] = [
    MethodParameters {
        fajr_angle: 19.5,
        isha: IshaRule::Angle(17.5),
    },
    MethodParameters {
        fajr_angle: 18.0,
        isha: IshaRule::Angle(17.0),
    },
    MethodParameters {
        fajr_angle: 18.5,
        isha: IshaRule::MinutesAfterMaghrib {
            usual: 90,
            ramadan: 120,
        },
    },
];

impl Method {
    fn parameters(self) -> &'static MethodParameters {
        match self {
            Self::EgyptianGeneralAuthority => &METHOD_PARAMETERS[0],
            Self::MuslimWorldLeague => &METHOD_PARAMETERS[1],
            Self::UmmAlQura => &METHOD_PARAMETERS[2],
        }
    }

    /// Returns Fajr angle below the horizon, in degrees.
    pub fn fajr_angle(self) -> f64 {
        self.parameters().fajr_angle
    }

    /// Returns the method's Isha rule.
    pub fn isha_rule(self) -> IshaRule {
        self.parameters().isha
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

/// A bad input or a solar event that cannot be returned on the local date.
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
    /// The sun does not rise to altitude minus 0.833 degrees on the local date.
    SunriseDoesNotOccur,
    /// Solar noon does not fall on the local date with the supplied offset.
    DhuhrDoesNotOccur,
    /// The sun does not reach the afternoon shadow altitude on the local date.
    AsrDoesNotOccur,
    /// The sun does not set to altitude minus 0.833 degrees on the local date.
    MaghribDoesNotOccur,
    /// The Isha angle or the required time after sunset does not occur on the local date.
    IshaDoesNotOccur,
}

/// Computes Fajr for a Gregorian date and a known local UTC offset.
pub fn fajr(date: Date, place: Place, method: Method) -> Result<LocalTime, Error> {
    solar_time(
        date,
        place,
        SolarEvent::Rising(-method.fajr_angle().to_radians()),
        Error::FajrDoesNotOccur,
    )
}

/// Computes sunrise at altitude minus 0.833 degrees.
/// Returns `Error::SunriseDoesNotOccur` if it is absent on the local date.
pub fn sunrise(date: Date, place: Place) -> Result<LocalTime, Error> {
    solar_time(
        date,
        place,
        SolarEvent::Rising(SUNSET_ALTITUDE),
        Error::SunriseDoesNotOccur,
    )
}

/// Computes solar noon without any added minutes.
/// Returns `Error::DhuhrDoesNotOccur` if noon is outside the local date.
pub fn dhuhr(date: Date, place: Place) -> Result<LocalTime, Error> {
    solar_time(date, place, SolarEvent::Noon, Error::DhuhrDoesNotOccur)
}

/// Computes afternoon Asr using the chosen shadow factor.
/// Returns `Error::AsrDoesNotOccur` if the altitude is not reached on the local date.
pub fn asr(date: Date, place: Place, school: AsrSchool) -> Result<LocalTime, Error> {
    solar_time(date, place, SolarEvent::Asr(school), Error::AsrDoesNotOccur)
}

/// Computes Maghrib at sunset, altitude minus 0.833 degrees.
/// Returns `Error::MaghribDoesNotOccur` if sunset is absent on the local date.
pub fn maghrib(date: Date, place: Place) -> Result<LocalTime, Error> {
    solar_time(
        date,
        place,
        SolarEvent::Setting(SUNSET_ALTITUDE, 0),
        Error::MaghribDoesNotOccur,
    )
}

/// Computes Isha outside Ramadan using the method's rule.
/// The caller chooses `isha_in_ramadan` during Ramadan. No Hijri date is inferred.
/// Returns `Error::IshaDoesNotOccur` if the required event is absent on the local date.
pub fn isha(date: Date, place: Place, method: Method) -> Result<LocalTime, Error> {
    let event = match method.isha_rule() {
        IshaRule::Angle(angle) => SolarEvent::Setting(-angle.to_radians(), 0),
        IshaRule::MinutesAfterMaghrib { usual, .. } => SolarEvent::Setting(SUNSET_ALTITUDE, usual),
    };
    solar_time(date, place, event, Error::IshaDoesNotOccur)
}

/// Computes Isha during Ramadan, as selected by the caller.
/// Angle methods are unchanged. Umm al Qura uses 120 minutes after Maghrib.
/// Returns `Error::IshaDoesNotOccur` if the required event is absent on the local date.
pub fn isha_in_ramadan(date: Date, place: Place, method: Method) -> Result<LocalTime, Error> {
    let event = match method.isha_rule() {
        IshaRule::Angle(angle) => SolarEvent::Setting(-angle.to_radians(), 0),
        IshaRule::MinutesAfterMaghrib { ramadan, .. } => {
            SolarEvent::Setting(SUNSET_ALTITUDE, ramadan)
        }
    };
    solar_time(date, place, event, Error::IshaDoesNotOccur)
}

const SUNSET_ALTITUDE: f64 = -0.833 * PI / 180.0;

enum SolarEvent {
    Rising(f64),
    Noon,
    Asr(AsrSchool),
    Setting(f64, u16),
}

impl SolarEvent {
    fn minutes_from_noon(&self, latitude: f64, declination: f64) -> Option<f64> {
        match *self {
            Self::Noon => Some(0.0),
            Self::Rising(altitude) => {
                hour_angle(latitude, declination, altitude).map(|angle| -4.0 * angle)
            }
            Self::Setting(altitude, delay) => hour_angle(latitude, declination, altitude)
                .map(|angle| 4.0 * angle + f64::from(delay)),
            Self::Asr(school) => school
                .altitude(latitude, declination)
                .and_then(|altitude| hour_angle(latitude, declination, altitude))
                .map(|angle| 4.0 * angle),
        }
    }
}

fn hour_angle(latitude: f64, declination: f64, altitude: f64) -> Option<f64> {
    let cosine = (altitude.sin() - latitude.sin() * declination.sin())
        / (latitude.cos() * declination.cos());
    if !(-1.0..=1.0).contains(&cosine) {
        return None;
    }
    Some(cosine.acos().to_degrees())
}

fn solar_time(
    date: Date,
    place: Place,
    event: SolarEvent,
    missing: Error,
) -> Result<LocalTime, Error> {
    let latitude = place.latitude.to_radians();
    // Cases: prior, same, and next solar date can map to the requested local date.
    for day_offset in -1..=1 {
        let (equation_of_time, declination) = solar_position(date, day_offset);
        let Some(minutes_from_noon) = event.minutes_from_noon(latitude, declination) else {
            continue;
        };
        let utc_minutes = 720.0 - 4.0 * place.longitude - equation_of_time + minutes_from_noon;
        let local_minutes =
            f64::from(day_offset) * 1440.0 + utc_minutes + f64::from(place.utc_offset_minutes);
        if (0.0..1440.0).contains(&local_minutes) {
            return Ok(LocalTime {
                // Cap rounded midnight at 23:59 to keep the event on this date.
                minutes_since_midnight: local_minutes.round().min(1439.0) as u16,
            });
        }
    }
    Err(missing)
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
    } else if iso_code.eq_ignore_ascii_case("SA") {
        Method::UmmAlQura
    } else {
        Method::MuslimWorldLeague
    }
}
