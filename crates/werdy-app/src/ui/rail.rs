//! Navigation rail and phone tabs.

use dioxus::prelude::*;

use super::context::{use_actions, use_i18n};
use crate::state::{AppState, Screen};

/// Navigation rail, shown from the expanded breakpoint up.
#[component]
pub fn Rail() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let screen = state.read().screen;

    rsx! {
        nav { class: "rail",
            span { class: "rail-brand", {i18n.t("app-name")} }
            button { class: "rail-item", aria_current: if screen == Screen::Sheet { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Sheet)),
                {i18n.t("nav-sheet")}
            }
            button { class: "rail-item", aria_current: if screen == Screen::Reports { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Reports)),
                {i18n.t("nav-reports")}
            }
            button { class: "rail-item", aria_current: if matches!(screen, Screen::Deeds | Screen::Editor(_)) { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Deeds)),
                {i18n.t("nav-deeds")}
            }
            button { class: "rail-item", aria_current: if screen == Screen::Settings { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Settings)),
                {i18n.t("nav-settings")}
            }
        }
    }
}

/// Navigation below the expanded breakpoint.
#[component]
pub fn BottomNav() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let screen = state.read().screen;

    rsx! {
        nav { class: "bottom-nav", aria_label: i18n.t("nav-label"),
            button { class: "bottom-item", aria_current: if screen == Screen::Sheet { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Sheet)),
                {i18n.t("nav-sheet")}
            }
            button { class: "bottom-item", aria_current: if screen == Screen::Reports { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Reports)),
                {i18n.t("nav-reports")}
            }
            button { class: "bottom-item", aria_current: if matches!(screen, Screen::Deeds | Screen::Editor(_)) { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Deeds)),
                {i18n.t("nav-deeds")}
            }
        }
    }
}
