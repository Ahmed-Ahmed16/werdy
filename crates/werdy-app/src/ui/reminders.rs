//! Reminder controls. Saving follows the same path as other settings.
use super::context::{use_actions, use_i18n};
#[cfg(target_os = "android")]
use crate::platform;
#[cfg(target_os = "android")]
use crate::state::EditorDraft;
use crate::{
    reminders::{parse_time, time_value},
    state::AppState,
};
use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::SlotKey;
use werdy_store::{PrayerDelay, SlotReminder};

#[component]
#[cfg(target_os = "android")]
pub fn ReminderSettings() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let actions = use_actions();
    let mut allowed = use_signal(|| None::<bool>);
    use_future(move || async move {
        loop {
            match platform::reminder_permission() {
                Ok(permission) => allowed.set(Some(permission)),
                Err(error) => {
                    dioxus::prelude::warn!("Could not read notification permission: {error}")
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
    let needs_location = {
        let app = state.read();
        app.data.settings.location.is_none()
            && app
                .data
                .settings
                .reminders
                .slots
                .values()
                .any(|rule| matches!(rule, SlotReminder::Prayer { .. }))
    };
    rsx! {
        section { class: "settings-card",
            h2 { class: "slot-title", {i18n.t("reminders-title")} }
            p { class: "settings-help", {i18n.t("reminders-help")} }
            if allowed() == Some(false) {
                button { class: "primary-button", onclick: move |_| platform::request_reminder_permission(), {i18n.t("reminders-allow")} }
                p { class: "settings-help", {i18n.t("reminders-permission-help")} }
            }
            if needs_location {
                button { class: "settings-location", onclick: move |_| actions.view(|app| app.show_screen(crate::state::Screen::FirstRunLocation)), {i18n.t("reminders-location")} }
            }
            for slot in ["fajr", "dhuhr", "asr", "maghrib", "isha", "morning", "night", "any_time"] {
                SlotReminderRow { key: "{slot}", slot }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn WebReminderSettings() -> Element {
    use crate::{
        account::{
            config,
            runtime::{AccountHandle, AccountView},
        },
        platform::web_reminders::{self, Permission},
    };
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let account = use_context::<Signal<AccountView>>();
    let handle = use_context::<AccountHandle>();
    let actions = use_actions();
    let mut on = use_signal(web_reminders::enabled);
    let mut browser = use_signal(web_reminders::BrowserStatus::default);
    use_future(move || async move {
        browser.set(web_reminders::status().await);
    });
    let saved_location = state.read().data.settings.location.is_some();
    let signed_in = account.read().signed_in;
    let status = browser();
    let available = !config::VAPID_PUBLIC_KEY.is_empty()
        && signed_in
        && (on()
            || (saved_location
                && status.supported
                && (!status.ios || status.standalone)
                && status.permission != Permission::Denied));
    rsx! {
        section { class: "settings-card",
            h2 { class: "slot-title", {i18n.t("reminders-title")} }
            p { class: "settings-help", {i18n.t("reminders-push-privacy")} }
            if config::VAPID_PUBLIC_KEY.is_empty() {
                p { role: "status", {i18n.t("reminders-push-unavailable")} }
            } else if !signed_in {
                p { role: "status", {i18n.t("reminders-push-signed-out")} }
            } else if !saved_location {
                button { class: "settings-location", onclick: move |_| actions.view(|app| app.show_screen(crate::state::Screen::FirstRunLocation)), {i18n.t("reminders-location")} }
            } else if status.ios && !status.standalone {
                p { role: "status", {i18n.t("reminders-push-iphone")} }
            } else if !status.supported {
                p { role: "status", {i18n.t("reminders-push-unsupported")} }
            } else if status.permission == Permission::Denied {
                p { role: "status", {i18n.t("reminders-push-blocked")} }
            }
            if available {
                label { class: "reminder-switch",
                    input { r#type: "checkbox", checked: on(), onchange: move |event| {
                        if event.checked() {
                            let request = web_reminders::subscribe_from_tap();
                            spawn(async move {
                                match web_reminders::finish_subscribe(request).await {
                                    Ok(true) => {
                                        on.set(true);
                                        web_reminders::enqueue(&state.read().data);
                                    }
                                    Ok(false) => on.set(false),
                                    Err(error) => {
                                        on.set(false);
                                        warn!("Push subscription failed: {error}");
                                    }
                                }
                                browser.set(web_reminders::status().await);
                            });
                        } else {
                            let store = handle.0.clone();
                            spawn(async move {
                                match web_reminders::forget(&store).await {
                                    Ok(()) => on.set(false),
                                    Err(error) => {
                                        on.set(true);
                                        warn!("Push reminder removal failed: {error}");
                                    }
                                }
                            });
                        }
                    } }
                    span { {i18n.t("reminders-push-switch")} }
                }
            }
            for slot in ["fajr", "dhuhr", "asr", "maghrib", "isha"] {
                SlotReminderRow { key: "{slot}", slot }
            }
        }
    }
}

#[component]
fn SlotReminderRow(slot: &'static str) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let actions = use_actions();
    let slot_key = SlotKey::new(slot);
    let rule = state
        .read()
        .data
        .settings
        .reminders
        .slots
        .get(&slot_key)
        .cloned();
    let prayer = matches!(slot, "fajr" | "dhuhr" | "asr" | "maghrib" | "isha");
    let delay = match &rule {
        Some(SlotReminder::Prayer { offset_minutes }) => *offset_minutes,
        _ => PrayerDelay::default_for(&slot_key),
    };
    let delay_minutes = u8::from(delay);
    let time = match &rule {
        Some(SlotReminder::Clock(time)) => time_value(*time),
        _ => "09:00".into(),
    };
    let toggle_time = time.clone();
    rsx! {
        fieldset { class: "settings-row",
            legend { {i18n.t(&format!("slot-{slot}"))} }
            label { class: "reminder-switch",
                input { r#type: "checkbox", checked: rule.is_some(), onchange: move |event| {
                    let reminder = if !event.checked() { None }
                        else if prayer { Some(SlotReminder::Prayer { offset_minutes: delay }) }
                        else { parse_time(&toggle_time).map(SlotReminder::Clock) };
                    actions.change(|app| app.set_slot_reminder(SlotKey::new(slot), reminder));
                } }
                span { {i18n.t(&format!("slot-{slot}"))} }
            }
            if rule.is_some() {
                if prayer {
                    label { class: "editor-label", r#for: "reminder-delay-{slot}", {i18n.t("reminders-delay")} }
                    select { id: "reminder-delay-{slot}", class: "editor-input", value: "{delay_minutes}", onchange: move |event| {
                        if let Ok(delay) = event.value().parse::<u8>() && let Ok(delay) = PrayerDelay::try_from(delay) {
                            actions.change(|app| app.set_slot_reminder(SlotKey::new(slot), Some(SlotReminder::Prayer { offset_minutes: delay })));
                        }
                    },
                        if ![0, 5, 10, 15, 30].contains(&delay_minutes) {
                            option { value: "{delay_minutes}", {i18n.t_with("reminders-delay-minutes", &[("minutes", FluentValue::from(i64::from(delay_minutes)))])} }
                        }
                        for minutes in [0, 5, 10, 15, 30] {
                            option { value: "{minutes}", {i18n.t_with("reminders-delay-minutes", &[("minutes", FluentValue::from(minutes))])} }
                        }
                    }
                } else {
                    label { class: "editor-label", r#for: "reminder-time-{slot}", {i18n.t("reminders-time")} }
                    input { id: "reminder-time-{slot}", class: "editor-input", r#type: "time", value: "{time}", onchange: move |event| {
                        if let Some(time) = parse_time(&event.value()) {
                            actions.change(|app| app.set_slot_reminder(SlotKey::new(slot), Some(SlotReminder::Clock(time))));
                        }
                    } }
                }
            }
        }
    }
}

#[component]
#[cfg(target_os = "android")]
pub fn DeedReminder(mut draft: Signal<EditorDraft>) -> Element {
    let i18n = use_i18n();
    let reminder = draft.read().reminder;
    let time = reminder.map(time_value).unwrap_or_else(|| "09:00".into());
    rsx! {
        fieldset { class: "editor-field editor-card",
            legend { {i18n.t("reminders-title")} }
            label { class: "settings-choice",
                input { r#type: "checkbox", checked: reminder.is_some(), onchange: move |event| {
                    draft.write().reminder = if event.checked() { parse_time("09:00") } else { None };
                } }
                span { {i18n.t("reminders-deed")} }
            }
            if reminder.is_some() {
                label { class: "editor-label", r#for: "deed-reminder-time", {i18n.t("reminders-time")} }
                input { id: "deed-reminder-time", class: "editor-input", r#type: "time", value: "{time}", onchange: move |event| {
                    if let Some(time) = parse_time(&event.value()) { draft.write().reminder = Some(time); }
                } }
            }
        }
    }
}
