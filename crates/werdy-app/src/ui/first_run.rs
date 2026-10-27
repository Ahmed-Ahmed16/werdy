//! First run location and shared starter set picker.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use std::collections::BTreeSet;
use werdy_store::{PrayerMethod, Settings};

use super::context::{use_actions, use_i18n};
use crate::state::{AppState, Screen};
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
pub fn LocationStep() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let mut error = use_signal(|| None::<&'static str>);
    let Ok(cities) = location::cities() else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("location-data-error")} } };
    };
    let countries: BTreeSet<_> = cities.iter().map(|city| city.country.clone()).collect();
    let state = use_context::<Signal<AppState>>();
    let selected = state.read().data.settings.location.clone();
    let first_run = !state.read().data.first_run_complete;
    let confirmation = place_summary(&i18n, &state.read().data.settings);

    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                span { class: "rail-brand", {i18n.t("app-name")} }
                h1 { {i18n.t(if first_run { "first-run-title" } else { "location-manage-title" })} }
                p { {i18n.t(if first_run { "first-run-intro" } else { "location-manage-intro" })} }
            }
            section { class: "first-run-card",
                h2 { {i18n.t("first-run-location-title")} }
                p { {i18n.t("first-run-location-help")} }
                button { class: "secondary-button", onclick: move |_| {
                    platform::request_location(
                        move |place| { if actions.change_deed(|state| state.set_location(place)).is_err() { error.set(Some("location-save-error")); } },
                        move || error.set(Some("location-error")),
                    );
                }, {i18n.t("location-use-current")} }
                label { r#for: "city-choice", {i18n.t("location-choose-city")} }
                select { id: "city-choice", class: "editor-input", value: selected.as_ref().and_then(|place| place.city_key.clone()).unwrap_or_default(),
                    onchange: move |event| {
                        if let Some(city) = location::cities().ok().and_then(|cities| cities.into_iter().find(|city| city.key == event.value()))
                            && let Some(place) = city.location()
                            && actions.change_deed(|state| state.set_location(place)).is_err() { error.set(Some("location-save-error")); }
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
                            if actions.change_deed(|state| {
                                let mut place = state.data.settings.location.clone().ok_or(crate::state::EditorError::RuleChange)?;
                                place.country = (!country.is_empty()).then_some(country);
                                state.set_location(place)
                            }).is_err() { error.set(Some("location-save-error")); }
                        },
                        option { value: "", {i18n.t("location-country-unknown")} }
                        for country in countries { option { key: "{country}", value: "{country}", {i18n.t(&format!("country-{country}"))} } }
                    }
                }
                if let Some(key) = error() { p { class: "editor-error", role: "alert", {i18n.t(key)} } }
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
                    button { class: "primary-button", onclick: move |_| actions.view(|state| state.show_screen(Screen::SetPicker)), {i18n.t("first-run-next")} }
                    button { class: "text-button", onclick: move |_| actions.change(|state| { state.clear_location(); state.show_screen(Screen::SetPicker); }), {i18n.t("location-skip")} }
                } else {
                    button { class: "primary-button", onclick: move |_| actions.view(|state| state.show_screen(state.location_return)), {i18n.t(if state.read().location_return == Screen::Settings { "settings-back-location" } else { "location-back-deeds" })} }
                    button { class: "text-button", onclick: move |_| actions.change(|state| { state.clear_location(); state.show_screen(state.location_return); }), {i18n.t("location-remove")} }
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
    let first_run = !app.data.first_run_complete;
    let chosen = app.data.chosen_sets.clone();
    let confirmation = place_summary(&i18n, &app.data.settings);
    let Ok(catalog) = editor_data::catalog() else {
        return rsx! { p { class: "editor-error", {i18n.t("sets-data-error")} } };
    };

    rsx! {
        section { class: "first-run-screen",
            header { class: "first-run-header",
                if first_run { span { class: "rail-brand", {i18n.t("app-name")} } }
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
                            }
                            button { class: "set-toggle", aria_label: i18n.t_with(if active { "sets-remove-named" } else { "sets-add-named" }, &[("set", FluentValue::from(i18n.t(&set.name)))]), aria_pressed: "{active}", onclick: move |_| {
                                if actions.change_deed(|state| state.toggle_set(&key)).is_err() { error.set(true); }
                            }, {i18n.t(if active { "sets-remove" } else { "sets-add" })} }
                        }
                    } }
                }
            }
            footer { class: "first-run-footer",
                if first_run {
                    button { class: "primary-button", onclick: move |_| actions.change(|state| state.finish_first_run(Screen::Sheet)), {i18n.t("sets-start-page")} }
                    button { class: "text-button", onclick: move |_| actions.change(|state| state.finish_first_run(Screen::Editor(None))), {i18n.t("sets-add-own")} }
                    button { class: "text-button", onclick: move |_| actions.view(|state| state.show_screen(Screen::FirstRunLocation)), {i18n.t("sets-back-location")} }
                } else {
                    button { class: "primary-button", onclick: move |_| actions.view(|state| state.show_screen(Screen::Deeds)), {i18n.t("sets-back-deeds")} }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::i18n::{Language, Texts};
    use fluent_bundle::FluentValue;

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
