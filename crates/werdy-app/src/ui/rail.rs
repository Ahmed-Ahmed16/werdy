//! Navigation rail and phone tabs.

use dioxus::prelude::*;

use super::context::{use_actions, use_i18n};
use crate::state::{AppState, Screen};

/// Navigation rail, shown from the expanded breakpoint up. Other screens come
/// with their own tickets.
#[component]
pub fn Rail() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let screen = state.read().screen;
    let later = i18n.t("nav-coming-later");

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
            for key in ["nav-deeds", "nav-settings"] {
                button { key: "{key}", class: "rail-item", disabled: true, title: "{later}",
                    {i18n.t(key)}
                }
            }
        }
    }
}

/// Navigation below the expanded breakpoint. The deeds screen has its own ticket.
#[component]
pub fn BottomNav() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let screen = state.read().screen;

    rsx! {
        nav { class: "bottom-nav", aria_label: i18n.t("nav-label"),
            button { aria_current: if screen == Screen::Sheet { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Sheet)),
                {i18n.t("nav-sheet")}
            }
            button { aria_current: if screen == Screen::Reports { Some("page") } else { None },
                onclick: move |_| actions.view(|state| state.show_screen(Screen::Reports)),
                {i18n.t("nav-reports")}
            }
            button { disabled: true, title: i18n.t("nav-coming-later"), {i18n.t("nav-deeds")} }
        }
    }
}
