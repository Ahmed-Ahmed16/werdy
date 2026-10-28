//! Pure reminder planning. Delivery and translated text belong to the app.
#![forbid(unsafe_code)]

use werdy_calendar::{AsrSchool, Date, LocalTime, Method, Place};
use werdy_core::{DeedId, Mark, SlotKey, WerdyDay};
use werdy_store::{OwnerData, PrayerMethod, Settings, SlotReminder};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalMoment {
    pub date: WerdyDay,
    minute: u16,
}

impl LocalMoment {
    pub fn new(date: WerdyDay, hour: u8, minute: u8) -> Result<Self, Error> {
        calendar_date(date)?;
        let minute = LocalTime::new(hour, minute)?.minutes_since_midnight();
        Ok(Self { date, minute })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Target {
    Slot(SlotKey),
    Deed(DeedId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notification {
    /// Collision free key of source date and target. The app maps it to a platform id.
    pub id: String,
    /// Local civil date, which can differ from the werdy day.
    pub date: WerdyDay,
    /// Minutes since local midnight.
    pub minute: u16,
    pub werdy_day: WerdyDay,
    pub target: Target,
    /// Only deeds still due and not Done, in id order.
    pub deeds: Vec<DeedId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Plan {
    pub notifications: Vec<Notification>,
    pub prayer_slots_skipped_no_location: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Calendar(werdy_calendar::Error),
    Core(werdy_core::Error),
    InvalidSlotRule(SlotKey),
}

impl From<werdy_calendar::Error> for Error {
    fn from(error: werdy_calendar::Error) -> Self {
        Self::Calendar(error)
    }
}
impl From<werdy_core::Error> for Error {
    fn from(error: werdy_core::Error) -> Self {
        Self::Core(error)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "reminder planning failed: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Plans `days` local dates including today. Zero returns an empty plan.
/// Keeps the current minute; omits earlier minutes. Unknown or deleted deeds are ignored.
/// `offset_at` takes a date and elapsed minutes from that date's local midnight,
/// like the app's Fajr lookup. It must be deterministic and include daylight saving.
/// Without location, clock reminders use the civil day, like the app.
/// Invalid slot rules, invalid dates and failed solar calculations return errors.
/// If Fajr does not occur, the civil day is used, like the app.
pub fn schedule(
    owner: &OwnerData,
    now: LocalMoment,
    days: u8,
    offset_at: impl Fn(WerdyDay, u16) -> i32,
) -> Result<Plan, Error> {
    let mut plan = Plan::default();
    if days == 0 {
        return Ok(plan);
    }
    validate_slots(&owner.settings)?;
    let mut last = now.date;
    for _ in 1..days {
        last = last.next()?;
    }
    calendar_date(last)?;
    let planner = Planner {
        owner,
        now,
        last,
        calendar: Calendar {
            settings: &owner.settings,
            offset_at: &offset_at,
        },
    };
    plan.prayer_slots_skipped_no_location = owner.settings.location.is_none()
        && owner
            .settings
            .reminders
            .slots
            .values()
            .any(|rule| matches!(rule, SlotReminder::Prayer { .. }));
    // A prayer plus its delay can cross midnight from the previous source date.
    let mut source = now.date.previous()?;
    loop {
        plan.notifications.extend(planner.on_date(source)?);
        if source == last {
            break;
        }
        source = source.next()?;
    }
    plan.notifications.sort_by(|left, right| {
        (left.date, left.minute, &left.id).cmp(&(right.date, right.minute, &right.id))
    });
    Ok(plan)
}
fn validate_slots(settings: &Settings) -> Result<(), Error> {
    for (slot, rule) in &settings.reminders.slots {
        let valid = match rule {
            SlotReminder::Prayer { .. } => matches!(
                slot.0.as_str(),
                "fajr" | "dhuhr" | "asr" | "maghrib" | "isha"
            ),
            SlotReminder::Clock(_) => matches!(slot.0.as_str(), "morning" | "night" | "any_time"),
        };
        if !valid {
            return Err(Error::InvalidSlotRule(slot.clone()));
        }
    }
    Ok(())
}

struct Planner<'a, F> {
    owner: &'a OwnerData,
    calendar: Calendar<'a, F>,
    now: LocalMoment,
    last: WerdyDay,
}

impl<F: Fn(WerdyDay, u16) -> i32> Planner<'_, F> {
    fn on_date(&self, source: WerdyDay) -> Result<Vec<Notification>, Error> {
        let mut notifications = Vec::new();
        for (slot, rule) in &self.owner.settings.reminders.slots {
            if let Some(minute) = self.slot_minute(source, slot, rule)? {
                notifications.extend(self.candidate(source, minute, Target::Slot(slot.clone()))?);
            }
        }
        if source >= self.now.date {
            for (id, time) in &self.owner.settings.reminders.deeds {
                notifications.extend(self.candidate(source, time.minutes(), Target::Deed(*id))?);
            }
        }
        Ok(notifications)
    }

    fn slot_minute(
        &self,
        source: WerdyDay,
        slot: &SlotKey,
        rule: &SlotReminder,
    ) -> Result<Option<u16>, Error> {
        match rule {
            SlotReminder::Clock(time) => Ok((source >= self.now.date).then_some(time.minutes())),
            SlotReminder::Prayer { offset_minutes } => {
                let delay = u16::from(u8::from(*offset_minutes));
                if self.owner.settings.location.is_none() || (source < self.now.date && delay == 0)
                {
                    return Ok(None);
                }
                Ok(Some(self.calendar.prayer(source, &slot.0)? + delay))
            }
        }
    }

    fn candidate(
        &self,
        source: WerdyDay,
        minute: u16,
        target: Target,
    ) -> Result<Option<Notification>, Error> {
        let date = if minute >= 1440 {
            source.next()?
        } else {
            source
        };
        let minute = minute % 1440;
        if (date, minute) < (self.now.date, self.now.minute) || date > self.last {
            return Ok(None);
        }
        let werdy_day = self.calendar.werdy_day(date, minute)?;
        let deeds = self.pending_deeds(werdy_day, &target);
        if deeds.is_empty() {
            return Ok(None);
        }
        Ok(Some(Notification {
            id: notification_id(source, &target),
            date,
            minute,
            werdy_day,
            target,
            deeds,
        }))
    }

    fn pending_deeds(&self, day: WerdyDay, target: &Target) -> Vec<DeedId> {
        let mut deeds: Vec<_> = self
            .owner
            .deeds
            .iter()
            .filter(|deed| {
                !self.owner.deleted_deeds.contains(&deed.id)
                    && deed.is_due_on(day, &self.owner.settings.calendar)
                    && !self
                        .owner
                        .marks
                        .get(&(deed.id, day))
                        .is_some_and(|entry| entry.mark == Mark::Done)
                    && match target {
                        Target::Deed(id) => deed.id == *id,
                        Target::Slot(slot) => {
                            deed.rule_on(day).is_some_and(|rule| rule.slot == *slot)
                        }
                    }
            })
            .map(|deed| deed.id)
            .collect();
        deeds.sort();
        deeds.dedup();
        deeds
    }
}

fn notification_id(source: WerdyDay, target: &Target) -> String {
    let (year, month, day) = source.date_parts();
    let key = match target {
        Target::Slot(slot) => format!("slot:{}", slot.0),
        Target::Deed(id) => format!("deed:{}", id.0),
    };
    format!("{year:04}-{month:02}-{day:02}:{key}")
}
struct Calendar<'a, F> {
    settings: &'a Settings,
    offset_at: &'a F,
}

impl<F: Fn(WerdyDay, u16) -> i32> Calendar<'_, F> {
    fn method(&self) -> Method {
        match self.settings.prayer_method {
            PrayerMethod::EgyptianGeneralAuthority => Method::EgyptianGeneralAuthority,
            PrayerMethod::MuslimWorldLeague => Method::MuslimWorldLeague,
            PrayerMethod::UmmAlQura => Method::UmmAlQura,
        }
    }

    fn prayer(&self, day: WerdyDay, slot: &str) -> Result<u16, Error> {
        let midnight_offset = (self.offset_at)(day, 0);
        let first = self.prayer_at_offset(day, slot, midnight_offset)?;
        let actual_offset = (self.offset_at)(day, first);
        if actual_offset == midnight_offset {
            return Ok(first);
        }
        self.prayer_at_offset(day, slot, actual_offset)
    }

    fn prayer_at_offset(&self, day: WerdyDay, slot: &str, offset: i32) -> Result<u16, Error> {
        let location = self
            .settings
            .location
            .as_ref()
            .ok_or_else(|| Error::InvalidSlotRule(SlotKey::new(slot)))?;
        let (latitude, longitude) = location.coordinates();
        let place = Place::new(latitude, longitude, offset)?;
        let date = calendar_date(day)?;
        let time = match slot {
            "fajr" => werdy_calendar::fajr(date, place, self.method()),
            "dhuhr" => werdy_calendar::dhuhr(date, place),
            "asr" => werdy_calendar::asr(date, place, AsrSchool::default()),
            "maghrib" => werdy_calendar::maghrib(date, place),
            "isha" => {
                let hijri = day.hijri(self.settings.calendar.offset_on(day))?;
                if hijri.month == 9 {
                    werdy_calendar::isha_in_ramadan(date, place, self.method())
                } else {
                    werdy_calendar::isha(date, place, self.method())
                }
            }
            _ => return Err(Error::InvalidSlotRule(SlotKey::new(slot))),
        }?;
        Ok(time.minutes_since_midnight())
    }

    fn werdy_day(&self, date: WerdyDay, minute: u16) -> Result<WerdyDay, Error> {
        if self.settings.location.is_none() {
            return Ok(date);
        }
        let fajr = match self.prayer(date, "fajr") {
            Ok(fajr) => fajr,
            Err(Error::Calendar(werdy_calendar::Error::FajrDoesNotOccur)) => return Ok(date),
            Err(error) => return Err(error),
        };
        if minute < fajr {
            Ok(date.previous()?)
        } else {
            Ok(date)
        }
    }
}

fn calendar_date(day: WerdyDay) -> Result<Date, Error> {
    let (year, month, date) = day.date_parts();
    let year = u16::try_from(year).map_err(|_| werdy_calendar::Error::InvalidDate)?;
    Ok(Date::new(year, month, date)?)
}
