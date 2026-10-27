//! Side rail placeholder for wide screens (ADR 0022). Only the sheet exists so far.

use dioxus::prelude::*;

use super::Page;
use super::context::use_i18n;

/// Navigation rail, shown from the expanded breakpoint up. Other screens come
/// with their own tickets.
#[component]
pub fn Rail() -> Element {
    let i18n = use_i18n();
    let later = i18n.t("nav-coming-later");
    let mut page = use_context::<Signal<Page>>();

    rsx! {
        nav { class: "rail",
            span { class: "rail-brand", {i18n.t("app-name")} }
            button { class: "rail-item", aria_current: if page() == Page::Sheet { "page" } else { "false" }, onclick: move |_| page.set(Page::Sheet), {i18n.t("nav-sheet")} }
            button { class: "rail-item", aria_current: if matches!(page(), Page::Deeds | Page::Editor(_)) { "page" } else { "false" }, onclick: move |_| page.set(Page::Deeds), {i18n.t("nav-deeds")} }
            for key in ["nav-reports", "nav-settings"] {
                button { key: "{key}", class: "rail-item", disabled: true, title: "{later}",
                    {i18n.t(key)}
                }
            }
        }
    }
}

#[component]
pub fn BottomNav() -> Element {
    let i18n = use_i18n();
    let mut page = use_context::<Signal<Page>>();
    rsx! {
        nav { class: "bottom-nav", aria_label: i18n.t("nav-label"),
            button { class: "bottom-item", aria_current: if page() == Page::Sheet { "page" } else { "false" }, onclick: move |_| page.set(Page::Sheet), {i18n.t("nav-sheet")} }
            button { class: "bottom-item", aria_current: if matches!(page(), Page::Deeds | Page::Editor(_)) { "page" } else { "false" }, onclick: move |_| page.set(Page::Deeds), {i18n.t("nav-deeds")} }
            button { class: "bottom-item", disabled: true, {i18n.t("nav-reports")} }
        }
    }
}
