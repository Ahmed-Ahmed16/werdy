//! First run level and location steps, and shared starter set picker.

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
pub fn LevelStep() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
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
                                    actions.view(|state| {
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
                    actions.view(|state| {
                        if state.continue_first_run_level().is_err() { error.set(true); }
                    });
                }, {i18n.t("first-run-level-next")} }
                button { class: "text-button", onclick: move |_| {
                    if actions.change_deed(|state| state.start_first_run_empty()).is_err() { error.set(true); }
                }, {i18n.t("first-run-level-empty")} }
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
                                    actions.view(|state| {
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
                button { class: "text-button", onclick: move |_| actions.view(|state| state.back_first_run_extras()), {i18n.t("first-run-extras-back")} }
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
                    button { class: "primary-button", onclick: move |_| actions.view(|state| state.show_screen(Screen::SetPicker)), {i18n.t("first-run-next")} }
                    button { class: "text-button", onclick: move |_| { actions.view(|state| state.location_request.cancel()); actions.change(|state| { state.clear_location(); state.show_screen(Screen::SetPicker); }); }, {i18n.t("location-skip")} }
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
