//! The daily sheet: slot groups and deed rows.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{SheetRow, SheetSlot};
use werdy_store::DeedName;

use super::context::use_i18n;
use super::icons::CheckIcon;
use super::marks::{CountStepper, MarkButton};
use crate::state::{AppState, Tap};
use crate::{platform, state};

/// All slots of the day on screen, in slot order.
#[component]
pub fn Sheet() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let sheet = state.read().sheet();
    let viewed_day = sheet.day;
    let mut last_scroll_day = use_signal(|| None::<(werdy_core::WerdyDay, werdy_core::WerdyDay)>);
    use_effect(move || {
        let app = state.read();
        let day = (app.today, app.viewing);
        if last_scroll_day() == Some(day) {
            return;
        }
        last_scroll_day.set(Some(day));
        if app.is_today() {
            let (hour, minute) = platform::local_time();
            let wanted = state::current_slot(hour, minute);
            let slots = app.sheet().slots;
            let order = &app.slot_order;
            let target = slots
                .iter()
                .find(|slot| slot.slot.0 == wanted)
                .or_else(|| {
                    let wanted_index = order.iter().position(|slot| slot.0 == wanted).unwrap_or(0);
                    slots.iter().find(|slot| {
                        order
                            .iter()
                            .position(|key| key == &slot.slot)
                            .unwrap_or(order.len())
                            >= wanted_index
                    })
                })
                .or_else(|| slots.last());
            if let Some(slot) = target {
                platform::scroll_slot(&slot.slot.0);
            }
        }
    });

    rsx! {
        section { class: "sheet",
            if sheet.slots.is_empty() {
                p { class: "sheet-empty", {i18n.t("sheet-empty")} }
            }
            for slot in sheet.slots {
                SlotGroup { key: "{viewed_day:?}-{slot.slot.0}", slot }
            }
        }
    }
}

/// One time slot. A fully done slot folds into one line until it is opened.
#[component]
fn SlotGroup(slot: SheetSlot) -> Element {
    let i18n = use_i18n();
    let mut opened = use_signal(|| false);
    let name = i18n.t(&format!("slot-{}", slot.slot.0));
    let has_late_mark = slot.rows.iter().any(|row| row.added_later);

    if slot.folded && !opened() {
        let line = i18n.t_with("slot-folded", &[("slot", FluentValue::from(name))]);
        return rsx! {
            button {
                id: "slot-{slot.slot.0}",
                class: "slot-folded",
                aria_expanded: "false",
                onclick: move |_| opened.set(true),
                span { class: "slot-folded-mark", CheckIcon {} }
                span { class: "slot-folded-text", "{line}" }
                if has_late_mark {
                    span {
                        class: "late-sign",
                        aria_label: i18n.t("mark-added-later"),
                        {i18n.t("mark-added-later")}
                    }
                }
                span { class: "slot-folded-show", {i18n.t("slot-show")} }
            }
        };
    }

    rsx! {
        div { class: "slot",
            id: "slot-{slot.slot.0}",
            h2 { class: "slot-title", "{name}" }
            ul { class: "slot-rows",
                for row in slot.rows {
                    DeedRow { key: "{row.deed_id.0}", row }
                }
            }
        }
    }
}

/// One deed: its name and its mark controls.
#[component]
fn DeedRow(row: SheetRow) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let name = match state.read().deed_name(row.deed_id) {
        Some(DeedName::Key(key)) => i18n.t(key),
        Some(DeedName::Text(text)) => text.clone(),
        None => String::new(),
    };
    let deed_id = row.deed_id;
    let mark = row.mark;

    rsx! {
        li { class: "deed-row",
            div { class: "deed-identity",
                span { class: "deed-name", "{name}" }
                if row.added_later {
                    span {
                        class: "late-sign",
                        aria_label: i18n.t("mark-added-later"),
                        {i18n.t("mark-added-later")}
                    }
                }
            }
            div { class: "deed-controls",
                if let Some(target) = row.target {
                    CountStepper { deed_id, mark, target: target.amount() }
                } else {
                    MarkButton { deed_id, tap: Tap::Done, mark }
                    MarkButton { deed_id, tap: Tap::Partial, mark }
                }
                MarkButton { deed_id, tap: Tap::Missed, mark }
            }
        }
    }
}
