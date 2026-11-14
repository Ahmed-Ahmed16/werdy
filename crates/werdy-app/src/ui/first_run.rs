//! First run level and location steps, and shared starter set picker.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use std::collections::BTreeSet;
use werdy_store::{DeedName, PrayerMethod, Settings};

use super::context::{use_actions, use_i18n};
use crate::account::runtime::{AccountHandle, AccountView};
use crate::state::{AppState, EditorError, Screen};
use crate::{editor_data, location, platform};

fn fajr_clock(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn place_summary(i18n: &super::context::I18n, settings: &Settings) -> Option<String> {
    let place = settings.location.as_ref()?;
    let city = place.city_key.as_ref().map_or_else(
        || i18n.t("location-current"),
        |key| i18n.t(&format!("city-{key}")),
    );
    match platform::fajr_today(settings) {
        Ok(Some(minutes)) => Some(i18n.t_with(
            "location-fajr",
            &[
                ("city", FluentValue::from(city)),
                ("time", FluentValue::from(i18n.num(fajr_clock(minutes)))),
            ],
        )),
        _ => Some(i18n.t_with(
            "location-fajr-unknown",
            &[("city", FluentValue::from(city))],
        )),
    }
}

#[component]
fn StepIndicator(step: usize) -> Element {
    let i18n = use_i18n();
    let total = if use_context::<AccountHandle>().0.enabled {
        6
    } else {
        5
    };
    rsx! {
        div { class: "first-run-progress",
            span { class: "sr-only", {i18n.t_with("first-run-step", &[("step", FluentValue::from(step)), ("total", FluentValue::from(total))])} }
            div { class: "first-run-dots", aria_hidden: "true",
                for dot in 1..=total { span { class: if dot == step { "first-run-dot current" } else { "first-run-dot" } } }
            }
        }
    }
}

#[component]
pub fn LevelStep() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let watcher_step = use_context::<AccountHandle>().0.enabled;
    let mut open_level = use_signal(|| None::<String>);
    let mut error = use_signal(|| false);
    let selected = state.read().pending_level.clone();
    let Ok(catalog) = editor_data::catalog() else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("sets-data-error")} } };
    };
    let levels = catalog.level_sets();
    let credit = levels.first().and_then(|set| set.credit.as_deref());

    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                span { class: "rail-brand", {i18n.t("app-name")} }
                StepIndicator { step: 1 }
                h1 { {i18n.t("first-run-level-title")} }
                p { {i18n.t("first-run-level-help")} }
            }
            if error() { p { class: "editor-error", role: "alert", {i18n.t("sets-add-error")} } }
            div { class: "level-grid",
                for set in levels {
                    if let Some(level) = &set.level {
                        { let key = set.key.clone();
                          let active = selected.as_deref() == Some(key.as_str());
                          let expanded = open_level().as_deref() == Some(key.as_str());
                          let deeds = catalog.deeds_in(set);
                          rsx! {
                            article { class: if active { "level-card selected" } else { "level-card" }, key: "{key}",
                                button { class: "level-choice", aria_pressed: "{active}", onclick: move |_| {
                                    actions.change(|state| {
                                        if state.select_first_run_level(&key).is_err() { error.set(true); }
                                    });
                                },
                                    strong { {i18n.t(&level.title)} }
                                    span { class: "level-count", {i18n.t_with("first-run-level-count", &[("count", FluentValue::from(deeds.len()))])} }
                                    span { class: "level-summary", {i18n.t(&level.summary)} }
                                }
                                button { class: "text-button level-details-toggle", aria_expanded: "{expanded}", onclick: move |_| {
                                    if expanded { open_level.set(None); } else { open_level.set(Some(set.key.clone())); }
                                }, {i18n.t(if expanded { "first-run-level-hide" } else { "first-run-level-show" })} }
                                if expanded {
                                    div { class: "level-deeds",
                                        for slot in &catalog.slots {
                                            { let in_slot = deeds.iter().filter(|deed| deed.slot == *slot).collect::<Vec<_>>();
                                              if in_slot.is_empty() { rsx! {} } else { rsx! {
                                                section { class: "level-slot", key: "{slot}",
                                                    h3 { {i18n.t(&format!("slot-{slot}"))} }
                                                    ul { for deed in in_slot { li { key: "{deed.key}", {i18n.t(&deed.name)} } } }
                                                }
                                              } }
                                            }
                                        }
                                    }
                                }
                            }
                          }
                        }
                    }
                }
            }
            if let Some(credit) = credit { p { class: "level-credit", {i18n.t(credit)} } }
            footer { class: "first-run-footer",
                button { class: "primary-button", disabled: selected.is_none(), onclick: move |_| {
                    actions.change(|state| {
                        if state.continue_first_run_level().is_err() { error.set(true); }
                    });
                }, {i18n.t("first-run-level-next")} }
                button { class: "text-button", onclick: move |_| {
                    if actions.change_deed(|state| state.start_first_run_empty()).is_err() { error.set(true); }
                }, {i18n.t("first-run-level-empty")} }
                if watcher_step {
                    button { class: "text-button", onclick: move |_| {
                        if actions.change_deed(|state| state.watch_only_first_run()).is_err() { error.set(true); }
                    }, {i18n.t("first-run-watch-only")} }
                }
            }
        }
    }
}

#[component]
pub fn ExtrasStep() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let mut error = use_signal(|| false);
    let selected = state.read().pending_extra_ticks.clone();
    let level_key = state.read().pending_level.clone();
    let Ok(catalog) = editor_data::catalog() else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("sets-data-error")} } };
    };
    let Some(level_set) = level_key.as_deref().and_then(|key| catalog.set(key)) else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("sets-data-error")} } };
    };
    let Some(level) = &level_set.level else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("sets-data-error")} } };
    };
    let help = i18n.t_with(
        "first-run-extras-help",
        &[("level", FluentValue::from(i18n.t(&level.title)))],
    );

    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                span { class: "rail-brand", {i18n.t("app-name")} }
                StepIndicator { step: 2 }
                h1 { {i18n.t("first-run-extras-title")} }
                p { "{help}" }
            }
            if error() { p { class: "editor-error", role: "alert", {i18n.t("sets-add-error")} } }
            div { class: "set-grid",
                for offer in &level.offers {
                    if let Some(set) = catalog.set(&offer.set) {
                        { let key = offer.set.clone();
                          let active = selected.contains(&key);
                          let deed_names = catalog.deeds_in(set).iter().map(|deed| i18n.t(&deed.name)).collect::<Vec<_>>().join(&i18n.t("sets-separator"));
                          rsx! {
                            article { class: if active { "set-card selected extra-card" } else { "set-card extra-card" }, key: "{key}",
                                div { class: "set-copy",
                                    h2 { {i18n.t(&set.name)} }
                                    p { class: "level-count", {i18n.t_with("first-run-level-count", &[("count", FluentValue::from(set.deeds.len()))])} }
                                    p { {i18n.t(&offer.why)} }
                                    p { "{deed_names}" }
                                }
                                button { class: "set-toggle", aria_label: i18n.t_with(if active { "sets-remove-named" } else { "sets-add-named" }, &[("set", FluentValue::from(i18n.t(&set.name)))]), aria_pressed: "{active}", onclick: move |_| {
                                    actions.change(|state| {
                                        if state.toggle_first_run_extra(&key).is_err() { error.set(true); }
                                    });
                                }, {i18n.t(if active { "sets-remove" } else { "sets-add" })} }
                            }
                          }
                        }
                    }
                }
            }
            footer { class: "first-run-footer",
                button { class: "primary-button", onclick: move |_| {
                    if actions.change_deed(|state| state.continue_first_run_extras()).is_err() { error.set(true); }
                }, {i18n.t("first-run-level-next")} }
                button { class: "text-button", onclick: move |_| {
                    if actions.change_deed(|state| state.skip_first_run_extras()).is_err() { error.set(true); }
                }, {i18n.t("first-run-extras-skip")} }
                button { class: "text-button", onclick: move |_| actions.change(|state| state.back_first_run_extras()), {i18n.t("first-run-extras-back")} }
            }
        }
    }
}

#[component]
pub fn OwnDeedsStep() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let mut name = use_signal(String::new);
    let mut slot = use_signal(|| "any_time".to_owned());
    let mut error = use_signal(|| None::<EditorError>);
    let Ok(catalog) = editor_data::catalog() else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("sets-data-error")} } };
    };
    let own = state
        .read()
        .data
        .first_run
        .own_deed_ids
        .iter()
        .filter_map(|id| {
            let app = state.read();
            let name = match app.deed_name(*id)? {
                DeedName::Text(text) => text.clone(),
                DeedName::Key(key) => i18n.t(key),
            };
            Some((*id, name))
        })
        .collect::<Vec<_>>();

    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                span { class: "rail-brand", {i18n.t("app-name")} }
                StepIndicator { step: 3 }
                h1 { {i18n.t("first-run-own-title")} }
                p { {i18n.t("first-run-own-help")} }
            }
            div { class: "first-run-card",
                label { r#for: "first-run-own-name", {i18n.t("first-run-own-name")} }
                input { id: "first-run-own-name", class: "editor-input", value: "{name}", placeholder: i18n.t("first-run-own-placeholder"), oninput: move |event| name.set(event.value()) }
                div {
                    span { {i18n.t("first-run-own-slot")} }
                    div { class: "first-run-slot-chips",
                        for choice in &catalog.slots {
                            { let key = choice.clone(); let selected = slot() == key; rsx! {
                                button { class: "choice-chip", aria_pressed: "{selected}", onclick: move |_| slot.set(key.clone()), {i18n.t(&format!("slot-{choice}"))} }
                            } }
                        }
                    }
                }
                button { class: "secondary-button", disabled: name().trim().is_empty(), onclick: move |_| {
                    match actions.change_deed(|state| state.add_first_run_deed(&name(), &slot())) {
                        Ok(Some(_)) => { name.set(String::new()); error.set(None); }
                        Ok(None) => {}
                        Err(problem) => error.set(Some(problem)),
                    }
                }, {i18n.t("first-run-own-add")} }
                if let Some(problem) = error() { p { class: "editor-error", role: "alert", {i18n.t(match problem { EditorError::NameTooLong => "editor-error-name-too-long", _ => "editor-error-rule-change" })} } }
            }
            if !own.is_empty() {
                ul { class: "first-run-own-list",
                    for (id, deed_name) in own {
                        li { key: "{id:?}",
                            span { "{deed_name}" }
                            button { class: "text-button", aria_label: i18n.t_with("first-run-own-remove", &[("name", FluentValue::from(deed_name.clone()))]), onclick: move |_| {
                                if let Err(problem) = actions.change_deed(|state| state.remove_first_run_deed(id)) { error.set(Some(problem)); }
                            }, {i18n.t("sets-remove")} }
                        }
                    }
                }
            }
            footer { class: "first-run-footer",
                button { class: "primary-button", onclick: move |_| actions.change(|state| state.continue_first_run_own_deeds()), {i18n.t("first-run-level-next")} }
                if state.read().data.first_run.own_deed_ids.is_empty() {
                    button { class: "text-button", onclick: move |_| actions.change(|state| state.continue_first_run_own_deeds()), {i18n.t("first-run-extras-skip")} }
                }
                button { class: "text-button", onclick: move |_| actions.change(|state| state.back_first_run_own_deeds()), {i18n.t("first-run-extras-back")} }
            }
        }
    }
}

#[component]
pub fn SummaryStep() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let watcher_step = use_context::<AccountHandle>().0.enabled;
    let state = use_context::<Signal<AppState>>();
    let Ok(catalog) = editor_data::catalog() else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("sets-data-error")} } };
    };
    let Ok(rows) = state.read().first_run_summary_rows() else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("sets-data-error")} } };
    };
    let total = rows.iter().map(|row| row.1).sum::<usize>();
    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                span { class: "rail-brand", {i18n.t("app-name")} }
                StepIndicator { step: if watcher_step { 6 } else { 5 } }
                h1 { {i18n.t("first-run-summary-title")} }
                p { {i18n.t("first-run-summary-help")} }
            }
            div { class: "first-run-card",
                for (key, count) in rows {
                    div { class: "first-run-summary-row", key: "{key}",
                        span { {if key == "first-run-own-row" { i18n.t(&key) } else { catalog.set(&key).map_or_else(|| key.clone(), |set| i18n.t(&set.name)) }} }
                        strong { {i18n.num(count)} }
                    }
                }
                div { class: "first-run-summary-row total",
                    strong { {i18n.t("first-run-total")} }
                    strong { {i18n.num(total)} }
                }
            }
            footer { class: "first-run-footer",
                button { class: "primary-button", onclick: move |_| actions.change(|state| state.finish_first_run(Screen::Sheet)), {i18n.t("first-run-summary-open")} }
                button { class: "text-button", onclick: move |_| actions.change(|state| state.back_first_run_summary(watcher_step)), {i18n.t("first-run-extras-back")} }
            }
        }
    }
}

#[component]
pub fn WatcherStep() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let account = use_context::<Signal<AccountView>>().read().clone();
    let mut yes = use_signal(|| false);
    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                span { class: "rail-brand", {i18n.t("app-name")} }
                StepIndicator { step: 5 }
                h1 { {i18n.t("first-run-watcher-title")} }
                p { {i18n.t("first-run-watcher-help")} }
            }
            if yes() {
                section { class: "first-run-card",
                    if account.signed_in {
                        super::sharing::WatchRequestForm { standalone: true }
                    } else {
                        p { {i18n.t("sharing-signin")} }
                        super::account::SignInButton {}
                        if account.busy { p { role: "status", {i18n.t("account-signing-in")} } }
                        if let Some(error) = account.error { p { role: "status", {i18n.t(error)} } }
                    }
                }
            }
            footer { class: "first-run-footer",
                if !yes() {
                    button { class: "primary-button", onclick: move |_| yes.set(true), {i18n.t("first-run-watcher-yes")} }
                } else if account.signed_in {
                    button { class: "primary-button", onclick: move |_| actions.change(|state| state.continue_first_run_watcher()), {i18n.t("first-run-level-next")} }
                }
                button { class: "text-button", onclick: move |_| actions.change(|state| state.continue_first_run_watcher()), {i18n.t("first-run-watcher-not-now")} }
                button { class: "text-button", onclick: move |_| actions.change(|state| state.back_first_run_watcher()), {i18n.t("first-run-extras-back")} }
            }
        }
    }
}

#[component]
pub fn LocationStep() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let Ok(cities) = location::cities() else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("location-data-error")} } };
    };
    let countries: BTreeSet<_> = cities.iter().map(|city| city.country.clone()).collect();
    let mut state = use_context::<Signal<AppState>>();
    let selected = state.read().data.settings.location.clone();
    let first_run = !state.read().data.first_run_complete;
    let watcher_step = use_context::<AccountHandle>().0.enabled;
    let confirmation = place_summary(&i18n, &state.read().data.settings);

    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                span { class: "rail-brand", {i18n.t("app-name")} }
                if first_run { StepIndicator { step: 4 } }
                h1 { {i18n.t(if first_run { "first-run-title" } else { "location-manage-title" })} }
                p { {i18n.t(if first_run { "first-run-intro" } else { "location-manage-intro" })} }
            }
            section { class: "first-run-card",
                h2 { {i18n.t("first-run-location-title")} }
                p { {i18n.t("first-run-location-help")} }
                button { class: "secondary-button", disabled: state.read().location_request.is_finding(), onclick: move |_| {
                    let Some(request_id) = state.write().location_request.start() else { return; };
                    platform::request_location(
                        move |place| {
                            if !state.read().location_request.is_current(request_id) { return; }
                            if actions.change_deed(|state| state.accept_location(request_id, place)).is_err() {
                                actions.view(|state| { state.location_request.fail_to_save(request_id); });
                            }
                        },
                        move || actions.view(|state| { state.location_request.fail(request_id); }),
                    );
                }, {i18n.t("location-use-current")} }
                if state.read().location_request.is_finding() { p { role: "status", {i18n.t("location-finding")} } }
                label { r#for: "city-choice", {i18n.t("location-choose-city")} }
                select { id: "city-choice", class: "editor-input", value: selected.as_ref().and_then(|place| place.city_key.clone()).unwrap_or_default(),
                    onchange: move |event| {
                        if let Some(city) = location::cities().ok().and_then(|cities| cities.into_iter().find(|city| city.key == event.value()))
                            && let Some(place) = city.location() {
                            actions.view(|state| state.location_request.cancel());
                            if actions.change_deed(|state| state.set_location(place)).is_err() {
                                actions.view(|state| state.location_request.save_error());
                            }
                        }
                    },
                    option { value: "", {i18n.t("location-choose-placeholder")} }
                    for city in cities { option { key: "{city.key}", value: "{city.key}", {i18n.t(&format!("city-{}", city.key))} } }
                }
                if let Some(confirmation) = confirmation { p { class: "location-confirmation", "{confirmation}" } }
                if let Some(place) = selected.as_ref().filter(|place| place.city_key.is_none()) {
                    label { r#for: "gps-country", {i18n.t("location-country")} }
                    select { id: "gps-country", class: "editor-input", value: place.country.clone().unwrap_or_default(),
                        onchange: move |event| {
                            let country = event.value();
                            actions.view(|state| state.location_request.cancel());
                            if actions.change_deed(|state| {
                                let mut place = state.data.settings.location.clone().ok_or(crate::state::EditorError::RuleChange)?;
                                place.country = (!country.is_empty()).then_some(country);
                                state.set_location(place)
                            }).is_err() { actions.view(|state| state.location_request.save_error()); }
                        },
                        option { value: "", {i18n.t("location-country-unknown")} }
                        for country in countries { option { key: "{country}", value: "{country}", {i18n.t(&format!("country-{country}"))} } }
                    }
                }
                if let Some(key) = state.read().location_request.error_key() { p { class: "editor-error", role: "alert", {i18n.t(key)} } }
                if selected.is_some() {
                    label { r#for: "prayer-method", {i18n.t("location-method")} }
                    select { id: "prayer-method", class: "editor-input", value: match state.read().data.settings.prayer_method { PrayerMethod::EgyptianGeneralAuthority => "egyptian_general_authority", PrayerMethod::MuslimWorldLeague => "muslim_world_league", PrayerMethod::UmmAlQura => "umm_al_qura" },
                        onchange: move |event| {
                            let method = match event.value().as_str() { "egyptian_general_authority" => PrayerMethod::EgyptianGeneralAuthority, "umm_al_qura" => PrayerMethod::UmmAlQura, _ => PrayerMethod::MuslimWorldLeague };
                            actions.change(|state| state.set_prayer_method(method));
                        },
                        option { value: "egyptian_general_authority", {i18n.t("method-egyptian")} }
                        option { value: "muslim_world_league", {i18n.t("method-mwl")} }
                        option { value: "umm_al_qura", {i18n.t("method-umm-al-qura")} }
                    }
                }
            }
            footer { class: "first-run-footer",
                if first_run {
                    button { class: "primary-button", onclick: move |_| actions.change(|state| state.continue_first_run_location(watcher_step)), {i18n.t("first-run-level-next")} }
                    button { class: "text-button", onclick: move |_| actions.change(|state| state.skip_first_run_location(watcher_step)), {i18n.t("location-skip")} }
                    button { class: "text-button", onclick: move |_| actions.change(|state| state.back_first_run_location()), {i18n.t("first-run-extras-back")} }
                } else {
                    button { class: "primary-button", onclick: move |_| actions.view(|state| state.show_screen(state.location_return)), {i18n.t(if state.read().location_return == Screen::Settings { "settings-back-location" } else { "location-back-deeds" })} }
                    button { class: "text-button", onclick: move |_| { actions.view(|state| state.location_request.cancel()); actions.change(|state| { state.clear_location(); state.show_screen(state.location_return); }); }, {i18n.t("location-remove")} }
                }
            }
        }
    }
}

#[component]
pub fn SetPicker() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let mut error = use_signal(|| false);
    let app = state.read();
    let chosen = app.data.chosen_sets.clone();
    let confirmation = place_summary(&i18n, &app.data.settings);
    let Ok(catalog) = editor_data::catalog() else {
        return rsx! { p { class: "editor-error", {i18n.t("sets-data-error")} } };
    };

    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                h1 { {i18n.t("sets-title")} }
                p { {i18n.t("sets-help")} }
                if let Some(confirmation) = confirmation { p { class: "location-confirmation", "{confirmation}" } }
            }
            if error() { p { class: "editor-error", role: "alert", {i18n.t("sets-add-error")} } }
            div { class: "set-grid",
                for set in &catalog.sets {
                    { let key = set.key.clone(); let active = chosen.contains(&key); let contents = catalog.deeds_in(set).iter().map(|deed| i18n.t(&deed.name)).collect::<Vec<_>>().join(&i18n.t("sets-separator")); rsx! {
                        article { class: if active { "set-card selected" } else { "set-card" }, key: "{key}",
                            div { class: "set-copy",
                                h2 { {i18n.t(&set.name)} }
                                p { "{contents}" }
                                if let Some(credit) = &set.credit { p { class: "set-credit", {i18n.t(credit)} } }
                            }
                            button { class: "set-toggle", aria_label: i18n.t_with(if active { "sets-remove-named" } else { "sets-add-named" }, &[("set", FluentValue::from(i18n.t(&set.name)))]), aria_pressed: "{active}", onclick: move |_| {
                                if actions.change_deed(|state| state.toggle_set(&key)).is_err() { error.set(true); }
                            }, {i18n.t(if active { "sets-remove" } else { "sets-add" })} }
                        }
                    } }
                }
            }
            footer { class: "first-run-footer",
                button { class: "primary-button", onclick: move |_| actions.view(|state| state.show_screen(Screen::Deeds)), {i18n.t("sets-back-deeds")} }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::i18n::{Language, Texts};
    use fluent_bundle::FluentValue;
    use werdy_store::Digits;

    #[test]
    fn level_deed_count_uses_arabic_plural_and_selected_digits() {
        let texts = Texts::load();
        for (count, expected) in [
            (1, "عمل واحد"),
            (2, "عملان"),
            (3, "٣ أعمال"),
            (11, "١١ عملًا"),
            (100, "١٠٠ عمل"),
        ] {
            let rendered = texts.get_with_digits(
                Language::Arabic,
                Digits::ArabicIndic,
                "first-run-level-count",
                &[("count", FluentValue::from(count))],
            );
            assert_eq!(rendered.replace(['\u{2068}', '\u{2069}'], ""), expected);
        }
    }

    #[test]
    fn fajr_clock_formats_local_minutes() {
        assert_eq!(super::fajr_clock(5 * 60 + 7), "05:07");
    }

    #[test]
    fn set_button_names_include_each_translated_set() {
        let texts = Texts::load();
        for language in [Language::Arabic, Language::English] {
            let prayer = texts.get(language, "set-five-prayers", &[]);
            let adhkar = texts.get(language, "set-morning-evening", &[]);
            let add_prayer = texts.get(
                language,
                "sets-add-named",
                &[("set", FluentValue::from(prayer))],
            );
            let add_adhkar = texts.get(
                language,
                "sets-add-named",
                &[("set", FluentValue::from(adhkar))],
            );
            assert_ne!(add_prayer, add_adhkar);
            assert!(add_prayer.contains(&texts.get(language, "set-five-prayers", &[])));
        }
    }

    #[test]
    fn starter_set_contents_use_spaced_language_separator() {
        let texts = Texts::load();
        for (language, separator) in [(Language::Arabic, "، "), (Language::English, ", ")] {
            assert_eq!(texts.get(language, "sets-separator", &[]), separator);
        }
    }
}
