#![allow(clippy::unwrap_used)]
use werdy_core::{DeedId, SlotKey};
use werdy_store::{ClockTime, OwnerData, PrayerDelay, SlotReminder, from_json, to_json};

#[test]
fn reminders_round_trip_and_old_settings_default_to_empty() {
    let mut owner = OwnerData::default();
    owner.settings.reminders.slots.insert(
        SlotKey::new("asr"),
        SlotReminder::Prayer {
            offset_minutes: PrayerDelay::try_from(20).unwrap(),
        },
    );
    owner.settings.reminders.slots.insert(
        SlotKey::new("night"),
        SlotReminder::Clock(ClockTime::new(1, 30).unwrap()),
    );
    owner
        .settings
        .reminders
        .deeds
        .insert(DeedId(42), ClockTime::new(12, 0).unwrap());
    let json = to_json(&owner).unwrap();
    assert_eq!(from_json(&json).unwrap().settings, owner.settings);
    let mut old: serde_json::Value = serde_json::from_str(&json).unwrap();
    old["settings"].as_object_mut().unwrap().remove("reminders");
    let loaded = from_json(&old.to_string()).unwrap();
    assert!(loaded.settings.reminders.slots.is_empty());
    assert!(loaded.settings.reminders.deeds.is_empty());
}

#[test]
fn invalid_reminder_values_rejected_and_prayer_delay_defaults_to_zero() {
    for json in [r#"{"clock":1440}"#, r#"{"prayer":{"offset_minutes":121}}"#] {
        assert!(serde_json::from_str::<SlotReminder>(json).is_err());
    }
    assert_eq!(
        serde_json::from_str::<SlotReminder>(r#"{"prayer":{}}"#).unwrap(),
        SlotReminder::Prayer {
            offset_minutes: PrayerDelay::default()
        }
    );
    assert!(ClockTime::new(24, 0).is_err());
    assert!(ClockTime::new(0, 60).is_err());
}
