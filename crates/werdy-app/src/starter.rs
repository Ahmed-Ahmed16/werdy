//! Data read at start: the order of time slots and the starter sets (ADR 0024).

use serde::Deserialize;
use werdy_core::{CategoryKey, Deed, DeedId, DeedRule, Schedule, SlotKey, Target, WerdyDay};
use werdy_store::{DeedName, OwnerData};

const TIME_SLOTS: &str = include_str!("../../../data/time-slots.json");
const FIVE_PRAYERS: &str = include_str!("../../../data/starter-sets/five-prayers.json");

/// Returns the time slot order for the sheet. If the data file is broken, the
/// list is empty and core shows every slot in key order instead.
pub fn slot_order() -> Vec<SlotKey> {
    serde_json::from_str::<Vec<String>>(TIME_SLOTS)
        .map(|keys| keys.iter().map(|key| SlotKey::new(key)).collect())
        .unwrap_or_default()
}

#[derive(Deserialize)]
struct StarterSet {
    deeds: Vec<StarterDeed>,
}

#[derive(Deserialize)]
struct StarterDeed {
    name_key: String,
    slot: String,
    category: String,
    schedule: Schedule,
    #[serde(default)]
    target: Option<Target>,
}

/// Builds first run records from the five prayers set (starter set 1),
/// with every deed created today.
pub fn five_prayers(today: WerdyDay) -> Result<OwnerData, serde_json::Error> {
    from_starter_json(FIVE_PRAYERS, today)
}

fn from_starter_json(text: &str, today: WerdyDay) -> Result<OwnerData, serde_json::Error> {
    let set: StarterSet = serde_json::from_str(text)?;
    let mut data = OwnerData::default();
    for (id, deed) in (1..).zip(set.deeds) {
        let id = DeedId(id);
        let rule = DeedRule {
            schedule: deed.schedule,
            target: deed.target,
            category: CategoryKey::new(&deed.category),
            slot: SlotKey::new(&deed.slot),
        };
        data.deeds.push(Deed::new(id, today, rule));
        data.last_deed_id = id.0;
        data.names.insert(id, DeedName::Key(deed.name_key));
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use werdy_core::{DeedId, Error, WerdyDay, daily_sheet};
    use werdy_store::DeedName;

    use super::{five_prayers, from_starter_json, slot_order};
    use crate::i18n::{Language, Texts};

    #[test]
    fn the_slot_order_lists_the_eight_slots_from_fajr_to_any_time() {
        let keys: Vec<_> = slot_order().into_iter().map(|slot| slot.0).collect();

        assert_eq!(
            keys,
            [
                "fajr", "morning", "dhuhr", "asr", "maghrib", "isha", "night", "any_time"
            ]
        );
    }

    #[test]
    fn five_prayers_fill_five_slots_in_prayer_order() -> Result<(), Box<dyn std::error::Error>> {
        let today = WerdyDay::new(2026, 10, 27)?;
        let data = five_prayers(today)?;

        let sheet = daily_sheet(
            &data.deeds,
            &data.marks,
            today,
            &slot_order(),
            data.records().calendar,
        );

        let slots: Vec<_> = sheet
            .slots
            .iter()
            .map(|slot| slot.slot.0.as_str())
            .collect();
        assert_eq!(slots, ["fajr", "dhuhr", "asr", "maghrib", "isha"]);
        assert_eq!(data.next_deed_id(), DeedId(6));
        Ok(())
    }

    #[test]
    fn every_starter_name_has_text_in_both_languages() -> Result<(), Box<dyn std::error::Error>> {
        let texts = Texts::load();
        let data = five_prayers(WerdyDay::new(2026, 10, 27)?)?;

        for name in data.names.values() {
            let DeedName::Key(key) = name else {
                return Err("starter names must be keys".into());
            };
            for language in [Language::Arabic, Language::English] {
                assert_ne!(texts.get(language, key, &[]), *key, "{key} has no text");
            }
        }
        Ok(())
    }

    #[test]
    fn a_broken_starter_file_is_an_error() -> Result<(), Error> {
        let today = WerdyDay::new(2026, 10, 27)?;

        assert!(from_starter_json(r#"{"deeds": [{"name_key": 1}]}"#, today).is_err());
        Ok(())
    }
}
