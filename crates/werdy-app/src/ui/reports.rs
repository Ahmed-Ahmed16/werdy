//! Reports screen. Core supplies every score and due day.

use std::collections::BTreeSet;
use std::rc::Rc;

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{
    Band, Consistency, DEFAULT_CONSISTENCY_WINDOW, DeedDay, DeedId, Fraction, Period, Report,
    ReportLead, SeriesEntry, WerdyDay, consistency, report_lead,
};
use werdy_store::DeedName;

use super::context::{I18n, use_actions, use_i18n};
use super::icons::{CheckIcon, ChevronBack, ChevronForward, CrossIcon, HalfIcon};
use crate::state::{AppState, Screen};

const PERIODS: [(Period, &str); 5] = [
    (Period::ThreeDays, "reports-three-days"),
    (Period::Week, "reports-week"),
    (Period::Month, "reports-month"),
    (Period::SixMonths, "reports-six-months"),
    (Period::Year, "reports-year"),
];

/// Full report, computed before any score is shown.
#[component]
pub fn ReportsScreen() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let (report, previous, period, steps_back) = {
        let state = state.read();
        (
            state.report(),
            state.previous_report(),
            state.report_period,
            state.report_steps_back,
        )
    };

    rsx! {
        section { class: "reports",
            ReportHeader { period, steps_back, days: report.as_ref().map_or_else(|_| Vec::new(), |report| report.days.clone()) }
            match report {
                Ok(report) if report.overall.due == 0 => rsx! {
                    section { class: "report-card report-empty",
                        p { {i18n.t("reports-empty")} }
                        TodayLink {}
                    }
                },
                Ok(report) => {
                    let report = Rc::new(report);
                    rsx! {
                        div { class: "report-layout",
                            div { class: "report-main",
                                ScoreCard { report: Rc::clone(&report), previous, steps_back }
                                MissedList { report: Rc::clone(&report) }
                                CategorySplit { report: Rc::clone(&report) }
                            }
                            div { class: "report-detail",
                                PerDeed { report: Rc::clone(&report) }
                                NotDue { report }
                            }
                        }
                    }
                },
                Err(_) => rsx! { p { class: "status-banner", role: "alert", {i18n.t("reports-error")} } },
            }
        }
    }
}

#[component]
fn ReportHeader(period: Period, steps_back: usize, days: Vec<WerdyDay>) -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let range = days
        .first()
        .zip(days.last())
        .map(|(first, last)| range_text(&i18n, *first, *last));

    rsx! {
        header { class: "report-header",
            h1 { class: "day-title", {i18n.t("reports-title")} }
            div { class: "period-chips", role: "group", aria_label: i18n.t("reports-periods"),
                for (choice, key) in PERIODS {
                    button { key: "{key}", class: "period-chip", aria_pressed: period == choice,
                        onclick: move |_| actions.view(|state| state.select_report_period(choice)),
                        {i18n.t(key)}
                    }
                }
            }
            div { class: "period-nav",
                button { class: "icon-button", aria_label: i18n.t("reports-previous"),
                    onclick: move |_| actions.view(AppState::show_previous_period), ChevronBack {}
                }
                div { class: "period-dates",
                    if let Some(range) = range { span { "{range}" } }
                    if steps_back == 0 { small { {i18n.t("reports-until-yesterday")} } }
                }
                button { class: "icon-button", aria_label: i18n.t("reports-next"), disabled: steps_back == 0,
                    onclick: move |_| actions.view(AppState::show_next_period), ChevronForward {}
                }
            }
        }
    }
}

fn range_text(i18n: &I18n, first: WerdyDay, last: WerdyDay) -> String {
    let (start_year, start_month, start_day) = first.date_parts();
    let (end_year, end_month, end_day) = last.date_parts();
    i18n.t_with(
        "reports-range",
        &[
            ("startDay", FluentValue::from(start_day)),
            (
                "startMonth",
                FluentValue::from(i18n.t(&format!("month-{start_month}"))),
            ),
            ("startYear", FluentValue::from(i18n.num(start_year))),
            ("endDay", FluentValue::from(end_day)),
            (
                "endMonth",
                FluentValue::from(i18n.t(&format!("month-{end_month}"))),
            ),
            ("endYear", FluentValue::from(i18n.num(end_year))),
        ],
    )
}

#[component]
fn ScoreCard(
    report: Rc<Report>,
    previous: Result<Report, werdy_core::Error>,
    steps_back: usize,
) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let today_counts = state.read().summary_today();
    let lead = match report_lead(&report) {
        ReportLead::MostMissed(id) => i18n.t_with(
            "reports-lead-missed",
            &[(
                "deed",
                FluentValue::from(deed_name(&state.read(), &i18n, id)),
            )],
        ),
        ReportLead::AllDone => i18n.t("reports-lead-all-done"),
        ReportLead::Empty => i18n.t("reports-empty"),
    };
    let due = i18n.t_with(
        "reports-score-due",
        &[("due", FluentValue::from(report.overall.due))],
    );
    let previous_line = match previous {
        Ok(previous) => match previous.overall.percent {
            Some(percent) => i18n.t_with(
                "reports-previous-score",
                &[
                    ("percent", FluentValue::from(percent)),
                    ("due", FluentValue::from(previous.overall.due)),
                ],
            ),
            None => i18n.t("reports-previous-empty"),
        },
        Err(_) => i18n.t("reports-previous-unavailable"),
    };

    rsx! {
        section { class: "report-card report-score-card",
            p { class: "report-lead", "{lead}" }
            if let Some(percent) = report.overall.percent {
                div { class: "report-score-line",
                    strong { class: "report-score", {percent_text(&i18n, percent)} }
                    span { class: "report-muted", "{due}" }
                }
            }
            p { class: "report-muted", "{previous_line}" }
            Series { entries: report.series.clone(), noted_days: report.noted_days.clone(), late_noted_days: report.late_noted_days.clone(), show_today: steps_back == 0 }
            if steps_back == 0 {
                div { class: "report-today-line",
                    span { {i18n.t("reports-today")} }
                    span { {i18n.t_with("reports-today-counts", &[
                        ("done", FluentValue::from(today_counts.done)),
                        ("partial", FluentValue::from(today_counts.partial)),
                        ("open", FluentValue::from(today_counts.open)),
                        ("missed", FluentValue::from(today_counts.missed)),
                    ])} }
                    TodayLink {}
                }
            }
        }
    }
}

#[component]
fn TodayLink() -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    rsx! {
        button { class: "report-link", onclick: move |_| actions.view(|state| { state.show_today(); state.show_screen(Screen::Sheet); }),
            {i18n.t("reports-open-today")}
        }
    }
}

#[component]
fn Series(
    entries: Vec<SeriesEntry>,
    noted_days: BTreeSet<WerdyDay>,
    late_noted_days: BTreeSet<WerdyDay>,
    show_today: bool,
) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let today_noted = {
        let state = state.read();
        state.data.day_notes.contains_key(&state.today)
    };
    let today_label = if today_noted {
        detail_label(&i18n, &i18n.t("reports-today"), &i18n.t("note-sign"))
    } else {
        i18n.t("reports-today")
    };
    let long = entries
        .first()
        .is_some_and(|entry| matches!(entry, SeriesEntry::Month { .. }));
    rsx! {
        div { class: if long { "report-series report-months" } else { "report-series report-days" },
            h2 { class: "sr-only", {i18n.t("reports-series")} }
            for (index, entry) in entries.into_iter().enumerate() {
                SeriesCell { key: "{index}", noted: entry_is_noted(&entry, &noted_days), late_note: entry_is_noted(&entry, &late_noted_days), entry }
            }
            if show_today {
                div { class: "report-day-cell report-today-cell", role: "img", aria_label: today_label,
                    span { {i18n.t("reports-today")} }
                    if today_noted { small { aria_label: i18n.t("note-sign"), {i18n.t("reports-note-sign")} } }
                }
            }
        }
    }
}

fn entry_is_noted(entry: &SeriesEntry, noted_days: &BTreeSet<WerdyDay>) -> bool {
    match entry {
        SeriesEntry::Day { day, .. } => noted_days.contains(day),
        SeriesEntry::Month { year, month, .. } => noted_days
            .iter()
            .any(|day| day.year_month() == (*year, *month)),
    }
}

#[component]
fn SeriesCell(entry: SeriesEntry, noted: bool, late_note: bool) -> Element {
    let i18n = use_i18n();
    let mut label = series_label(&i18n, &entry);
    if noted {
        label = detail_label(&i18n, &label, &i18n.t("reports-note"));
    }
    if late_note {
        label = detail_label(&i18n, &label, &i18n.t("mark-added-later"));
    }
    match entry {
        SeriesEntry::Day { day, score, band } => {
            let (_, _, number) = day.date_parts();
            rsx! {
                div { class: "report-day-cell", "data-band": band_name(band), role: "img", aria_label: label,
                    span { {i18n.num(number)} }
                    span { class: "report-cell-symbol",
                        match band {
                            Some(Band::Good) => rsx! { CheckIcon {} },
                            Some(Band::Mid) => rsx! { HalfIcon {} },
                            Some(Band::Low) => rsx! { CrossIcon {} },
                            None => rsx! {},
                        }
                    }
                    if noted { small { title: i18n.t("reports-note"), {i18n.t("reports-note-sign")} } }
                    if late_note { small { title: i18n.t("mark-added-later"), {i18n.t("reports-late-sign")} } }
                    if let Some(percent) = score.percent { small { {percent_text(&i18n, percent)} } }
                    if score.percent.is_some() { small { {i18n.t_with("reports-cell-due", &[("due", FluentValue::from(score.due))])} } }
                }
            }
        }
        SeriesEntry::Month {
            year: _,
            month,
            score,
            band,
        } => {
            let name = i18n.t(&format!("month-{month}"));
            rsx! {
                div { class: "report-month-row", role: "img", aria_label: label,
                    span { "{name}" if noted { {i18n.t("reports-note-sign")} } if late_note { {i18n.t("reports-late-sign")} } }
                    div { class: "report-bar-track",
                        if let Some(percent) = score.percent {
                            div { class: "report-bar", "data-band": band_name(band), style: "width: {percent}%" }
                        }
                    }
                    if let Some(percent) = score.percent { span { {percent_text(&i18n, percent)} } }
                    if score.percent.is_none() { span { {i18n.t("reports-no-score")} } }
                    if score.percent.is_some() { small { class: "report-muted", {i18n.t_with("reports-cell-due", &[("due", FluentValue::from(score.due))])} } }
                }
            }
        }
    }
}

fn series_label(i18n: &I18n, entry: &SeriesEntry) -> String {
    let (scored_key, empty_key, date_key, date_value, month, score) = match entry {
        SeriesEntry::Day { day, score, .. } => {
            let (_, month, number) = day.date_parts();
            (
                "reports-day-value",
                "reports-day-no-due",
                "day",
                number.to_string(),
                month,
                score,
            )
        }
        SeriesEntry::Month {
            year, month, score, ..
        } => (
            "reports-month-value",
            "reports-month-no-due",
            "year",
            year.to_string(),
            *month,
            score,
        ),
    };
    let mut args = vec![
        (
            "month",
            FluentValue::from(i18n.t(&format!("month-{month}"))),
        ),
        (date_key, FluentValue::from(i18n.num(date_value))),
    ];
    if let Some(percent) = score.percent {
        args.push(("percent", FluentValue::from(percent)));
        args.push(("due", FluentValue::from(score.due)));
        i18n.t_with(scored_key, &args)
    } else {
        i18n.t_with(empty_key, &args)
    }
}

fn percent_text(i18n: &I18n, percent: u8) -> String {
    i18n.t_with(
        "reports-percent",
        &[("percent", FluentValue::from(percent))],
    )
}

fn detail_label(i18n: &I18n, base: &str, detail: &str) -> String {
    i18n.t_with(
        "reports-detail-label",
        &[
            ("base", FluentValue::from(base.to_owned())),
            ("detail", FluentValue::from(detail.to_owned())),
        ],
    )
}

fn band_name(band: Option<Band>) -> &'static str {
    match band {
        Some(Band::Good) => "good",
        Some(Band::Mid) => "mid",
        Some(Band::Low) => "low",
        None => "none",
    }
}

#[component]
fn CategorySplit(report: Rc<Report>) -> Element {
    let i18n = use_i18n();
    rsx! {
        section { class: "report-card",
            h2 { class: "card-title", {i18n.t("reports-category")} }
            for (category, score) in report.by_category.iter() {
                div { class: "report-meter-row", key: "{category.0}",
                    span { {i18n.t(&format!("category-{}", category.0))} }
                    div { class: "report-bar-track",
                        if let Some(percent) = score.percent {
                            div { class: "report-bar", "data-band": band_name(Some(report.bands.band(percent))),
                                style: "width: {percent}%" }
                        }
                    }
                    if let Some(percent) = score.percent { span { {percent_text(&i18n, percent)} } }
                    small { {i18n.t_with("reports-category-due", &[("due", FluentValue::from(score.due))])} }
                }
            }
        }
    }
}

#[component]
fn MissedList(report: Rc<Report>) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    rsx! {
        section { class: "report-card",
            h2 { class: "card-title", {i18n.t("reports-missed")} }
            if report.missed.is_empty() { p { class: "report-muted", {i18n.t("reports-none-missed")} } }
            for missed in report.missed.iter() {
                div { class: "report-list-row", key: "{missed.deed_id.0}",
                    div { class: "report-list-detail",
                        span { {deed_name(&state.read(), &i18n, missed.deed_id)} }
                        small { class: "report-muted", {i18n.t_with("reports-missed-days", &[("days", FluentValue::from(missed.days_not_done))])} }
                    }
                    if let Some(score) = report.per_deed.get(&missed.deed_id) {
                        div { class: "report-list-detail",
                            if let Some(percent) = score.percent { strong { {percent_text(&i18n, percent)} } }
                            small { class: "report-muted", {i18n.t_with("reports-deed-due", &[("due", FluentValue::from(score.due))])} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PerDeed(report: Rc<Report>) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let last_day = report.days.last().copied();
    rsx! {
        section { class: "report-card",
            h2 { class: "card-title", {i18n.t("reports-deeds")} }
            for (id, score) in report.per_deed.iter() {
                div { class: "report-deed-row", key: "{id.0}",
                    div { class: "report-deed-head",
                        strong { {deed_name(&state.read(), &i18n, *id)} }
                        if let Some(percent) = score.percent { strong { {percent_text(&i18n, percent)} } }
                    }
                    small { class: "report-muted", {i18n.t_with("reports-deed-due", &[("due", FluentValue::from(score.due))])} }
                    small { class: "report-muted", {i18n.consistency_line(deed_consistency(&state.read(), *id, last_day))} }
                    if let Some(days) = report.per_deed_days.get(id) {
                        div { class: "report-history",
                            for entry in days { HistoryDay { key: "{entry.day:?}", entry: *entry, noted: report.noted_days.contains(&entry.day) } }
                        }
                    }
                }
            }
        }
    }
}

fn deed_consistency(state: &AppState, id: DeedId, last_day: Option<WerdyDay>) -> Consistency {
    let cutoff = match last_day.and_then(|day| day.next().ok()) {
        Some(day) => day,
        None => state.today,
    };
    consistency(
        &state.data.records(),
        id,
        cutoff,
        DEFAULT_CONSISTENCY_WINDOW,
    )
}

#[component]
fn HistoryDay(entry: DeedDay, noted: bool) -> Element {
    let i18n = use_i18n();
    let (_, month, day) = entry.day.date_parts();
    let date = i18n.t_with(
        "reports-history-day",
        &[
            ("day", FluentValue::from(day)),
            (
                "month",
                FluentValue::from(i18n.t(&format!("month-{month}"))),
            ),
        ],
    );
    let (kind, key) = if entry.value == Fraction::ONE {
        ("done", "reports-history-done")
    } else if entry.value == Fraction::ZERO {
        ("missed", "reports-history-missed")
    } else {
        ("partial", "reports-history-partial")
    };
    let mut label = i18n.t_with(
        "reports-history-label",
        &[
            ("date", FluentValue::from(date.clone())),
            ("mark", FluentValue::from(i18n.t(key))),
        ],
    );
    if noted {
        label = detail_label(&i18n, &label, &i18n.t("reports-note"));
    }
    if entry.added_later {
        label = detail_label(&i18n, &label, &i18n.t("mark-added-later"));
    }
    rsx! {
        span { class: "report-history-cell", "data-kind": kind, role: "img",
            aria_label: label.clone(),
            title: label,
            span { "{date}" }
            span { class: "report-history-symbol",
                match kind {
                    "done" => rsx! { CheckIcon {} },
                    "missed" => rsx! { CrossIcon {} },
                    _ => rsx! { HalfIcon {} },
                }
            }
            if noted || entry.added_later {
                span { class: "report-history-signs",
                    if noted { {i18n.t("reports-note-sign")} }
                    if entry.added_later { {i18n.t("reports-late-sign")} }
                }
            }
        }
    }
}

#[component]
fn NotDue(report: Rc<Report>) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    if report.not_due.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "report-card report-not-due",
            h2 { class: "card-title", {i18n.t("reports-not-due")} }
            for id in report.not_due.iter() {
                p { key: "{id.0}", {deed_name(&state.read(), &i18n, *id)} }
            }
        }
    }
}

fn deed_name(state: &AppState, i18n: &I18n, id: DeedId) -> String {
    match state.deed_name(id) {
        Some(DeedName::Key(key)) => i18n.t(key),
        Some(DeedName::Text(text)) => text.clone(),
        None => i18n.t("reports-unknown-deed"),
    }
}
