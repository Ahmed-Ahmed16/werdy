use werdy_core::{Error, WerdyDay};

#[test]
fn previous_steps_back_across_a_month_and_a_year() -> Result<(), Error> {
    assert_eq!(
        WerdyDay::new(2026, 3, 1)?.previous()?,
        WerdyDay::new(2026, 2, 28)?
    );
    assert_eq!(
        WerdyDay::new(2026, 1, 1)?.previous()?,
        WerdyDay::new(2025, 12, 31)?
    );
    Ok(())
}

#[test]
fn date_parts_return_year_month_and_day() -> Result<(), Error> {
    assert_eq!(WerdyDay::new(2024, 2, 29)?.date_parts(), (2024, 2, 29));
    assert_eq!(WerdyDay::new(1969, 12, 31)?.date_parts(), (1969, 12, 31));
    assert_eq!(WerdyDay::new(2026, 10, 27)?.date_parts(), (2026, 10, 27));
    Ok(())
}

#[test]
fn unix_days_count_from_the_first_of_january_1970() -> Result<(), Error> {
    assert_eq!(WerdyDay::from_unix_days(0)?, WerdyDay::new(1970, 1, 1)?);
    assert_eq!(WerdyDay::from_unix_days(-1)?, WerdyDay::new(1969, 12, 31)?);
    assert_eq!(
        WerdyDay::from_unix_days(20_388)?,
        WerdyDay::new(2025, 10, 27)?
    );
    Ok(())
}

#[test]
fn unix_days_outside_the_day_range_are_an_error() {
    assert_eq!(WerdyDay::from_unix_days(i64::MAX), Err(Error::InvalidDate));
}
