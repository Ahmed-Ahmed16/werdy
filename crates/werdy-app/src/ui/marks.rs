//! Mark buttons and the count stepper on a deed row.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{DeedId, Mark};

use super::context::{use_actions, use_i18n};
use super::icons::{CheckIcon, CrossIcon, HalfIcon, MinusIcon, PlusIcon};
use crate::state::{Step, Tap, is_on, shown_count};

/// One 48 px mark button. A second tap on a button that is on clears the mark.
#[component]
pub fn MarkButton(deed_id: DeedId, tap: Tap, mark: Option<Mark>) -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let (kind, label_key) = match tap {
        Tap::Done => ("done", "mark-done"),
        Tap::Partial => ("partial", "mark-partial"),
        Tap::Missed => ("missed", "mark-missed"),
    };
    let on = is_on(mark, tap);

    rsx! {
        button {
            disabled: super::sharing::read_only(),
            class: "mark-button",
            "data-kind": kind,
            aria_pressed: if on { "true" } else { "false" },
            aria_label: i18n.t(label_key),
            title: i18n.t(label_key),
            onclick: move |_| actions.change(|state| state.tap(deed_id, tap)),
            match tap {
                Tap::Done => rsx! { CheckIcon {} },
                Tap::Partial => rsx! { HalfIcon {} },
                Tap::Missed => rsx! { CrossIcon {} },
            }
        }
    }
}

/// Minus, count and plus for a deed with a target (ADR 0011).
#[component]
pub fn CountStepper(deed_id: DeedId, mark: Option<Mark>, target: u32) -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let count = shown_count(mark);
    let progress = i18n.t_with(
        "count-progress",
        &[
            ("count", FluentValue::from(count)),
            ("target", FluentValue::from(target)),
        ],
    );
    let reached = count >= target;

    rsx! {
        div { class: "stepper", "data-reached": if reached { "true" } else { "false" },
            button {
                class: "step-button",
                aria_label: i18n.t("count-minus"),
                disabled: count == 0 || super::sharing::read_only(),
                onclick: move |_| actions.change(|state| state.step(deed_id, Step::Minus)),
                MinusIcon {}
            }
            span { class: "step-count", aria_live: "polite", "{progress}" }
            button {
                class: "step-button",
                aria_label: i18n.t("count-plus"),
                disabled: super::sharing::read_only(),
                onclick: move |_| actions.change(|state| state.step(deed_id, Step::Plus)),
                PlusIcon {}
            }
        }
    }
}
