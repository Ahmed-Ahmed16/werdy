//! Device settings and the dated Hijri correction.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::HijriOffset;
use werdy_store::{Digits, PrayerMethod, Theme};

use super::context::{use_actions, use_i18n};
use super::icons::ChevronForward;
use crate::i18n::Language;
use crate::state::{AppState, EditorError, Screen};

#[component]
pub fn SettingsScreen() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let mut error = use_signal(|| false);
    let app = state.read();
    let settings = &app.data.settings;
    let language = i18n.language;
    let digits = settings.digits;
    let theme = settings.theme;
    let offset = settings.calendar.offset_on(app.today).days();
    let city = settings.location.as_ref().map_or_else(
        || i18n.t("settings-location-unset"),
        |place| {
            place.city_key.as_ref().map_or_else(
                || i18n.t("location-current"),
                |key| {
                    if crate::location::cities()
                        .ok()
                        .is_some_and(|cities| cities.iter().any(|city| city.key == *key))
                    {
                        i18n.t(&format!("city-{key}"))
                    } else {
                        i18n.t("settings-location-unavailable")
                    }
                },
            )
        },
    );
    let method = i18n.t(match settings.prayer_method {
        PrayerMethod::EgyptianGeneralAuthority => "method-egyptian",
        PrayerMethod::MuslimWorldLeague => "method-mwl",
    });
    let place = i18n.t_with(
        "settings-place",
        &[
            ("city", FluentValue::from(city)),
            ("method", FluentValue::from(method)),
        ],
    );

    rsx! {
        section { class: "settings-screen",
            header { class: "settings-header",
                h1 { class: "day-title", {i18n.t("nav-settings")} }
                button { class: "secondary-button settings-back", onclick: move |_| actions.view(|state| state.show_screen(Screen::Deeds)), {i18n.t("settings-back")} }
            }
            div { class: "settings-card",
                fieldset { class: "settings-row",
                    legend { {i18n.t("settings-language")} }
                    div { class: "settings-options",
                        label { class: "settings-choice",
                            input { r#type: "radio", name: "settings-language", checked: language == Language::Arabic, onchange: move |_| actions.change(|state| state.set_language("ar")) }
                            span { {i18n.t("settings-arabic")} }
                        }
                        label { class: "settings-choice",
                            input { r#type: "radio", name: "settings-language", checked: language == Language::English, onchange: move |_| actions.change(|state| state.set_language("en")) }
                            span { {i18n.t("settings-english")} }
                        }
                    }
                }
                fieldset { class: "settings-row",
                    legend { {i18n.t("settings-digits")} }
                    div { class: "settings-options",
                        label { class: "settings-choice",
                            input { r#type: "radio", name: "settings-digits", checked: digits == Digits::Western, onchange: move |_| actions.change(|state| state.set_digits(Digits::Western)) }
                            span { {i18n.t("settings-western")} }
                        }
                        label { class: "settings-choice",
                            input { r#type: "radio", name: "settings-digits", checked: digits == Digits::ArabicIndic, onchange: move |_| actions.change(|state| state.set_digits(Digits::ArabicIndic)) }
                            span { {i18n.t("settings-arabic-indic")} }
                        }
                    }
                }
                fieldset { class: "settings-row",
                    legend { {i18n.t("settings-theme")} }
                    div { class: "settings-options",
                        label { class: "settings-choice",
                            input { r#type: "radio", name: "settings-theme", checked: theme == Theme::System, onchange: move |_| actions.change(|state| state.set_theme(Theme::System)) }
                            span { {i18n.t("settings-system")} }
                        }
                        label { class: "settings-choice",
                            input { r#type: "radio", name: "settings-theme", checked: theme == Theme::Light, onchange: move |_| actions.change(|state| state.set_theme(Theme::Light)) }
                            span { {i18n.t("settings-light")} }
                        }
                        label { class: "settings-choice",
                            input { r#type: "radio", name: "settings-theme", checked: theme == Theme::Dark, onchange: move |_| actions.change(|state| state.set_theme(Theme::Dark)) }
                            span { {i18n.t("settings-dark")} }
                        }
                    }
                }
                fieldset { class: "settings-row",
                    legend { {i18n.t("settings-offset")} }
                    p { class: "settings-help", {i18n.t("settings-offset-help")} }
                    div { class: "settings-options",
                        for days in [-1_i8, 0, 1] {
                            label { class: "settings-choice", key: "{days}",
                                input { r#type: "radio", name: "settings-offset", checked: offset == days,
                                onchange: move |_| {
                                    if let Ok(value) = HijriOffset::new(days) {
                                        error.set(actions.change_deed(|state| state.set_hijri_offset(value).map_err(|_| EditorError::RuleChange)).is_err());
                                    }
                                } }
                                span { {i18n.t(match days { -1 => "settings-offset-minus", 0 => "settings-offset-zero", _ => "settings-offset-plus" })} }
                            }
                        }
                    }
                }
                if error() { p { class: "editor-error", role: "alert", {i18n.t("settings-offset-error")} } }
                button { class: "settings-location", onclick: move |_| actions.view(|state| state.show_screen(Screen::FirstRunLocation)),
                    span { class: "settings-location-copy",
                        strong { {i18n.t("settings-city-method")} }
                        span { class: "settings-help", "{place}" }
                    }
                    ChevronForward {}
                }
            }
        }
    }
}
