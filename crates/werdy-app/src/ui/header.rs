//! Header: the date on screen, day arrows, and the language switch.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{Weekday, WerdyDay};

use super::context::{I18n, use_actions, use_i18n};
use super::icons::{ChevronBack, ChevronForward};
use crate::state::AppState;

/// Date line with previous and next day arrows. Next is off on today.
#[component]
pub fn DayHeader(on_open_note: EventHandler<WerdyDay>) -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let (viewing, is_today, hijri_offset, noted, note_late) = {
        let state = state.read();
        (
            state.viewing,
            state.is_today(),
            state.data.settings.calendar.offset_on(state.viewing),
            state.viewing_note().is_some(),
            state
                .viewing_note()
                .is_some_and(|note| note.is_late_for(state.viewing)),
        )
    };
    let date = date_text(&i18n, viewing);
    let hijri = hijri_text(&i18n, viewing, hijri_offset);
    let other = i18n.language.other();

    rsx! {
        header { class: "day-header",
            div { class: "day-nav",
                button {
                    class: "icon-button",
                    aria_label: i18n.t("sheet-previous-day"),
                    title: i18n.t("sheet-previous-day"),
                    onclick: move |_| actions.view(AppState::show_previous_day),
                    ChevronBack {}
                }
                div { class: "day-dates",
                    h1 { class: "day-title", "{date}" }
                    p { class: "hijri-date", "{hijri}" }
                    if noted { span { class: "note-sign", role: "img", aria_label: i18n.t("note-sign"), {i18n.t("note-sign-symbol")} } }
                    if note_late { span { class: "late-sign", aria_label: i18n.t("mark-added-later"), {i18n.t("mark-added-later")} } }
                }
                button {
                    class: "icon-button",
                    aria_label: i18n.t("sheet-next-day"),
                    title: i18n.t("sheet-next-day"),
                    disabled: is_today,
                    onclick: move |_| actions.view(AppState::show_next_day),
                    ChevronForward {}
                }
            }
            div { class: "header-tools",
                button { id: "note-open", class: "text-button", onclick: move |_| on_open_note.call(viewing), {i18n.t("note-button")} }
                if !is_today {
                    button {
                        class: "text-button",
                        onclick: move |_| actions.view(AppState::show_today),
                        {i18n.t("sheet-back-to-today")}
                    }
                }
                button {
                    class: "text-button",
                    lang: other.code(),
                    onclick: move |_| actions.change(|state| state.set_language(other.code())),
                    {i18n.t("language-switch")}
                }
            }
        }
    }
}

fn hijri_text(i18n: &I18n, day: WerdyDay, offset: werdy_core::HijriOffset) -> String {
    let Ok(date) = day.hijri(offset) else {
        return i18n.t("sheet-hijri-unavailable");
    };
    let month = i18n.t(&format!("hijri-month-{}", date.month));
    i18n.t_with(
        "sheet-hijri-date",
        &[
            ("day", FluentValue::from(date.day)),
            ("month", FluentValue::from(month)),
            ("year", FluentValue::from(i18n.num(date.year))),
        ],
    )
}

pub(super) fn date_text(i18n: &I18n, day: WerdyDay) -> String {
    let (year, month, day_of_month) = day.date_parts();
    let weekday = i18n.t(weekday_key(day.weekday()));
    let month_name = i18n.t(&format!("month-{month}"));
    i18n.t_with(
        "sheet-date",
        &[
            ("weekday", FluentValue::from(weekday)),
            ("day", FluentValue::from(day_of_month)),
            ("month", FluentValue::from(month_name)),
            // As text, so the year is never formatted with a group separator.
            ("year", FluentValue::from(i18n.num(year))),
        ],
    )
}

fn weekday_key(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "weekday-monday",
        Weekday::Tuesday => "weekday-tuesday",
        Weekday::Wednesday => "weekday-wednesday",
        Weekday::Thursday => "weekday-thursday",
        Weekday::Friday => "weekday-friday",
        Weekday::Saturday => "weekday-saturday",
        Weekday::Sunday => "weekday-sunday",
    }
}
