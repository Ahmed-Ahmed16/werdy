#![allow(clippy::unwrap_used)]
use werdy_calendar::{Date, Method, Place};
use werdy_core::{
    CategoryKey, Deed, DeedId, DeedRule, Mark, MarkEntry, Schedule, SlotKey, Weekday, WerdyDay,
};
use werdy_reminders::{LocalMoment, schedule};
use werdy_store::{ClockTime, Location, OwnerData, PrayerDelay, PrayerMethod, SlotReminder};

fn day() -> WerdyDay {
    WerdyDay::new(2026, 10, 28).unwrap()
}
fn owner(slot: &str) -> OwnerData {
    let mut owner = OwnerData::default();
    owner.settings.location = Location::new(None, None, 30.04, 31.24);
    owner.settings.prayer_method = PrayerMethod::EgyptianGeneralAuthority;
    for id in [1, 2] {
        owner.deeds.push(Deed::new(
            DeedId(id),
            day().previous().unwrap(),
            DeedRule {
                schedule: Schedule::EveryDay,
                target: None,
                category: CategoryKey::new("prayer"),
                slot: SlotKey::new(slot),
            },
        ));
    }
    owner
}
fn now(hour: u8, minute: u8) -> LocalMoment {
    LocalMoment::new(day(), hour, minute).unwrap()
}
fn prayer(owner: &mut OwnerData, slot: &str, delay: u8) {
    owner.settings.reminders.slots.insert(
        SlotKey::new(slot),
        SlotReminder::Prayer {
            offset_minutes: PrayerDelay::try_from(delay).unwrap(),
        },
    );
}
fn clock(owner: &mut OwnerData, slot: &str, hour: u8, minute: u8) {
    owner.settings.reminders.slots.insert(
        SlotKey::new(slot),
        SlotReminder::Clock(ClockTime::new(hour, minute).unwrap()),
    );
}

#[test]
fn cairo_asr_matches_calendar_plus_delay_and_ids_are_stable() {
    let mut owner = owner("asr");
    prayer(&mut owner, "asr", 20);
    let first = schedule(&owner, now(0, 0), 2, |_, _| 180).unwrap();
    let second = schedule(&owner, now(1, 0), 2, |_, _| 180).unwrap();
    let expected = werdy_calendar::asr(
        Date::new(2026, 10, 28).unwrap(),
        Place::new(30.04, 31.24, 180).unwrap(),
        Default::default(),
    )
    .unwrap()
    .minutes_since_midnight()
        + 20;
    assert_eq!(first.notifications.len(), 2);
    assert_eq!(first.notifications[0].minute, expected);
    assert_eq!(first.notifications[0].deeds, vec![DeedId(1), DeedId(2)]);
    assert_eq!(first, second);
    assert_ne!(first.notifications[0].id, first.notifications[1].id);
    assert!(first.notifications[0].date < first.notifications[1].date);
}

#[test]
fn done_suppresses_but_partial_and_missed_remain() {
    let mut owner = owner("asr");
    prayer(&mut owner, "asr", 0);
    for mark in [Mark::Done, Mark::partial_without_target(), Mark::Missed] {
        for id in [1, 2] {
            owner
                .marks
                .insert((DeedId(id), day()), MarkEntry::new(mark, day()));
        }
        let planned = schedule(&owner, now(0, 0), 1, |_, _| 180).unwrap();
        assert_eq!(planned.notifications.is_empty(), mark == Mark::Done);
    }
}

#[test]
fn weekday_deed_not_due_gives_none_and_unknown_deeds_are_ignored() {
    let mut owner = owner("any_time");
    owner.deeds[0]
        .change_rule(
            day(),
            DeedRule {
                schedule: Schedule::Weekdays([Weekday::Friday].into()),
                target: None,
                category: CategoryKey::new("other"),
                slot: SlotKey::new("any_time"),
            },
        )
        .unwrap();
    for id in [1, 999] {
        owner
            .settings
            .reminders
            .deeds
            .insert(DeedId(id), ClockTime::new(12, 0).unwrap());
    }
    assert!(
        schedule(&owner, now(0, 0), 1, |_, _| 180)
            .unwrap()
            .notifications
            .is_empty()
    );
}

#[test]
fn after_midnight_checks_previous_werdy_day_marks_and_slot_rule() {
    let mut owner = owner("night");
    clock(&mut owner, "night", 1, 0);
    let previous = day().previous().unwrap();
    owner
        .marks
        .insert((DeedId(1), previous), MarkEntry::new(Mark::Done, previous));
    let planned = schedule(&owner, now(0, 0), 1, |_, _| 180).unwrap();
    assert_eq!(planned.notifications[0].werdy_day, previous);
    assert_eq!(planned.notifications[0].deeds, vec![DeedId(2)]);
}

#[test]
fn no_location_skips_prayer_but_clock_and_deed_still_work() {
    let mut owner = owner("night");
    owner.settings.location = None;
    prayer(&mut owner, "asr", 0);
    clock(&mut owner, "night", 1, 0);
    owner
        .settings
        .reminders
        .deeds
        .insert(DeedId(1), ClockTime::new(12, 0).unwrap());
    let planned = schedule(&owner, now(0, 0), 1, |_, _| 180).unwrap();
    assert!(planned.prayer_slots_skipped_no_location);
    assert_eq!(planned.notifications.len(), 2);
    assert_eq!(planned.notifications[0].minute, 60);
    assert_eq!(planned.notifications[1].minute, 720);
    assert_eq!(planned.notifications[0].werdy_day, day());
}

#[test]
fn ramadan_isha_uses_owner_hijri_calendar_and_umm_al_qura_interval() {
    let mut owner = owner("isha");
    owner.settings.prayer_method = PrayerMethod::UmmAlQura;
    prayer(&mut owner, "isha", 0);
    let date = WerdyDay::new(2027, 2, 15).unwrap();
    assert_eq!(
        date.hijri(owner.settings.calendar.offset_on(date))
            .unwrap()
            .month,
        9
    );
    let planned = schedule(&owner, LocalMoment::new(date, 0, 0).unwrap(), 1, |_, _| 120).unwrap();
    let place = Place::new(30.04, 31.24, 120).unwrap();
    let calendar_date = Date::new(2027, 2, 15).unwrap();
    let expected = werdy_calendar::isha_in_ramadan(calendar_date, place, Method::UmmAlQura)
        .unwrap()
        .minutes_since_midnight();
    assert_eq!(planned.notifications[0].minute, expected);
    assert_eq!(
        expected,
        werdy_calendar::maghrib(calendar_date, place)
            .unwrap()
            .minutes_since_midnight()
            + 120
    );
}

#[test]
fn past_times_excluded_current_minute_included_and_zero_days_empty() {
    let mut owner = owner("any_time");
    for (id, hour, minute) in [(1, 11, 59), (2, 12, 0)] {
        owner
            .settings
            .reminders
            .deeds
            .insert(DeedId(id), ClockTime::new(hour, minute).unwrap());
    }
    let planned = schedule(&owner, now(12, 0), 1, |_, _| 180).unwrap();
    assert_eq!(planned.notifications.len(), 1);
    assert_eq!(planned.notifications[0].deeds, vec![DeedId(2)]);
    assert!(
        schedule(&owner, now(12, 0), 0, |_, _| 180)
            .unwrap()
            .notifications
            .is_empty()
    );
}

#[test]
fn dst_uses_offset_at_prayer_instead_of_midnight() {
    let mut owner = owner("asr");
    prayer(&mut owner, "asr", 15);
    let planned = schedule(
        &owner,
        now(0, 0),
        1,
        |_, minute| if minute == 0 { 120 } else { 180 },
    )
    .unwrap();
    let expected = werdy_calendar::asr(
        Date::new(2026, 10, 28).unwrap(),
        Place::new(30.04, 31.24, 180).unwrap(),
        Default::default(),
    )
    .unwrap()
    .minutes_since_midnight()
        + 15;
    assert_eq!(planned.notifications[0].minute, expected);
}

#[test]
fn delayed_isha_from_yesterday_crosses_midnight_and_keeps_same_id() {
    let mut owner = owner("isha");
    prayer(&mut owner, "isha", 120);
    let previous = day().previous().unwrap();
    let (year, month, date) = previous.date_parts();
    let base = werdy_calendar::isha(
        Date::new(year as u16, month, date).unwrap(),
        Place::new(30.04, 31.24, 420).unwrap(),
        Method::EgyptianGeneralAuthority,
    )
    .unwrap()
    .minutes_since_midnight();
    assert!(base + 120 >= 1440);
    let previous_plan = schedule(
        &owner,
        LocalMoment::new(previous, 0, 0).unwrap(),
        2,
        |_, _| 420,
    )
    .unwrap();
    let today_plan = schedule(&owner, now(0, 0), 1, |_, _| 420).unwrap();
    let notification = &today_plan.notifications[0];
    assert_eq!(notification.date, day());
    assert_eq!(notification.minute, (base + 120) % 1440);
    assert_eq!(notification.werdy_day, previous);
    assert!(previous_plan.notifications.contains(notification));
}

#[test]
fn dated_slot_changes_and_deleted_deeds_are_respected() {
    let mut owner = owner("night");
    clock(&mut owner, "night", 1, 0);
    owner.deeds[0]
        .change_rule(
            day(),
            DeedRule {
                schedule: Schedule::EveryDay,
                target: None,
                category: CategoryKey::new("other"),
                slot: SlotKey::new("morning"),
            },
        )
        .unwrap();
    owner.deleted_deeds.insert(DeedId(2));
    owner
        .settings
        .reminders
        .deeds
        .insert(DeedId(2), ClockTime::new(12, 0).unwrap());
    let planned = schedule(&owner, now(0, 0), 1, |_, _| 180).unwrap();
    assert_eq!(planned.notifications.len(), 1);
    assert_eq!(planned.notifications[0].deeds, vec![DeedId(1)]);
}

#[test]
fn invalid_rules_offsets_and_date_horizons_return_errors() {
    let mut owner = owner("asr");
    prayer(&mut owner, "night", 0);
    assert!(schedule(&owner, now(0, 0), 1, |_, _| 180).is_err());
    owner.settings.reminders.slots.clear();
    prayer(&mut owner, "asr", 0);
    assert!(schedule(&owner, now(0, 0), 1, |_, _| 841).is_err());
    assert!(LocalMoment::new(day(), 24, 0).is_err());
    let end = LocalMoment::new(WerdyDay::new(9999, 12, 31).unwrap(), 12, 0).unwrap();
    assert!(schedule(&owner, end, 2, |_, _| 180).is_err());
}

#[test]
fn dated_hijri_offset_changes_isha_interval_at_ramadan_boundary() {
    use werdy_core::{CalendarSettings, HijriOffset};
    let mut owner = owner("isha");
    owner.settings.prayer_method = PrayerMethod::UmmAlQura;
    owner.settings.calendar = CalendarSettings::new(HijriOffset::new(0).unwrap());
    prayer(&mut owner, "isha", 0);
    let mut date = WerdyDay::new(2027, 2, 1).unwrap();
    while date.hijri(HijriOffset::new(0).unwrap()).unwrap().month != 9 {
        date = date.next().unwrap();
    }
    let moment = LocalMoment::new(date, 0, 0).unwrap();
    let ramadan = schedule(&owner, moment, 1, |_, _| 120).unwrap();
    owner
        .settings
        .calendar
        .change_offset(date, HijriOffset::new(-1).unwrap())
        .unwrap();
    let shaban = schedule(&owner, moment, 1, |_, _| 120).unwrap();
    assert_eq!(
        ramadan.notifications[0].minute,
        shaban.notifications[0].minute + 30
    );
}

#[test]
fn london_missing_summer_prayer_keeps_clock_reminder() {
    let date = WerdyDay::new(2026, 6, 21).unwrap();
    for slot in ["fajr", "isha"] {
        let mut owner = OwnerData::default();
        owner.settings.location = Location::new(None, None, 51.5074, -0.1278);
        owner.settings.prayer_method = PrayerMethod::MuslimWorldLeague;
        owner.deeds.push(Deed::new(
            DeedId(1),
            date,
            DeedRule {
                schedule: Schedule::EveryDay,
                target: None,
                category: CategoryKey::new("other"),
                slot: SlotKey::new("any_time"),
            },
        ));
        prayer(&mut owner, slot, 0);
        clock(&mut owner, "any_time", 12, 0);
        let plan = schedule(&owner, LocalMoment::new(date, 0, 0).unwrap(), 1, |_, _| 60).unwrap();
        assert_eq!(plan.notifications.len(), 1);
        assert_eq!(
            plan.notifications[0].target,
            werdy_reminders::Target::Slot(SlotKey::new("any_time"))
        );
        assert_eq!(plan.notifications[0].minute, 720);
        assert!(!plan.prayer_slots_skipped_no_location);
    }
}
