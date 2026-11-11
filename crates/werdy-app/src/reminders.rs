//! Notification payloads and local time conversion, independent of Android.
use fluent_bundle::FluentValue;
use serde::Serialize;
use werdy_core::WerdyDay;
#[cfg(any(test, target_os = "android"))]
use werdy_reminders::Notification;
use werdy_reminders::{Plan, Target};
#[cfg(any(test, target_os = "android"))]
use werdy_store::DeedName;
use werdy_store::{ClockTime, OwnerData};

use crate::i18n::{Language, Texts};

#[derive(Debug, Serialize)]
#[cfg(any(test, target_os = "android"))]
pub struct Alarm {
    pub id: String,
    pub epoch_millis: i64,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
#[cfg(any(test, target_os = "android"))]
pub struct Payload {
    pub channel_name: String,
    pub notifications: Vec<Alarm>,
}

#[derive(Debug, Serialize)]
#[cfg(any(test, target_arch = "wasm32"))]
pub struct PushReminder {
    pub id: String,
    pub send_at_ms: i64,
    pub title: String,
    pub body: String,
    pub topic: String,
}

/// Builds a server replacement with prayer names only.
#[cfg(any(test, target_arch = "wasm32"))]
pub fn web_batch(
    owner: &OwnerData,
    plan: &Plan,
    texts: &Texts,
    epoch_at: impl Fn(WerdyDay, u16) -> Result<Option<i64>, String>,
) -> Result<Vec<PushReminder>, String> {
    let language = Language::from_code(&owner.settings.language);
    let mut batch = Vec::new();
    for notice in &plan.notifications {
        let Target::Slot(slot) = &notice.target else {
            continue;
        };
        if !matches!(
            slot.0.as_str(),
            "fajr" | "dhuhr" | "asr" | "maghrib" | "isha"
        ) {
            continue;
        }
        let Some(send_at_ms) = epoch_at(notice.date, notice.minute)? else {
            continue;
        };
        let prayer = texts.get(language, &format!("slot-{}", slot.0), &[]);
        batch.push(PushReminder {
            id: notice.id.clone(),
            send_at_ms,
            title: texts.get(language, "reminders-push-title", &[]),
            body: texts.get(
                language,
                "reminders-push-body",
                &[("prayer", FluentValue::from(prayer))],
            ),
            topic: push_topic(&notice.id),
        });
    }
    if let Some(last) = batch.last_mut() {
        let prayer = plan
            .notifications
            .iter()
            .find(|notice| notice.id == last.id)
            .and_then(|notice| match &notice.target {
                Target::Slot(slot) => Some(texts.get(language, &format!("slot-{}", slot.0), &[])),
                Target::Deed(_) => None,
            })
            .ok_or_else(|| "missing final prayer".to_owned())?;
        last.body = texts.get(
            language,
            "reminders-push-last",
            &[("prayer", FluentValue::from(prayer))],
        );
    }
    Ok(batch)
}

#[cfg(any(test, target_arch = "wasm32"))]
fn push_topic(id: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let hash = id.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    let bytes = hash.to_be_bytes();
    let mut output = String::with_capacity(11);
    for chunk in bytes.chunks(3) {
        let value = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        output.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        output.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((value >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(value & 63) as usize] as char);
        }
    }
    output
}

/// A fixed offset is minutes east of UTC. Invalid wall times and offsets are errors.
pub fn epoch_millis(date: WerdyDay, minute: u16, offset: i32) -> Result<i64, String> {
    if minute >= 1440 || !(-720..=840).contains(&offset) {
        return Err("invalid reminder time or UTC offset".to_owned());
    }
    date.days_since_unix_epoch()
        .checked_mul(86_400_000)
        .and_then(|midnight| midnight.checked_add(i64::from(minute) * 60_000))
        .and_then(|local| local.checked_sub(i64::from(offset) * 60_000))
        .ok_or_else(|| "reminder epoch is outside range".to_owned())
}

/// Resolves a wall time with the offset at its instant, not the offset at midnight.
/// Chooses one occurrence of a repeated minute. A skipped minute returns None.
pub fn epoch_with_lookup(
    date: WerdyDay,
    minute: u16,
    offset_at: impl Fn(i64) -> i32,
) -> Result<Option<i64>, String> {
    let (_, actual) = epoch_candidates(date, minute, &offset_at)?;
    let resolved = epoch_millis(date, minute, offset_at(actual / 1000))?;
    Ok((resolved == actual).then_some(actual))
}

/// Uses the start of the day when midnight is skipped, expressed with the offset
/// before the gap. Repeated midnights use the same occurrence as clock reminders.
pub fn day_start_with_lookup(
    date: WerdyDay,
    offset_at: impl Fn(i64) -> i32,
) -> Result<i64, String> {
    if let Some(midnight) = epoch_with_lookup(date, 0, &offset_at)? {
        return Ok(midnight);
    }
    let (first, actual) = epoch_candidates(date, 0, &offset_at)?;
    Ok(first.max(actual))
}

fn epoch_candidates(
    date: WerdyDay,
    minute: u16,
    offset_at: &impl Fn(i64) -> i32,
) -> Result<(i64, i64), String> {
    let local = epoch_millis(date, minute, 0)?;
    let first = epoch_millis(date, minute, offset_at(local / 1000))?;
    let actual_offset = offset_at(first / 1000);
    let actual = epoch_millis(date, minute, actual_offset)?;
    Ok((first, actual))
}

/// Omits notifications in a clock gap. Other conversion errors remain errors.
#[cfg(any(test, target_os = "android"))]
pub fn payload(
    owner: &OwnerData,
    plan: &Plan,
    texts: &Texts,
    epoch_at: impl Fn(WerdyDay, u16) -> Result<Option<i64>, String>,
) -> Result<Payload, String> {
    let language = Language::from_code(&owner.settings.language);
    let mut notifications = Vec::new();
    for notice in &plan.notifications {
        let Some(epoch_millis) = epoch_at(notice.date, notice.minute)? else {
            continue;
        };
        let (title, body) = notification_text(owner, notice, texts);
        notifications.push(Alarm {
            id: notice.id.clone(),
            epoch_millis,
            title,
            body,
        });
    }
    Ok(Payload {
        channel_name: texts.get(language, "reminders-title", &[]),
        notifications,
    })
}

#[cfg(any(test, target_os = "android"))]
fn notification_text(owner: &OwnerData, notice: &Notification, texts: &Texts) -> (String, String) {
    let language = Language::from_code(&owner.settings.language);
    let name = match &notice.target {
        Target::Slot(slot) => texts.get(language, &format!("slot-{}", slot.0), &[]),
        Target::Deed(id) => deed_name(owner, *id, texts),
    };
    let mut names: Vec<_> = notice
        .deeds
        .iter()
        .take(3)
        .map(|id| short(&deed_name(owner, *id, texts), 60))
        .collect();
    if notice.deeds.len() > 3 {
        names.push(texts.get_with_digits(
            language,
            owner.settings.digits,
            "reminders-more",
            &[("count", FluentValue::from((notice.deeds.len() - 3) as i64))],
        ));
    }
    let separator = texts.get(language, "reminders-list-separator", &[]);
    let title = texts.get(
        language,
        "reminders-notification-title",
        &[("name", FluentValue::from(short(&name, 80)))],
    );
    let body = texts.get(
        language,
        "reminders-notification-body",
        &[("deeds", FluentValue::from(names.join(&separator)))],
    );
    (title, body)
}

#[cfg(any(test, target_os = "android"))]
fn deed_name(owner: &OwnerData, id: werdy_core::DeedId, texts: &Texts) -> String {
    let language = Language::from_code(&owner.settings.language);
    match owner.names.get(&id) {
        Some(DeedName::Text(name)) => name.clone(),
        Some(DeedName::Key(key)) => {
            texts.get_with_digits(language, owner.settings.digits, key, &[])
        }
        None => texts.get(language, "deeds-name-unavailable", &[]),
    }
}

#[cfg(any(test, target_os = "android"))]
fn short(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut short: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        short.push('…');
    }
    short
}

pub fn parse_time(text: &str) -> Option<ClockTime> {
    let (hour, minute) = text.split_once(':')?;
    ClockTime::new(hour.parse().ok()?, minute.parse().ok()?).ok()
}

pub fn time_value(time: ClockTime) -> String {
    format!("{:02}:{:02}", time.minutes() / 60, time.minutes() % 60)
}

#[cfg(test)]
mod tests;

#[derive(serde::Deserialize)]
#[cfg(any(test, target_os = "android"))]
struct Reply {
    ok: Option<bool>,
    allowed: Option<bool>,
    error: Option<String>,
}

#[cfg(any(test, target_os = "android"))]
fn reply(json: &str) -> Result<Reply, String> {
    let reply: Reply = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if let Some(error) = reply.error {
        return Err(error);
    }
    Ok(reply)
}

#[cfg(any(test, target_os = "android"))]
pub fn ok_reply(json: &str) -> Result<(), String> {
    if reply(json)?.ok == Some(true) {
        Ok(())
    } else {
        Err("Android did not accept reminder operation".into())
    }
}

#[cfg(any(test, target_os = "android"))]
pub fn permission_reply(json: &str) -> Result<bool, String> {
    reply(json)?
        .allowed
        .ok_or_else(|| "Android did not return reminder permission".into())
}
