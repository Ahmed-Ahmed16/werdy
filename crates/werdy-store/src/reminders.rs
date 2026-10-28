use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use werdy_core::{DeedId, SlotKey};

/// Minutes since midnight, from 0 through 1439. JSON holds this number.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct ClockTime(u16);

impl ClockTime {
    pub fn new(hour: u8, minute: u8) -> Result<Self, &'static str> {
        if hour > 23 || minute > 59 {
            return Err("invalid reminder clock time");
        }
        Self::try_from(u16::from(hour) * 60 + u16::from(minute))
    }

    pub fn minutes(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for ClockTime {
    type Error = &'static str;

    fn try_from(minutes: u16) -> Result<Self, Self::Error> {
        if minutes >= 1440 {
            return Err("invalid reminder clock time");
        }
        Ok(Self(minutes))
    }
}

impl From<ClockTime> for u16 {
    fn from(time: ClockTime) -> Self {
        time.0
    }
}

/// Minutes after a prayer, from 0 through 120. JSON holds this number.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct PrayerDelay(u8);

impl TryFrom<u8> for PrayerDelay {
    type Error = &'static str;

    fn try_from(minutes: u8) -> Result<Self, Self::Error> {
        if minutes > 120 {
            return Err("prayer reminder delay exceeds 120 minutes");
        }
        Ok(Self(minutes))
    }
}

impl From<PrayerDelay> for u8 {
    fn from(delay: PrayerDelay) -> Self {
        delay.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotReminder {
    Prayer {
        #[serde(default)]
        offset_minutes: PrayerDelay,
    },
    Clock(ClockTime),
}

/// Empty maps disable reminders. Unknown deed ids are ignored by the scheduler.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReminderSettings {
    #[serde(default)]
    pub slots: BTreeMap<SlotKey, SlotReminder>,
    #[serde(default)]
    pub deeds: BTreeMap<DeedId, ClockTime>,
}
