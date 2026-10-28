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
        epoch_millis(date, 210, 120)?
    );
    assert!(epoch_with_lookup(date, 150, offset).is_err());
    let autumn = |seconds| if seconds < change { 120 } else { 60 };
    assert_eq!(
        epoch_with_lookup(date, 150, autumn)?,
        epoch_millis(date, 150, 60)?
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
            epoch_millis(date, minute, 120)
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
