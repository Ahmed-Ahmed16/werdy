//! Choices for the deed editor come from the checked catalog and range data.

use serde::Deserialize;
use std::sync::OnceLock;
use werdy_core::HijriRange;

#[derive(Clone, Deserialize)]
pub struct RangeChoice {
    pub key: String,
    month: Option<u8>,
    first_day: u8,
    last_day: u8,
}

impl RangeChoice {
    pub fn range(&self) -> Result<HijriRange, werdy_core::Error> {
        HijriRange::new(self.month, self.first_day, self.last_day)
    }
}

pub fn ranges() -> Result<Vec<RangeChoice>, serde_json::Error> {
    // ___ The takbir end day is 13 in the draft. Owner must confirm it.
    serde_json::from_str(include_str!("../../../data/editor-hijri-ranges.json"))
}

pub fn catalog() -> Result<&'static werdy_data::Catalog, &'static werdy_data::Error> {
    static CATALOG: OnceLock<Result<werdy_data::Catalog, werdy_data::Error>> = OnceLock::new();
    CATALOG.get_or_init(werdy_data::builtin).as_ref()
}

#[cfg(test)]
mod tests {
    #[test]
    fn checked_catalog_is_reused() -> Result<(), Box<dyn std::error::Error>> {
        let first = super::catalog()?;
        let second = super::catalog()?;
        assert!(std::ptr::eq(first, second));
        Ok(())
    }

    #[test]
    fn editor_ranges_are_valid_catalog_choices() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = werdy_data::builtin()?;
        for choice in super::ranges()? {
            assert!(catalog.hijri_ranges.contains(&choice.key));
            choice.range()?;
        }
        Ok(())
    }
}
