#![allow(clippy::unwrap_used)]

use werdy_calendar::{
    Date, Error, LocalTime, Method, Moment, Place, default_method_for_country, fajr, werdy_day_for,
};

fn place(latitude: f64, longitude: f64, utc_offset_minutes: i32) -> Place {
    Place::new(latitude, longitude, utc_offset_minutes).unwrap()
}

fn date(month: u8, day: u8) -> Date {
    Date::new(2026, month, day).unwrap()
}

fn assert_close(actual: LocalTime, hour: u8, minute: u8) {
    let expected = u16::from(hour) * 60 + u16::from(minute);
    assert!(
        actual.minutes_since_midnight().abs_diff(expected) <= 2,
        "actual: {actual:?}, expected: {hour:02}:{minute:02}"
    );
}

#[test]
fn fajr_in_cairo_matches_reference_within_two_minutes() {
    // AlAdhan API, Egyptian General Authority of Survey, read 2026-10-27.
    // https://api.aladhan.com/v1/timings/01-01-2026?latitude=30.0444&longitude=31.2357&method=5&timezonestring=Africa%2FCairo
    // https://api.aladhan.com/v1/timings/01-04-2026?latitude=30.0444&longitude=31.2357&method=5&timezonestring=Africa%2FCairo
    // https://api.aladhan.com/v1/timings/01-07-2026?latitude=30.0444&longitude=31.2357&method=5&timezonestring=Africa%2FCairo
    // https://api.aladhan.com/v1/timings/01-10-2026?latitude=30.0444&longitude=31.2357&method=5&timezonestring=Africa%2FCairo
    // Egypt uses UTC+2 in January and April, UTC+3 in July and October.
    for (day, offset, hour, minute) in [
        (date(1, 1), 120, 5, 19),
        (date(4, 1), 120, 4, 17),
        (date(7, 1), 180, 4, 12),
        (date(10, 1), 180, 5, 22),
    ] {
        assert_close(
            fajr(
                day,
                place(30.0444, 31.2357, offset),
                Method::EgyptianGeneralAuthority,
            )
            .unwrap(),
            hour,
            minute,
        );
    }
}

#[test]
fn fajr_in_alexandria_matches_reference_within_two_minutes() {
    // AlAdhan API, Egyptian General Authority of Survey, read 2026-10-27.
    // https://api.aladhan.com/v1/timings/01-01-2026?latitude=31.2001&longitude=29.9187&method=5&timezonestring=Africa%2FCairo
    // https://api.aladhan.com/v1/timings/01-07-2026?latitude=31.2001&longitude=29.9187&method=5&timezonestring=Africa%2FCairo
    // Egypt uses UTC+2 in January and UTC+3 in July.
    assert_close(
        fajr(
            date(1, 1),
            place(31.2001, 29.9187, 120),
            Method::EgyptianGeneralAuthority,
        )
        .unwrap(),
        5,
        25,
    );
    assert_close(
        fajr(
            date(7, 1),
            place(31.2001, 29.9187, 180),
            Method::EgyptianGeneralAuthority,
        )
        .unwrap(),
        4,
        12,
    );
}

#[test]
fn fajr_near_date_line_east() {
    let time = fajr(
        date(6, 21),
        place(-18.14, 178.44, 720),
        Method::MuslimWorldLeague,
    )
    .unwrap();
    assert!((210..=390).contains(&time.minutes_since_midnight()));
}

#[test]
fn fajr_near_date_line_west() {
    for (latitude, longitude, offset) in [
        (-13.83, -171.76, 780),
        (1.87, -157.4, 840),
        (-14.28, -170.7, -660),
    ] {
        let time = fajr(
            date(6, 21),
            place(latitude, longitude, offset),
            Method::MuslimWorldLeague,
        )
        .unwrap();
        assert!(
            (210..=390).contains(&time.minutes_since_midnight()),
            "place: {latitude}, {longitude}, UTC offset: {offset}, time: {time:?}"
        );
    }
}

#[test]
fn fajr_stays_continuous_across_consecutive_days_in_cairo() {
    let cairo = place(30.0444, 31.2357, 120);
    let mut previous: Option<LocalTime> = None;
    for (month, days) in [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        .into_iter()
        .enumerate()
    {
        for day in 1..=days {
            let today = Date::new(2026, (month + 1) as u8, day).unwrap();
            let current = fajr(today, cairo, Method::EgyptianGeneralAuthority).unwrap();
            if let Some(previous) = previous {
                assert!(
                    current
                        .minutes_since_midnight()
                        .abs_diff(previous.minutes_since_midnight())
                        <= 10,
                    "date: {today:?}, previous: {previous:?}, current: {current:?}"
                );
            }
            previous = Some(current);
        }
    }
    let next_year = fajr(
        Date::new(2027, 1, 1).unwrap(),
        cairo,
        Method::EgyptianGeneralAuthority,
    )
    .unwrap();
    assert!(
        next_year
            .minutes_since_midnight()
            .abs_diff(previous.unwrap().minutes_since_midnight())
            <= 10
    );
}

#[test]
fn witr_at_one_thirty_belongs_to_the_day_before() {
    let moment = Moment::new(date(7, 2), LocalTime::new(1, 30).unwrap());
    assert_eq!(
        werdy_day_for(
            moment,
            place(30.0444, 31.2357, 180),
            Method::EgyptianGeneralAuthority
        )
        .unwrap(),
        date(7, 1)
    );
}

#[test]
fn time_after_fajr_belongs_to_the_same_day() {
    let cairo = place(30.0444, 31.2357, 120);
    let today = date(1, 1);
    let fajr_time = fajr(today, cairo, Method::EgyptianGeneralAuthority).unwrap();
    assert_eq!(
        werdy_day_for(
            Moment::new(today, fajr_time),
            cairo,
            Method::EgyptianGeneralAuthority
        )
        .unwrap(),
        today
    );
}

#[test]
fn egypt_defaults_to_egyptian_method() {
    assert_eq!(
        default_method_for_country("EG"),
        Method::EgyptianGeneralAuthority
    );
    assert_eq!(default_method_for_country("US"), Method::MuslimWorldLeague);
}

#[test]
fn invalid_latitude_is_an_error() {
    assert_eq!(Place::new(91.0, 31.2357, 120), Err(Error::InvalidLatitude));
    assert_eq!(
        Place::new(f64::NAN, 31.2357, 120),
        Err(Error::InvalidLatitude)
    );
}

#[test]
fn impossible_date_and_high_latitude_return_errors() {
    assert_eq!(Date::new(2026, 2, 29), Err(Error::InvalidDate));
    assert_eq!(
        fajr(
            date(6, 21),
            place(70.0, 0.0, 0),
            Method::EgyptianGeneralAuthority
        ),
        Err(Error::FajrDoesNotOccur)
    );
}

#[test]
fn witr_on_new_year_belongs_to_december() {
    let today = Date::new(2026, 1, 1).unwrap();
    let moment = Moment::new(today, LocalTime::new(1, 30).unwrap());
    assert_eq!(
        werdy_day_for(
            moment,
            place(30.0444, 31.2357, 120),
            Method::EgyptianGeneralAuthority
        )
        .unwrap(),
        Date::new(2025, 12, 31).unwrap()
    );
}
