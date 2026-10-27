//! Today card with counts, and the save status banner.

use dioxus::prelude::*;

use super::context::use_i18n;
use crate::state::{AppState, SaveStatus};

/// Done, partial and open counts for the day on screen. No percent: the day is
/// not finished (ADR 0013).
#[component]
pub fn TodayCard() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let (summary, is_today) = {
        let state = state.read();
        (state.summary(), state.is_today())
    };
    let title_key = if is_today {
        "summary-title-today"
    } else {
        "summary-title-past"
    };

    rsx! {
        aside { class: "today-card",
            h2 { class: "card-title", {i18n.t(title_key)} }
            div { class: "counts",
                Count { kind: "done", value: summary.done, label: i18n.t("summary-done") }
                Count { kind: "partial", value: summary.partial, label: i18n.t("summary-partial") }
                Count { kind: "open", value: summary.open, label: i18n.t("summary-open") }
            }
            if is_today {
                p { class: "card-note", {i18n.t("summary-in-progress")} }
            }
        }
    }
}

#[component]
fn Count(kind: &'static str, value: usize, label: String) -> Element {
    rsx! {
        div { class: "count", "data-kind": kind,
            span { class: "count-value", "{value}" }
            span { class: "count-label", "{label}" }
        }
    }
}

/// Shows a short line when saving failed or saved data could not be read.
#[component]
pub fn StatusBanner() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let key = match state.read().save_status {
        SaveStatus::Saved => return rsx! {},
        SaveStatus::SaveFailed => "store-save-failed",
        SaveStatus::LoadFailed => "store-load-failed",
        SaveStatus::DateFailed => "app-date-failed",
    };

    rsx! {
        p { class: "status-banner", role: "alert", {i18n.t(key)} }
    }
}
