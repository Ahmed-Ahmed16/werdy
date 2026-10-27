//! Side rail placeholder for wide screens (ADR 0022). Only the sheet exists so far.

use dioxus::prelude::*;

use super::context::use_i18n;

/// Navigation rail, shown from the expanded breakpoint up. Other screens come
/// with their own tickets.
#[component]
pub fn Rail() -> Element {
    let i18n = use_i18n();
    let later = i18n.t("nav-coming-later");

    rsx! {
        nav { class: "rail",
            span { class: "rail-brand", {i18n.t("app-name")} }
            button { class: "rail-item", aria_current: "page", {i18n.t("nav-sheet")} }
            for key in ["nav-deeds", "nav-reports", "nav-settings"] {
                button { key: "{key}", class: "rail-item", disabled: true, title: "{later}",
                    {i18n.t(key)}
                }
            }
        }
    }
}
