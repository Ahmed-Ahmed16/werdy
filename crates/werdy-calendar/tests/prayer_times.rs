#![allow(clippy::unwrap_used)]

use werdy_calendar::{
    AsrSchool, Date, Error, LocalTime, Method, Place, asr, dhuhr, fajr, isha, isha_in_ramadan,
    maghrib, sunrise,
};

fn assert_close(actual: LocalTime, expected: &str) {
    let (hour, minute) = expected.split_once(':').unwrap();
    let minutes = hour.parse::<u16>().unwrap() * 60 + minute.parse::<u16>().unwrap();
    assert!(
        actual.minutes_since_midnight().abs_diff(minutes) <= 2,
        "actual: {actual:?}, expected: {expected}"
    );
}

#[test]
fn cairo_prayers_match_aladhan_within_two_minutes() {
    // AlAdhan method 5, school 0, read 2026-10-28.
    // https://api.aladhan.com/v1/timings/01-01-2026?latitude=30.0444&longitude=31.2357&method=5&school=0&timezonestring=Africa%2FCairo
    // https://api.aladhan.com/v1/timings/01-04-2026?latitude=30.0444&longitude=31.2357&method=5&school=0&timezonestring=Africa%2FCairo
    // https://api.aladhan.com/v1/timings/01-07-2026?latitude=30.0444&longitude=31.2357&method=5&school=0&timezonestring=Africa%2FCairo
    // https://api.aladhan.com/v1/timings/01-10-2026?latitude=30.0444&longitude=31.2357&method=5&school=0&timezonestring=Africa%2FCairo
    // Node Intl Africa/Cairo confirms UTC+2 in January and April, UTC+3 in July and October.
    // Reference order: Fajr, sunrise, Dhuhr, Asr, Maghrib, Isha.
    for (month, offset, expected) in [
        (
            1,
            120,
            ["05:19", "06:51", "11:59", "14:47", "17:06", "18:29"],
        ),
        (
            4,
            120,
            ["04:17", "05:45", "11:59", "15:29", "18:14", "19:32"],
        ),
        (
            7,
            180,
            ["04:12", "05:57", "12:59", "16:35", "20:00", "21:33"],
        ),
        (
            10,
            180,
            ["05:22", "06:48", "12:45", "16:09", "18:41", "19:58"],
        ),
    ] {
        assert_prayers(
            Date::new(2026, month, 1).unwrap(),
            Place::new(30.0444, 31.2357, offset).unwrap(),
            Method::EgyptianGeneralAuthority,
            expected,
        );
    }
}

#[test]
fn makkah_prayers_match_aladhan_within_two_minutes() {
    // AlAdhan method 4, school 0, read 2026-10-28. Asia/Riyadh uses UTC+3 on all dates.
    // https://api.aladhan.com/v1/timings/01-01-2026?latitude=21.3891&longitude=39.8579&method=4&school=0&timezonestring=Asia%2FRiyadh
    // https://api.aladhan.com/v1/timings/01-04-2026?latitude=21.3891&longitude=39.8579&method=4&school=0&timezonestring=Asia%2FRiyadh
    // https://api.aladhan.com/v1/timings/01-07-2026?latitude=21.3891&longitude=39.8579&method=4&school=0&timezonestring=Asia%2FRiyadh
    // https://api.aladhan.com/v1/timings/01-10-2026?latitude=21.3891&longitude=39.8579&method=4&school=0&timezonestring=Asia%2FRiyadh
    for (month, expected) in [
        (1, ["05:37", "06:58", "12:24", "15:29", "17:50", "19:20"]),
        (4, ["04:57", "06:14", "12:24", "15:49", "18:35", "20:05"]),
        (7, ["04:14", "05:42", "12:24", "15:43", "19:07", "20:37"]),
        (10, ["04:56", "06:12", "12:10", "15:35", "18:08", "19:38"]),
    ] {
        assert_prayers(
            Date::new(2026, month, 1).unwrap(),
            Place::new(21.3891, 39.8579, 180).unwrap(),
            Method::UmmAlQura,
            expected,
        );
    }
}

fn assert_prayers(date: Date, place: Place, method: Method, expected: [&str; 6]) {
    let actual = [
        fajr(date, place, method),
        sunrise(date, place),
        dhuhr(date, place),
        asr(date, place, AsrSchool::default()),
        maghrib(date, place),
        isha(date, place, method),
    ];
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert_close(actual.unwrap(), expected);
    }
}

#[test]
fn hanafi_asr_matches_aladhan() {
    // Read 2026-10-28, school 1.
    // https://api.aladhan.com/v1/timings/01-07-2026?latitude=30.0444&longitude=31.2357&method=5&school=1&timezonestring=Africa%2FCairo
    let date = Date::new(2026, 7, 1).unwrap();
    let place = Place::new(30.0444, 31.2357, 180).unwrap();
    assert_close(asr(date, place, AsrSchool::Hanafi).unwrap(), "17:52");
}

#[test]
fn ramadan_isha_uses_two_hours_after_maghrib() {
    // Read 2026-10-28. The response identifies 12 Ramadan 1447.
    // https://api.aladhan.com/v1/timings/01-03-2026?latitude=21.3891&longitude=39.8579&method=4&school=0&timezonestring=Asia%2FRiyadh
    let date = Date::new(2026, 3, 1).unwrap();
    let place = Place::new(21.3891, 39.8579, 180).unwrap();
    assert_close(maghrib(date, place).unwrap(), "18:25");
    assert_close(
        isha_in_ramadan(date, place, Method::UmmAlQura).unwrap(),
        "20:25",
    );
    assert_eq!(
        isha_in_ramadan(date, place, Method::UmmAlQura)
            .unwrap()
            .minutes_since_midnight()
            - isha(date, place, Method::UmmAlQura)
                .unwrap()
                .minutes_since_midnight(),
        30
    );
    for method in [Method::EgyptianGeneralAuthority, Method::MuslimWorldLeague] {
        assert_eq!(
            isha(date, place, method),
            isha_in_ramadan(date, place, method)
        );
    }
}

#[test]
fn missing_solar_events_return_errors() {
    let date = Date::new(2026, 6, 21).unwrap();
    let place = Place::new(90.0, 0.0, 0).unwrap();
    assert_eq!(sunrise(date, place), Err(Error::SunriseDoesNotOccur));
    assert_eq!(maghrib(date, place), Err(Error::MaghribDoesNotOccur));
    assert_eq!(
        asr(date, place, AsrSchool::Shafi),
        Err(Error::AsrDoesNotOccur)
    );
    for method in [
        Method::EgyptianGeneralAuthority,
        Method::MuslimWorldLeague,
        Method::UmmAlQura,
    ] {
        assert_eq!(isha(date, place, method), Err(Error::IshaDoesNotOccur));
        assert_eq!(
            isha_in_ramadan(date, place, method),
            Err(Error::IshaDoesNotOccur)
        );
    }
    assert!(dhuhr(date, place).is_ok());
}

#[test]
fn twilight_can_be_absent_even_when_the_sun_sets() {
    let date = Date::new(2026, 6, 21).unwrap();
    let place = Place::new(55.0, 0.0, 0).unwrap();
    assert!(maghrib(date, place).is_ok());
    for method in [Method::EgyptianGeneralAuthority, Method::MuslimWorldLeague] {
        assert_eq!(isha(date, place, method), Err(Error::IshaDoesNotOccur));
    }
    assert!(isha(date, place, Method::UmmAlQura).is_ok());
}

#[test]
fn prayer_times_handle_date_line_and_supported_year_boundaries() {
    // Cases: adjacent solar dates, first supported year, last supported year.
    for year in [1, 2026, 9999] {
        for (month, day) in [(1, 1), (12, 31)] {
            let date = Date::new(year, month, day).unwrap();
            for (longitude, offset) in [(178.44, 720), (-171.76, 780), (-170.7, -660)] {
                let place = Place::new(0.0, longitude, offset).unwrap();
                let times = [
                    fajr(date, place, Method::MuslimWorldLeague).unwrap(),
                    sunrise(date, place).unwrap(),
                    dhuhr(date, place).unwrap(),
                    asr(date, place, AsrSchool::Shafi).unwrap(),
                    asr(date, place, AsrSchool::Hanafi).unwrap(),
                    maghrib(date, place).unwrap(),
                    isha(date, place, Method::MuslimWorldLeague).unwrap(),
                ];
                assert!(
                    times.windows(2).all(|pair| pair[0] < pair[1]),
                    "{date:?} {place:?}: {times:?}"
                );
            }
        }
    }
}

#[test]
fn fixed_isha_after_midnight_uses_the_adjacent_solar_date() {
    let date = Date::new(2026, 1, 1).unwrap();
    let place = Place::new(0.0, -90.0, 0).unwrap();
    let time = isha(date, place, Method::UmmAlQura).unwrap();
    assert!((60..=120).contains(&time.minutes_since_midnight()));
    assert_eq!(
        isha_in_ramadan(date, place, Method::UmmAlQura)
            .unwrap()
            .minutes_since_midnight(),
        time.minutes_since_midnight() + 30
    );
}
