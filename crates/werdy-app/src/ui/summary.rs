//! Today card with counts, and the save status banner.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;

use super::context::use_i18n;
use crate::state::{AppState, SaveStatus};

/// Done, partial and open or missed counts for the day on screen.
#[component]
pub fn TodayCard() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let (summary, score, is_today, noted, note_late) = {
        let state = state.read();
        (
            state.summary(),
            state.viewing_score(),
            state.is_today(),
            state.viewing_note().is_some(),
            state
                .viewing_note()
                .is_some_and(|note| note.is_late_for(state.viewing)),
        )
    };
    let title_key = if is_today {
        "summary-title-today"
    } else {
        "summary-title-past"
    };

    rsx! {
        aside { class: "today-card",
            h2 { class: "card-title", {i18n.t(title_key)} }
            if noted { span { class: "note-sign", role: "img", aria_label: i18n.t("note-sign"), {i18n.t("note-sign-symbol")} } }
            if note_late { span { class: "late-sign", aria_label: i18n.t("mark-added-later"), {i18n.t("mark-added-later")} } }
            match score {
                Ok(Some(score)) => rsx! {
                    if let Some(percent) = score.percent {
                        p { class: "card-note", {i18n.t_with("summary-score", &[("percent", FluentValue::from(percent))])} }
                    }
                },
                Err(_) => rsx! { p { class: "editor-error", role: "alert", {i18n.t("summary-score-error")} } },
                Ok(None) => rsx! {},
            }
            div { class: "counts",
                Count { kind: "done", value: summary.done, label: i18n.t("summary-done") }
                Count { kind: "partial", value: summary.partial, label: i18n.t("summary-partial") }
                if is_today {
                    Count { kind: "open", value: summary.open, label: i18n.t("summary-open") }
                } else {
                    Count { kind: "missed", value: summary.missed, label: i18n.t("summary-missed") }
                }
            }
            if is_today {
                p { class: "card-note", {i18n.t("summary-in-progress")} }
            }
        }
    }
}

#[component]
fn Count(kind: &'static str, value: usize, label: String) -> Element {
    let i18n = use_i18n();
    rsx! {
        div { class: "count", "data-kind": kind,
            span { class: "count-value", {i18n.num(value)} }
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
