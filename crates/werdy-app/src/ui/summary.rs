//! Today card with counts, and the save status banner.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::TodaySummary;

use super::context::{use_actions, use_i18n};
use super::reports::Series;
use crate::state::{AppState, SaveStatus, Screen};

/// One line for the day on screen: counts, a thin bar and, on a finished day,
/// the score.
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
    let (line_key, last) = if is_today {
        ("summary-line-today", summary.open)
    } else {
        ("summary-line-past", summary.missed)
    };
    let line = i18n.t_with(
        line_key,
        &[
            ("done", FluentValue::from(summary.done)),
            ("partial", FluentValue::from(summary.partial)),
            ("last", FluentValue::from(last)),
        ],
    );
    let [done, partial, missed] = bar_shares(&summary);

    rsx! {
        aside { class: "today-card",
            div { class: "today-line",
                h2 { class: "today-title", {i18n.t(title_key)} }
                p { class: "today-counts", "{line}" }
                if noted { span { class: "note-sign", role: "img", aria_label: i18n.t("note-sign"), {i18n.t("note-sign-symbol")} } }
                if note_late { span { class: "late-sign", aria_label: i18n.t("mark-added-later"), {i18n.t("mark-added-later")} } }
                match score {
                    Ok(Some(score)) => rsx! {
                        if let Some(percent) = score.percent {
                            p { class: "today-score", {i18n.t_with("summary-score", &[("percent", FluentValue::from(percent))])} }
                        }
                    },
                    Err(_) => rsx! { p { class: "editor-error", role: "alert", {i18n.t("summary-score-error")} } },
                    Ok(None) => rsx! {},
                }
            }
            div { class: "today-bar", "aria-hidden": "true",
                span { "data-kind": "done", style: "inline-size: {done}%" }
                span { "data-kind": "partial", style: "inline-size: {partial}%" }
                span { "data-kind": "missed", style: "inline-size: {missed}%" }
            }
            if is_today {
                p { class: "sr-only", {i18n.t("summary-in-progress")} }
            }
        }
    }
}

/// Done, partial and missed as percents of all due deeds, for the bar. The
/// rest of the bar is open deeds.
fn bar_shares(summary: &TodaySummary) -> [f64; 3] {
    let total = summary.done + summary.partial + summary.open + summary.missed;
    if total == 0 {
        return [0.0; 3];
    }
    let share = |count: usize| count as f64 * 100.0 / total as f64;
    [
        share(summary.done),
        share(summary.partial),
        share(summary.missed),
    ]
}

/// Finished days from the latest week, using the same scored cells as reports.
#[component]
pub fn WeekCard() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let report = state.read().recent_week();

    rsx! {
        section { class: "week-card report-card",
            h2 { class: "card-title", {i18n.t("summary-last-week")} }
            match report {
                Ok(report) => rsx! {
                    if let Some(percent) = report.overall.percent {
                        p { class: "card-note", {i18n.t_with("reports-percent", &[("percent", FluentValue::from(percent))])} }
                    }
                    Series {
                        entries: report.series,
                        noted_days: report.noted_days,
                        late_noted_days: report.late_noted_days,
                        show_today: true,
                    }
                },
                Err(_) => rsx! { p { class: "editor-error", role: "alert", {i18n.t("reports-error")} } },
            }
            button { class: "report-link", onclick: move |_| actions.view(|state| state.show_screen(Screen::Reports)),
                {i18n.t("summary-open-reports")}
            }
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

#[cfg(test)]
mod tests {
    use werdy_core::TodaySummary;

    use super::bar_shares;

    #[test]
    fn the_bar_splits_the_day_into_done_partial_and_missed_shares() {
        // 2 done, 1 partial, 4 open and 1 missed out of 8 due deeds.
        let summary = TodaySummary {
            done: 2,
            partial: 1,
            open: 4,
            missed: 1,
        };
        assert_eq!(bar_shares(&summary), [25.0, 12.5, 12.5]);
        let empty = TodaySummary {
            done: 0,
            partial: 0,
            open: 0,
            missed: 0,
        };
        assert_eq!(bar_shares(&empty), [0.0, 0.0, 0.0]);
    }
}
