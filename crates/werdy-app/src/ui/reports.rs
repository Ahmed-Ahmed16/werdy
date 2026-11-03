//! Reports screen. Core supplies every score and due day.

use std::collections::BTreeSet;
use std::rc::Rc;

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{
    Band, DayDot, DeedId, DeedRank, Period, PrayerDay, Report, ReportLead, SeriesEntry, WerdyDay,
    categories_weakest_first, percent_change, prayer_grid, ranked_deeds, report_lead,
};
use werdy_store::DeedName;

use super::context::{I18n, use_actions, use_i18n};
use super::icons::{CheckIcon, ChevronBack, ChevronForward, CrossIcon, HalfIcon};
use crate::starter::fard_prayer_ids;
use crate::state::{AppState, Screen};

const PERIODS: [(Period, &str); 5] = [
    (Period::ThreeDays, "reports-three-days"),
    (Period::Week, "reports-week"),
    (Period::Month, "reports-month"),
    (Period::SixMonths, "reports-six-months"),
    (Period::Year, "reports-year"),
];

/// Days of the period shown in the prayer grid, counted back from its last day.
const PRAYER_GRID_DAYS: usize = 7;
/// Deeds in each of the weakest and strongest lists.
const RANKED_DEEDS: usize = 3;

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
                    section { class: "report-surface report-empty",
                        p { {i18n.t("reports-empty")} }
                        TodayLink {}
                    }
                },
                Ok(report) => {
                    let report = Rc::new(report);
                    rsx! {
                        div { class: "report-layout",
                            div { class: "report-surface",
                                Headline { report: Rc::clone(&report), previous, steps_back }
                                DaysBlock { report: Rc::clone(&report), show_today: steps_back == 0 }
                            }
                            div { class: "report-surface",
                                RankedBlock { report: Rc::clone(&report) }
                                PrayersBlock { report: Rc::clone(&report) }
                                CategorySplit { report: Rc::clone(&report) }
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

/// The period's percent, its change from the previous period, and today.
#[component]
fn Headline(
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
    let (trend, change) = change_line(&i18n, &report, previous.as_ref());

    rsx! {
        section { class: "report-block report-headline",
            div { class: "report-score-line",
                if let Some(percent) = report.overall.percent {
                    strong { class: "report-score", {percent_text(&i18n, percent)} }
                }
                span { class: "report-muted", "{due}" }
            }
            p { class: "report-change", "data-trend": trend,
                if let Some(sign) = trend_sign(trend) { span { aria_hidden: "true", "{sign} " } }
                "{change}"
            }
            p { class: "report-lead", "{lead}" }
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

/// Trend name and text for the change from the previous period.
fn change_line(
    i18n: &I18n,
    report: &Report,
    previous: Result<&Report, &werdy_core::Error>,
) -> (&'static str, String) {
    let Ok(previous) = previous else {
        return ("none", i18n.t("reports-previous-unavailable"));
    };
    let (Some(before), Some(points)) = (
        previous.overall.percent,
        percent_change(&report.overall, &previous.overall),
    ) else {
        return ("none", i18n.t("reports-previous-empty"));
    };
    let (trend, key) = match points.signum() {
        1 => ("up", "reports-change-up"),
        -1 => ("down", "reports-change-down"),
        _ => ("same", "reports-change-same"),
    };
    let text = i18n.t_with(
        key,
        &[
            ("points", FluentValue::from(points.unsigned_abs())),
            ("percent", FluentValue::from(before)),
        ],
    );
    (trend, text)
}

fn trend_sign(trend: &str) -> Option<&'static str> {
    match trend {
        "up" => Some("▲"),
        "down" => Some("▼"),
        _ => None,
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

/// A coloured bar per day for short periods, a bar per month for long ones.
/// The numbers sit in each bar's label and in the list under the legend.
#[component]
fn DaysBlock(report: Rc<Report>, show_today: bool) -> Element {
    let i18n = use_i18n();
    let by_day = report
        .series
        .iter()
        .all(|entry| matches!(entry, SeriesEntry::Day { .. }));
    let signs = by_day && !report.noted_days.is_empty();
    rsx! {
        section { class: "report-block",
            h2 { class: "report-block-title", {i18n.t("reports-series")} }
            if by_day {
                DayBars { report: Rc::clone(&report), show_today }
            } else {
                Series { entries: report.series.clone(), noted_days: report.noted_days.clone(), late_noted_days: report.late_noted_days.clone(), show_today: false }
            }
            BandLegend { report: Rc::clone(&report), signs }
            details { class: "report-numbers",
                summary { {i18n.t("reports-show-numbers")} }
                ul {
                    for (index, entry) in report.series.iter().enumerate() {
                        li { key: "{index}", {entry_label(&i18n, entry, &report.noted_days, &report.late_noted_days)} }
                    }
                }
            }
        }
    }
}

#[component]
fn DayBars(report: Rc<Report>, show_today: bool) -> Element {
    let i18n = use_i18n();
    let first = report.days.first().map(|day| short_date(&i18n, *day));
    let last = if show_today {
        Some(i18n.t("reports-today-short"))
    } else {
        report.days.last().map(|day| short_date(&i18n, *day))
    };
    rsx! {
        div { class: "report-bars", role: "list",
            for (index, entry) in report.series.iter().enumerate() {
                if let SeriesEntry::Day { day, score, band } = entry {
                    div { key: "{index}", class: "report-day-bar", role: "listitem",
                        "data-sign": report.noted_days.contains(day),
                        aria_label: entry_label(&i18n, entry, &report.noted_days, &report.late_noted_days),
                        title: entry_label(&i18n, entry, &report.noted_days, &report.late_noted_days),
                        if let Some(percent) = score.percent {
                            div { class: "report-day-fill", "data-band": band_name(*band), style: "block-size: {percent}%" }
                        }
                    }
                }
            }
            if show_today {
                div { class: "report-day-bar report-today-bar", role: "listitem", aria_label: i18n.t("reports-today"), title: i18n.t("reports-today") }
            }
        }
        div { class: "report-axis", aria_hidden: "true",
            if let Some(first) = first { span { "{first}" } }
            if let Some(last) = last { span { "{last}" } }
        }
    }
}

#[component]
fn BandLegend(report: Rc<Report>, signs: bool) -> Element {
    let i18n = use_i18n();
    let bands = report.bands;
    rsx! {
        ul { class: "report-legend",
            li { span { class: "report-swatch", "data-band": "good" } {i18n.t_with("reports-legend-good", &[("percent", FluentValue::from(bands.good))])} }
            li { span { class: "report-swatch", "data-band": "mid" } {i18n.t_with("reports-legend-mid", &[("low", FluentValue::from(bands.mid)), ("high", FluentValue::from(bands.good))])} }
            li { span { class: "report-swatch", "data-band": "low" } {i18n.t_with("reports-legend-low", &[("percent", FluentValue::from(bands.mid))])} }
            if signs { li { span { class: "report-swatch report-sign-swatch" } {i18n.t("reports-note")} } }
        }
    }
}

fn entry_label(
    i18n: &I18n,
    entry: &SeriesEntry,
    noted_days: &BTreeSet<WerdyDay>,
    late_noted_days: &BTreeSet<WerdyDay>,
) -> String {
    let mut label = series_label(i18n, entry);
    if entry_is_noted(entry, noted_days) {
        label = detail_label(i18n, &label, &i18n.t("reports-note"));
    }
    if entry_is_noted(entry, late_noted_days) {
        label = detail_label(i18n, &label, &i18n.t("mark-added-later"));
    }
    label
}

fn short_date(i18n: &I18n, day: WerdyDay) -> String {
    let (_, month, number) = day.date_parts();
    i18n.t_with(
        "reports-history-day",
        &[
            ("day", FluentValue::from(i18n.num(number))),
            (
                "month",
                FluentValue::from(i18n.t(&format!("month-{month}"))),
            ),
        ],
    )
}

#[component]
pub(super) fn Series(
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
                    span { class: "report-today-label", {i18n.t("reports-today")} }
                    span { class: "report-today-compact", {i18n.t("reports-today-short")} }
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
                    if let Some(percent) = score.percent { small { class: "report-score-detail", {percent_text(&i18n, percent)} } }
                    if score.percent.is_some() { small { class: "report-score-detail", {i18n.t_with("reports-cell-due", &[("due", FluentValue::from(score.due))])} } }
                }
            }
        }
        SeriesEntry::Month {
            year: _,
            month,
            score,
            band,
        } => {
            let mut name = i18n.t(&format!("month-{month}"));
            if noted {
                name.push_str(&i18n.t("reports-note-sign"));
            }
            if late_note {
                name.push_str(&i18n.t("reports-late-sign"));
            }
            let detail = if score.percent.is_some() {
                i18n.t_with("reports-cell-due", &[("due", FluentValue::from(score.due))])
            } else {
                String::new()
            };
            rsx! {
                div { role: "img", aria_label: label,
                    MeterRow { label: name, percent: score.percent, band, detail }
                }
            }
        }
    }
}

/// A name and its percent on one line, the bar under them, then a detail line.
/// Long names wrap instead of squeezing the bar.
#[component]
fn MeterRow(label: String, percent: Option<u8>, band: Option<Band>, detail: String) -> Element {
    let i18n = use_i18n();
    rsx! {
        div { class: "report-meter-row",
            div { class: "report-row-head",
                span { class: "report-row-name", "{label}" }
                if let Some(percent) = percent {
                    strong { {percent_text(&i18n, percent)} }
                } else {
                    span { class: "report-muted", {i18n.t("reports-no-score")} }
                }
            }
            div { class: "report-bar-track",
                if let Some(percent) = percent {
                    div { class: "report-bar", "data-band": band_name(band), style: "inline-size: {percent}%" }
                }
            }
            if !detail.is_empty() { small { class: "report-muted", "{detail}" } }
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

/// The weakest deeds, then the strongest, each with a bar.
#[component]
fn RankedBlock(report: Rc<Report>) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let ranked = ranked_deeds(&report, RANKED_DEEDS);
    let due_days = |rank: &DeedRank| {
        i18n.t_with(
            "reports-deed-due",
            &[("due", FluentValue::from(rank.score.due))],
        )
    };
    let not_done = |rank: &DeedRank| {
        report
            .missed
            .iter()
            .find(|missed| missed.deed_id == rank.deed_id)
            .map_or_else(
                || due_days(rank),
                |missed| {
                    i18n.t_with(
                        "reports-missed-days",
                        &[("days", FluentValue::from(missed.days_not_done))],
                    )
                },
            )
    };
    rsx! {
        section { class: "report-block",
            h2 { class: "report-block-title", {i18n.t("reports-ranked")} }
            h3 { class: "report-list-title", {i18n.t("reports-weakest")} }
            if ranked.weakest.is_empty() { p { class: "report-muted", {i18n.t("reports-none-weak")} } }
            for rank in ranked.weakest.iter() {
                MeterRow { key: "{rank.deed_id.0}",
                    label: deed_name(&state.read(), &i18n, rank.deed_id),
                    percent: rank.score.percent,
                    band: rank.score.percent.map(|percent| report.bands.band(percent)),
                    detail: not_done(rank),
                }
            }
            if !ranked.strongest.is_empty() {
                h3 { class: "report-list-title", {i18n.t("reports-strongest")} }
                for rank in ranked.strongest.iter() {
                    MeterRow { key: "{rank.deed_id.0}",
                        label: deed_name(&state.read(), &i18n, rank.deed_id),
                        percent: rank.score.percent,
                        band: rank.score.percent.map(|percent| report.bands.band(percent)),
                        detail: due_days(rank),
                    }
                }
            }
        }
    }
}

/// The five fard prayers (decision 0041): a dot per day for the last days of
/// the period, and the period percent. Hidden when the owner has none of them.
#[component]
fn PrayersBlock(report: Rc<Report>) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let prayers = fard_prayer_ids(&state.read().data);
    let start = report.days.len().saturating_sub(PRAYER_GRID_DAYS);
    let days = report.days.get(start..).unwrap_or_default();
    let grid = prayer_grid(&report, &prayers, days);
    if grid.is_empty() {
        return rsx! {};
    }
    let range = days
        .first()
        .zip(days.last())
        .map(|(first, last)| range_text(&i18n, *first, *last));
    rsx! {
        section { class: "report-block",
            h2 { class: "report-block-title", {i18n.t("reports-prayers")} }
            if let Some(range) = range { p { class: "report-muted", "{range}" } }
            for row in grid {
                div { class: "report-prayer-row", key: "{row.deed_id.0}",
                    div { class: "report-row-head",
                        span { class: "report-row-name", {deed_name(&state.read(), &i18n, row.deed_id)} }
                        if let Some(percent) = report.per_deed.get(&row.deed_id).and_then(|score| score.percent) {
                            strong { {percent_text(&i18n, percent)} }
                        }
                    }
                    div { class: "report-dots", role: "list",
                        for cell in row.days {
                            PrayerDot { key: "{cell.day:?}", cell }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PrayerDot(cell: PrayerDay) -> Element {
    let i18n = use_i18n();
    let (kind, key) = match cell.dot {
        DayDot::Done => ("done", "reports-history-done"),
        DayDot::Partial => ("partial", "reports-history-partial"),
        DayDot::Missed => ("missed", "reports-history-missed"),
        DayDot::NotDue => ("none", "reports-history-not-due"),
    };
    let mut label = i18n.t_with(
        "reports-history-label",
        &[
            ("date", FluentValue::from(short_date(&i18n, cell.day))),
            ("mark", FluentValue::from(i18n.t(key))),
        ],
    );
    if cell.noted {
        label = detail_label(&i18n, &label, &i18n.t("reports-note"));
    }
    if cell.added_later {
        label = detail_label(&i18n, &label, &i18n.t("mark-added-later"));
    }
    rsx! {
        span { class: "report-dot", role: "listitem", "data-kind": kind,
            "data-sign": cell.noted || cell.added_later,
            aria_label: label.clone(), title: label,
            match cell.dot {
                DayDot::Done => rsx! { CheckIcon {} },
                DayDot::Partial => rsx! { HalfIcon {} },
                DayDot::Missed => rsx! { CrossIcon {} },
                DayDot::NotDue => rsx! {},
            }
        }
    }
}

/// Category scores, weakest first. Groups replace categories once #107 lands.
#[component]
fn CategorySplit(report: Rc<Report>) -> Element {
    let i18n = use_i18n();
    rsx! {
        section { class: "report-block",
            h2 { class: "report-block-title", {i18n.t("reports-category")} }
            for (category, score) in categories_weakest_first(&report) {
                MeterRow { key: "{category.0}",
                    label: i18n.t(&format!("category-{}", category.0)),
                    percent: score.percent,
                    band: score.percent.map(|percent| report.bands.band(percent)),
                    detail: i18n.t_with("reports-category-due", &[("due", FluentValue::from(score.due))]),
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
        section { class: "report-block report-not-due",
            h2 { class: "report-block-title", {i18n.t("reports-not-due")} }
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
