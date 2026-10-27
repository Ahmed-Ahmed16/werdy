//! The daily sheet: slot groups and deed rows.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{SheetRow, SheetSlot};
use werdy_store::DeedName;

use super::context::use_i18n;
use super::icons::CheckIcon;
use super::marks::{CountStepper, MarkButton};
use crate::state::{AppState, Tap};

/// All slots of the day on screen, in slot order.
#[component]
pub fn Sheet() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let sheet = state.read().sheet();

    rsx! {
        section { class: "sheet",
            if sheet.slots.is_empty() {
                p { class: "sheet-empty", {i18n.t("sheet-empty")} }
            }
            for slot in sheet.slots {
                SlotGroup { key: "{slot.slot.0}", slot }
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

    if slot.folded && !opened() {
        let line = i18n.t_with("slot-folded", &[("slot", FluentValue::from(name))]);
        return rsx! {
            button {
                class: "slot-folded",
                aria_expanded: "false",
                onclick: move |_| opened.set(true),
                span { class: "slot-folded-mark", CheckIcon {} }
                span { class: "slot-folded-text", "{line}" }
                span { class: "slot-folded-show", {i18n.t("slot-show")} }
            }
        };
    }

    rsx! {
        div { class: "slot",
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
            span { class: "deed-name", "{name}" }
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
