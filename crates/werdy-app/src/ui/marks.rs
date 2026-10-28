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
    let kind = count_kind(mark, target);
    let mut editing = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let mut invalid = use_signal(|| false);
    let mut save = move || {
        // Closing after a valid save stops the following blur from saving twice.
        // Invalid text keeps the field open with the person's draft.
        if !editing() {
            return;
        }
        let mut accepted = false;
        actions.change(|state| accepted = state.type_count(deed_id, &draft()));
        invalid.set(!accepted);
        if accepted {
            editing.set(false);
        }
    };

    rsx! {
        div { class: "stepper", "data-kind": kind,
            button {
                class: "step-button",
                aria_label: i18n.t("count-minus"),
                disabled: count == 0 || super::sharing::read_only(),
                onclick: move |_| actions.change(|state| state.step(deed_id, Step::Minus)),
                MinusIcon {}
            }
            div { class: "count-entry",
                if editing() {
                    input {
                        id: "count-input-{deed_id.0}",
                        class: "step-count count-input",
                        r#type: "number", inputmode: "numeric", min: "0", max: "{u32::MAX}", step: "1",
                        aria_label: i18n.t("count-edit"),
                        aria_describedby: "count-progress-{deed_id.0}",
                        value: "{draft}",
                        onmounted: move |_| select_count_input(deed_id),
                        oninput: move |event| draft.set(event.value()),
                        onblur: move |_| save(),
                        onkeydown: move |event| match event.key() {
                            Key::Enter => { event.prevent_default(); save(); }
                            Key::Escape => { event.prevent_default(); editing.set(false); invalid.set(false); }
                            _ => {}
                        },
                    }
                    span { id: "count-progress-{deed_id.0}", class: "count-target", "{progress}" }
                } else {
                    button {
                        class: "step-count", aria_live: "polite",
                        disabled: super::sharing::read_only(),
                        aria_label: i18n.t("count-edit"),
                        onclick: move |_| {
                            draft.set(count.to_string());
                            invalid.set(false);
                            editing.set(true);
                        },
                        "{progress}"
                    }
                }
                if invalid() {
                    span { class: "count-error", role: "alert",
                        {i18n.t_with("count-invalid", &[("max", FluentValue::from(u32::MAX))])}
                    }
                }
            }
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

fn count_kind(mark: Option<Mark>, target: u32) -> &'static str {
    match mark {
        None => "open",
        Some(mark) => match shown_count(Some(mark)) {
            0 => "missed",
            count if count >= target => "done",
            _ => "partial",
        },
    }
}

fn select_count_input(deed_id: DeedId) {
    let _ = document::eval(&format!(
        "const inputId = 'count-input-{}';\n{}",
        deed_id.0,
        include_str!("counter-input.js")
    ));
}

#[cfg(test)]
mod tests {
    use super::count_kind;

    #[test]
    fn counter_colors_follow_zero_partial_and_full_values() {
        use werdy_core::Mark;
        // An unmarked counter is still open; only an explicit zero is a miss.
        assert_eq!(count_kind(None, 100), "open");
        for (count, expected) in [
            (0, "missed"),
            (1, "partial"),
            (99, "partial"),
            (100, "done"),
            (101, "done"),
            (u32::MAX, "done"),
        ] {
            assert_eq!(count_kind(Some(Mark::Count(count)), 100), expected);
        }
    }
}
