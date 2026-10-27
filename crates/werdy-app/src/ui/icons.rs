//! Mark shapes, so colour is never the only signal. All use `currentColor`.

use dioxus::prelude::*;

/// Check mark for Done.
#[component]
pub fn CheckIcon() -> Element {
    rsx! {
        svg { class: "icon", view_box: "0 0 24 24", "aria-hidden": "true",
            path {
                d: "M5 12.5l4.5 4.5L19 7.5",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.6",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

/// Half filled circle for Partial.
#[component]
pub fn HalfIcon() -> Element {
    rsx! {
        svg { class: "icon", view_box: "0 0 24 24", "aria-hidden": "true",
            circle {
                cx: "12",
                cy: "12",
                r: "7.5",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.2",
            }
            path { d: "M12 4.5a7.5 7.5 0 0 0 0 15z", fill: "currentColor" }
        }
    }
}

/// Cross for Missed.
#[component]
pub fn CrossIcon() -> Element {
    rsx! {
        svg { class: "icon", view_box: "0 0 24 24", "aria-hidden": "true",
            path {
                d: "M7 7l10 10M17 7L7 17",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.6",
                stroke_linecap: "round",
            }
        }
    }
}

/// Chevron that points back in reading order. CSS mirrors it in right to left.
#[component]
pub fn ChevronBack() -> Element {
    rsx! {
        svg { class: "icon chevron", view_box: "0 0 24 24", "aria-hidden": "true",
            path {
                d: "M15 5l-7 7 7 7",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.4",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

/// Chevron that points forward in reading order. CSS mirrors it in right to left.
#[component]
pub fn ChevronForward() -> Element {
    rsx! {
        svg { class: "icon chevron", view_box: "0 0 24 24", "aria-hidden": "true",
            path {
                d: "M9 5l7 7-7 7",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.4",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

/// Minus sign for the stepper.
#[component]
pub fn MinusIcon() -> Element {
    rsx! {
        svg { class: "icon", view_box: "0 0 24 24", "aria-hidden": "true",
            path {
                d: "M6 12h12",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.6",
                stroke_linecap: "round",
            }
        }
    }
}

/// Plus sign for the stepper.
#[component]
pub fn PlusIcon() -> Element {
    rsx! {
        svg { class: "icon", view_box: "0 0 24 24", "aria-hidden": "true",
            path {
                d: "M6 12h12M12 6v12",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.6",
                stroke_linecap: "round",
            }
        }
    }
}
