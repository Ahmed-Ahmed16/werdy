use super::*;
use werdy_core::{CategoryKey, Deed, DeedId, DeedRule, Schedule, SlotKey};
use werdy_store::{Digits, SlotReminder};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn fixed_offsets_convert_local_dates_to_epoch_and_reject_invalid_inputs() -> TestResult {
    let epoch = WerdyDay::new(1970, 1, 1)?;
    for (minute, offset, expected) in [
        (0, 0, 0),
        (180, 180, 0),
        (0, 180, -10_800_000),
        (0, -60, 3_600_000),
        (1439, 0, 86_340_000),
    ] {
        assert_eq!(epoch_millis(epoch, minute, offset)?, expected);
    }
    assert_eq!(
        epoch_millis(WerdyDay::new(2024, 1, 1)?, 0, 0)?,
        1_704_067_200_000
    );
    assert!(epoch_millis(epoch, 1440, 0).is_err());
    assert!(epoch_millis(epoch, 0, 841).is_err());
    assert!(epoch_millis(epoch, 0, -721).is_err());
    Ok(())
}

#[test]
fn epoch_lookup_uses_offset_at_event_and_rejects_spring_gap() -> TestResult {
    let date = WerdyDay::new(2026, 3, 29)?;
    let change = epoch_millis(date, 120, 60)? / 1000;
    let offset = |seconds| if seconds < change { 60 } else { 120 };
    assert_eq!(
        epoch_with_lookup(date, 210, offset)?,
        Some(epoch_millis(date, 210, 120)?)
    );
    assert_eq!(epoch_with_lookup(date, 150, offset)?, None);
    let autumn = |seconds| if seconds < change { 120 } else { 60 };
    assert_eq!(
        epoch_with_lookup(date, 150, autumn)?,
        Some(epoch_millis(date, 150, 60)?)
    );
    Ok(())
}

fn owner() -> Result<OwnerData, Box<dyn std::error::Error>> {
    let mut owner = OwnerData::default();
    let day = WerdyDay::new(2026, 10, 28)?;
    for id in 1..=5 {
        owner.deeds.push(Deed::new(
            DeedId(id),
            day,
            DeedRule {
                schedule: Schedule::EveryDay,
                target: None,
                category: CategoryKey::new("other"),
                slot: SlotKey::new("morning"),
            },
        ));
        owner
            .names
            .insert(DeedId(id), DeedName::Text(format!("Read {id} pages")));
    }
    owner
        .names
        .insert(DeedId(1), DeedName::Key("slot-fajr".into()));
    owner.settings.reminders.slots.insert(
        SlotKey::new("morning"),
        SlotReminder::Clock(ClockTime::new(9, 0)?),
    );
    owner
        .settings
        .reminders
        .deeds
        .insert(DeedId(2), ClockTime::new(10, 0)?);
    Ok(owner)
}

#[test]
fn real_plan_becomes_localized_payload_with_stable_ids_and_short_pending_list() -> TestResult {
    let mut owner = owner()?;
    let now = werdy_reminders::LocalMoment::new(WerdyDay::new(2026, 10, 28)?, 8, 0)?;
    let plan = werdy_reminders::schedule(&owner, now, 2, |_, _| 120)?;
    let texts = Texts::load();
    for (language, title, body, more) in [
        ("ar", "الصباح", "بقي لك", "آخران"),
        ("en", "Morning", "Still to do", "more"),
    ] {
        owner.settings.language = language.into();
        owner.settings.digits = Digits::ArabicIndic;
        let payload = payload(&owner, &plan, &texts, |date, minute| {
            epoch_millis(date, minute, 120).map(Some)
        })?;
        assert_eq!(payload.notifications.len(), 4);
        assert_eq!(payload.notifications[0].id, plan.notifications[0].id);
        assert!(payload.notifications[0].title.contains(title));
        assert!(payload.notifications[0].body.contains(body));
        assert!(payload.notifications[0].body.contains(more));
        assert!(!payload.notifications[0].body.contains("Read 4 pages"));
        assert!(payload.notifications[1].title.contains("Read 2 pages"));
        assert!(payload.notifications[1].body.contains("Read 2 pages"));
        let json = serde_json::to_value(&payload)?;
        assert!(json["channel_name"].is_string());
        assert_eq!(json["notifications"][0]["id"], plan.notifications[0].id);
        assert_eq!(
            json["notifications"][0]["epoch_millis"],
            epoch_millis(plan.notifications[0].date, 540, 120)?
        );
        assert_eq!(
            json["notifications"][0]
                .as_object()
                .map(|entry| entry.len()),
            Some(4)
        );
    }
    Ok(())
}

#[test]
fn empty_plan_still_sends_replacement_list_and_long_unicode_names_are_shortened() -> TestResult {
    let owner = owner()?;
    let texts = Texts::load();
    let empty = payload(&owner, &Plan::default(), &texts, |_, _| {
        Err("no time expected".into())
    })?;
    assert_eq!(
        serde_json::to_value(empty)?["notifications"],
        serde_json::json!([])
    );
    assert_eq!(short("قرآن", 3), "قرآ…");
    assert_eq!(short("قرآن", 4), "قرآن");
    Ok(())
}

#[test]
fn android_replies_report_errors_and_clock_inputs_validate_before_saving() -> TestResult {
    ok_reply(r#"{"ok":true}"#)?;
    assert!(permission_reply(r#"{"allowed":true}"#)?);
    assert!(!permission_reply(r#"{"allowed":false}"#)?);
    for bad in ["not json", "{}", r#"{"error":"blocked"}"#] {
        assert!(ok_reply(bad).is_err());
        assert!(permission_reply(bad).is_err());
    }
    assert!(ok_reply(r#"{"ok":false}"#).is_err());
    assert_eq!(
        time_value(parse_time("23:59").ok_or("invalid time")?),
        "23:59"
    );
    for bad in ["24:00", "12:60", "", "12:30:00", "text"] {
        assert!(parse_time(bad).is_none());
    }
    Ok(())
}

#[test]
fn midnight_gap_skips_only_missing_clock_notification() -> TestResult {
    let owner = owner()?;
    let date = WerdyDay::new(2026, 4, 24)?;
    let transition = epoch_millis(date, 0, 120)? / 1000;
    let offset = |seconds| if seconds < transition { 120 } else { 180 };
    let plan = Plan {
        notifications: [30, 90]
            .into_iter()
            .map(|minute| Notification {
                id: format!("gap-{minute}"),
                date,
                minute,
                werdy_day: date,
                target: Target::Deed(DeedId(1)),
                deeds: vec![DeedId(1)],
            })
            .collect(),
        ..Plan::default()
    };
    let payload = payload(&owner, &plan, &Texts::load(), |date, minute| {
        epoch_with_lookup(date, minute, offset)
    })?;
    assert_eq!(payload.notifications.len(), 1);
    assert_eq!(payload.notifications[0].id, "gap-90");
    assert_eq!(
        payload.notifications[0].epoch_millis,
        epoch_millis(date, 90, 180)?
    );
    Ok(())
}

#[test]
fn day_anchor_survives_midnight_forward_and_backward_changes() -> TestResult {
    let date = WerdyDay::new(2026, 4, 24)?;
    for (before, after) in [(120, 180), (180, 120), (-180, -120), (-120, -180)] {
        let transition = epoch_millis(date, 0, before)? / 1000;
        let offset = |seconds| if seconds < transition { before } else { after };
        let expected_offset = before.min(after);
        assert_eq!(
            day_start_with_lookup(date, offset)?,
            epoch_millis(date, 0, expected_offset)?
        );
        if after > before {
            assert_eq!(epoch_with_lookup(date, 0, offset)?, None);
        } else {
            assert_eq!(
                epoch_with_lookup(date, 0, offset)?,
                Some(epoch_millis(date, 0, after)?)
            );
        }
        assert_eq!(
            day_start_with_lookup(date.previous()?, offset)?,
            epoch_millis(date.previous()?, 0, before)?
        );
        assert_eq!(
            day_start_with_lookup(date.next()?, offset)?,
            epoch_millis(date.next()?, 0, after)?
        );
    }
    // A change later in the day must not move a midnight that exists.
    let transition = epoch_millis(date, 120, 180)? / 1000;
    let offset = |seconds| if seconds < transition { 180 } else { 120 };
    assert_eq!(
        day_start_with_lookup(date, offset)?,
        epoch_millis(date, 0, 180)?
    );
    assert!(day_start_with_lookup(date, |_| 841).is_err());
    Ok(())
}

#[test]
fn all_clock_gap_notifications_still_produce_empty_replacement_and_other_errors_propagate()
-> TestResult {
    let owner = owner()?;
    let date = WerdyDay::new(2026, 4, 24)?;
    let transition = epoch_millis(date, 0, 120)? / 1000;
    let offset = |seconds| if seconds < transition { 120 } else { 180 };
    let plan = Plan {
        notifications: vec![Notification {
            id: "gap-only".into(),
            date,
            minute: 30,
            werdy_day: date,
            target: Target::Deed(DeedId(1)),
            deeds: vec![DeedId(1)],
        }],
        ..Plan::default()
    };
    let texts = Texts::load();
    let empty = payload(&owner, &plan, &texts, |date, minute| {
        epoch_with_lookup(date, minute, offset)
    })?;
    assert_eq!(
        serde_json::to_value(empty)?["notifications"],
        serde_json::json!([])
    );
    let failed = payload(&owner, &plan, &texts, |_, _| {
        Err("invalid UTC offset".into())
    });
    assert_eq!(failed.err().as_deref(), Some("invalid UTC offset"));
    Ok(())
}
