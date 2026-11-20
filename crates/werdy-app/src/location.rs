//! Bundled city choices and country method defaults.

use serde::Deserialize;
use werdy_store::{Location, PrayerMethod};

#[derive(Clone, Deserialize)]
pub struct City {
    pub key: String,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
struct CityFile {
    cities: Vec<City>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationView {
    Choose,
    Finding,
    Detected,
    Fallback,
    Manual,
}

pub fn first_run_location_view(
    choice: werdy_store::LocationChoice,
    finding: bool,
    saved_is_gps: Option<bool>,
) -> LocationView {
    use werdy_store::LocationChoice;
    match choice {
        LocationChoice::Manual => LocationView::Manual,
        LocationChoice::Fallback => LocationView::Fallback,
        LocationChoice::Automatic if finding => LocationView::Finding,
        LocationChoice::Undecided if saved_is_gps == Some(false) => LocationView::Manual,
        _ if saved_is_gps == Some(true) => LocationView::Detected,
        _ => LocationView::Choose,
    }
}

pub fn cities() -> Result<Vec<City>, serde_json::Error> {
    let file = include_str!("../../../data/cities.jsonc");
    let json = file.lines().skip(1).collect::<Vec<_>>().join("\n");
    serde_json::from_str::<CityFile>(&json).map(|file| file.cities)
}

pub fn method_for_country(country: &str) -> Result<PrayerMethod, serde_json::Error> {
    let table: std::collections::BTreeMap<String, PrayerMethod> =
        serde_json::from_str(include_str!("../../../data/prayer-methods.json"))?;
    Ok(table.get(country).copied().unwrap_or_default())
}

impl City {
    pub fn location(&self) -> Option<Location> {
        Location::new(
            Some(self.key.clone()),
            Some(self.country.clone()),
            self.latitude,
            self.longitude,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{cities, method_for_country};
    use crate::i18n::{Language, Texts};
    use werdy_store::PrayerMethod;

    #[test]
    fn first_run_location_shows_the_path_chosen() {
        use super::{LocationView as View, first_run_location_view};
        use werdy_store::LocationChoice as Choice;

        for (choice, finding, saved, expected) in [
            (Choice::Undecided, false, None, View::Choose),
            (Choice::Automatic, true, None, View::Finding),
            (Choice::Automatic, false, Some(true), View::Detected),
            (Choice::Fallback, false, None, View::Fallback),
            (Choice::Manual, false, None, View::Manual),
            (Choice::Automatic, false, None, View::Choose),
            (Choice::Undecided, false, Some(true), View::Detected),
            (Choice::Undecided, false, Some(false), View::Manual),
        ] {
            assert_eq!(first_run_location_view(choice, finding, saved), expected);
        }
    }

    #[test]
    fn bundled_cities_have_valid_two_decimal_coordinates() -> Result<(), Box<dyn std::error::Error>>
    {
        for city in cities()? {
            assert!(city.location().is_some(), "{}", city.key);
            assert!((city.latitude * 100.0 - (city.latitude * 100.0).round()).abs() < 0.000001);
            assert!((city.longitude * 100.0 - (city.longitude * 100.0).round()).abs() < 0.000001);
        }
        Ok(())
    }

    #[test]
    fn country_method_defaults_follow_table() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            method_for_country("EG")?,
            PrayerMethod::EgyptianGeneralAuthority
        );
        assert_eq!(method_for_country("SA")?, PrayerMethod::UmmAlQura);
        assert_eq!(method_for_country("GB")?, PrayerMethod::MuslimWorldLeague);
        Ok(())
    }

    #[test]
    fn every_city_and_country_has_text_in_both_languages() -> Result<(), Box<dyn std::error::Error>>
    {
        let texts = Texts::load();
        for city in cities()? {
            for language in [Language::Arabic, Language::English] {
                let city_key = format!("city-{}", city.key);
                let country_key = format!("country-{}", city.country);
                assert_ne!(texts.get(language, &city_key, &[]), city_key);
                assert_ne!(texts.get(language, &country_key, &[]), country_key);
            }
        }
        Ok(())
    }
}
