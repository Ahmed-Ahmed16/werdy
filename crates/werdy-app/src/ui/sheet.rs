//! The daily sheet: one surface of foldable slots, with a strip of slot chips.

use std::collections::BTreeMap;

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{SheetRow, SheetSlot, SlotKey, WerdyDay};
use werdy_store::DeedName;

use super::context::{use_actions, use_i18n};
use super::icons::{CheckIcon, FoldChevron};
use super::marks::{CountEntry, CountTag, MarkButton};
use crate::state::{AppState, Tap};
use crate::{platform, state};

/// Slots folded or opened by hand, per day. Kept in memory only: whether a
/// hand fold should last to the next day is still open on #73.
type FoldChoices = BTreeMap<(WerdyDay, SlotKey), bool>;

/// All slots of the day on screen, in slot order. No slot is ever hidden.
#[component]
pub fn Sheet() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let actions = use_actions();
    let mut choices = use_signal(FoldChoices::new);
    let (sheet, showing_unmarked, open_count) = {
        let app = state.read();
        (
            app.sheet_on_screen(),
            app.showing_unmarked(),
            app.summary().open,
        )
    };
    let (welcome, milestone, is_today) = {
        let app = state.read();
        if app.is_today() && !super::sharing::read_only() {
            (app.welcome_visible, app.milestone_notice(), true)
        } else {
            (false, None, app.is_today())
        }
    };
    let saying = if welcome {
        crate::consistency_data::notices()
            .ok()
            .and_then(|data| data.sayings.into_iter().find(|entry| entry.is_verified()))
    } else {
        None
    };
    let viewed_day = sheet.day;
    let keys: Vec<SlotKey> = sheet.slots.iter().map(|slot| slot.slot.clone()).collect();
    let current = if is_today {
        let (hour, minute) = platform::local_time();
        let order = &state.read().slot_order;
        nearest_slot(&keys, order, state::current_slot(hour, minute)).cloned()
    } else {
        None
    };
    let opens: Vec<bool> = sheet
        .slots
        .iter()
        .map(|slot| {
            let choice = choices
                .read()
                .get(&(viewed_day, slot.slot.clone()))
                .copied();
            slot_open(slot.folded, choice)
        })
        .collect();
    let open_all = fold_all_opens(&opens);

    let mut last_scroll_day = use_signal(|| None::<(WerdyDay, WerdyDay)>);
    use_effect(move || {
        let app = state.read();
        let day = (app.today, app.viewing);
        if last_scroll_day() == Some(day) {
            return;
        }
        if !app.is_today() {
            last_scroll_day.set(Some(day));
            return;
        }
        if !should_scroll_to_slot(app.is_today(), welcome, milestone.is_some()) {
            return;
        }
        last_scroll_day.set(Some(day));
        let (hour, minute) = platform::local_time();
        let shown: Vec<SlotKey> = app
            .sheet_on_screen()
            .slots
            .into_iter()
            .map(|slot| slot.slot)
            .collect();
        let wanted = state::current_slot(hour, minute);
        if let Some(slot) = nearest_slot(&shown, &app.slot_order, wanted) {
            platform::scroll_slot(&slot.0);
        }
    });

    rsx! {
        section { class: "sheet",
            if welcome {
                div { class: "sheet-notice",
                    p { {i18n.t("sheet-welcome-back")} }
                    if let Some(saying) = saying {
                        p { class: "sheet-saying",
                            {i18n.t_with("sheet-saying", &[("text", FluentValue::from(match i18n.language {
                                crate::i18n::Language::Arabic => saying.text_ar,
                                crate::i18n::Language::English => saying.text_en,
                            }))])}
                        }
                        small { {i18n.t_with("sheet-saying-source", &[("source", FluentValue::from(saying.source))])} }
                    }
                }
            }
            if let Some(level) = milestone {
                div { class: "sheet-notice sheet-milestone",
                    p { {i18n.t_with("sheet-milestone", &[("days", FluentValue::from(level))])} }
                    button { class: "notice-dismiss", aria_label: i18n.t("sheet-dismiss-milestone"),
                        onclick: move |_| actions.change(|app| app.dismiss_milestone(level)),
                        {i18n.t("sheet-dismiss-milestone")}
                    }
                }
            }
            if showing_unmarked {
                div { class: "sheet-notice sheet-filter", role: "status",
                    p {
                        {i18n.t(unmarked_notice(sheet.slots.len(), state.read().sheet().slots.len()))}
                    }
                    button { class: "notice-dismiss",
                        onclick: move |_| actions.view(AppState::toggle_unmarked),
                        {i18n.t("sheet-show-all")}
                    }
                }
            } else if sheet.slots.is_empty() {
                p { class: "sheet-empty", {i18n.t("sheet-empty")} }
            }
            if !sheet.slots.is_empty() {
                nav { class: "slot-strip", aria_label: i18n.t("sheet-slots"),
                    for slot in sheet.slots.iter() {
                        SlotChip {
                            key: "{slot.slot.0}",
                            slot: slot.clone(),
                            current: current.as_ref() == Some(&slot.slot),
                            on_jump: move |key: SlotKey| {
                                choices.write().insert((viewed_day, key.clone()), true);
                                platform::scroll_slot(&key.0);
                            },
                        }
                    }
                }
                div { class: "sheet-tools",
                    if is_today && !showing_unmarked && open_count > 0 && !super::sharing::read_only() {
                        button { class: "text-button open-only",
                            onclick: move |_| actions.view(AppState::toggle_unmarked),
                            {i18n.t_with("sheet-open-only", &[("count", FluentValue::from(open_count))])}
                        }
                    }
                    button { class: "text-button fold-all",
                        onclick: move |_| {
                            let mut choices = choices.write();
                            for key in &keys {
                                choices.insert((viewed_day, key.clone()), open_all);
                            }
                        },
                        {i18n.t(if open_all { "sheet-open-all" } else { "sheet-fold-all" })}
                    }
                }
                div { class: "sheet-surface",
                    for (slot, open) in sheet.slots.into_iter().zip(opens) {
                        SlotGroup {
                            key: "{viewed_day:?}-{slot.slot.0}",
                            slot,
                            open,
                            on_toggle: move |key: SlotKey| {
                                choices.write().insert((viewed_day, key), !open);
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Text key for the notice above a sheet filtered to open deeds. A day with
/// no due deeds says so, rather than claiming every deed has a mark.
fn unmarked_notice(shown_slots: usize, due_slots: usize) -> &'static str {
    if due_slots == 0 {
        "sheet-empty"
    } else if shown_slots == 0 {
        "sheet-unmarked-none"
    } else {
        "sheet-unmarked-only"
    }
}

fn should_scroll_to_slot(is_today: bool, welcome: bool, milestone: bool) -> bool {
    is_today && !welcome && !milestone
}

/// Marked deeds and all deeds in a slot. A miss counts as marked: the person
/// has dealt with it.
fn slot_progress(rows: &[SheetRow]) -> (usize, usize) {
    let marked = rows.iter().filter(|row| row.mark.is_some()).count();
    (marked, rows.len())
}

/// Whether a slot shows its deeds. A complete slot folds by itself; a hand
/// choice on the same day wins either way.
fn slot_open(complete: bool, choice: Option<bool>) -> bool {
    match choice {
        Some(open) => open,
        None => !complete,
    }
}

/// True when the one fold button should open every slot, which is only when
/// every slot is folded. Otherwise it folds them all.
fn fold_all_opens(open: &[bool]) -> bool {
    !open.is_empty() && open.iter().all(|open| !open)
}

/// The slot that holds the wanted time, or the next one that has deeds, or the
/// last one. Slots missing from `order` sort after it, as on the sheet.
fn nearest_slot<'a>(shown: &'a [SlotKey], order: &[SlotKey], wanted: &str) -> Option<&'a SlotKey> {
    let position = |key: &SlotKey| {
        order
            .iter()
            .position(|slot| slot == key)
            .unwrap_or(order.len())
    };
    let wanted_index = order.iter().position(|slot| slot.0 == wanted).unwrap_or(0);
    shown
        .iter()
        .find(|slot| slot.0 == wanted)
        .or_else(|| shown.iter().find(|slot| position(slot) >= wanted_index))
        .or_else(|| shown.last())
}

/// A chip in the slot strip: name and marked count. Tapping it opens the slot
/// and scrolls to it.
#[component]
fn SlotChip(slot: SheetSlot, current: bool, on_jump: EventHandler<SlotKey>) -> Element {
    let i18n = use_i18n();
    let name = i18n.t(&format!("slot-{}", slot.slot.0));
    let (marked, total) = slot_progress(&slot.rows);
    let args = [
        ("slot", FluentValue::from(name)),
        ("marked", FluentValue::from(marked)),
        ("total", FluentValue::from(total)),
    ];
    let key = slot.slot.clone();

    rsx! {
        button {
            class: "slot-chip",
            "data-current": if current { "true" } else { "false" },
            "data-complete": if slot.folded { "true" } else { "false" },
            aria_current: if current { "time" } else { "false" },
            aria_label: i18n.t_with("slot-chip-label", &args),
            onclick: move |_| on_jump.call(key.clone()),
            if slot.folded { span { class: "chip-done", CheckIcon {} } }
            span { {i18n.t_with("slot-chip", &args)} }
        }
    }
}

/// One time slot: a thin heading that stays in view, and its deed rows.
/// Type groups (#107) will split the rows inside a slot; until then a slot
/// holds one plain list.
#[component]
fn SlotGroup(slot: SheetSlot, open: bool, on_toggle: EventHandler<SlotKey>) -> Element {
    let i18n = use_i18n();
    let name = i18n.t(&format!("slot-{}", slot.slot.0));
    let (marked, total) = slot_progress(&slot.rows);
    let count = i18n.t_with(
        "slot-count",
        &[
            ("marked", FluentValue::from(marked)),
            ("total", FluentValue::from(total)),
        ],
    );
    let has_late_mark = slot.rows.iter().any(|row| row.added_later);
    let key = slot.slot.clone();

    rsx! {
        div { class: "slot",
            id: "slot-{slot.slot.0}",
            "data-open": if open { "true" } else { "false" },
            h2 { class: "slot-heading",
                button {
                    class: "slot-toggle",
                    aria_expanded: if open { "true" } else { "false" },
                    aria_controls: "slot-rows-{slot.slot.0}",
                    onclick: move |_| on_toggle.call(key.clone()),
                    FoldChevron {}
                    span { class: "slot-name", "{name}" }
                    if !open && has_late_mark {
                        span { class: "late-sign", {i18n.t("mark-added-later")} }
                    }
                    span { class: "slot-count",
                        if slot.folded { span { class: "chip-done", CheckIcon {} } }
                        "{count}"
                    }
                }
            }
            if open {
                ul { class: "slot-rows", id: "slot-rows-{slot.slot.0}",
                    for row in slot.rows {
                        DeedRow { key: "{row.deed_id.0}", row }
                    }
                }
            }
        }
    }
}

/// One deed: its name and its mark controls. Yes or no deeds get done,
/// partial and missed; counted deeds get done, a typed count and missed.
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
    let target = row.target.as_ref().map(|target| target.amount());

    rsx! {
        li { class: "deed-row",
            div { class: "deed-identity",
                span { class: "deed-name", "{name}" }
                if let Some(target) = target {
                    CountTag { deed_id, mark, target }
                }
                if row.added_later {
                    span {
                        class: "late-sign",
                        aria_label: i18n.t("mark-added-later"),
                        {i18n.t("mark-added-later")}
                    }
                }
            }
            div { class: "deed-controls",
                MarkButton { deed_id, tap: Tap::Done, mark, target }
                if let Some(target) = target {
                    CountEntry { deed_id, mark, target }
                } else {
                    MarkButton { deed_id, tap: Tap::Partial, mark, target: None }
                }
                MarkButton { deed_id, tap: Tap::Missed, mark, target }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use werdy_core::{DeedId, Mark, SheetRow, SlotKey};

    use super::{
        fold_all_opens, nearest_slot, should_scroll_to_slot, slot_open, slot_progress,
        unmarked_notice,
    };

    #[test]
    fn unmarked_notice_distinguishes_no_due_deeds_from_all_marked() {
        // Cases: no due deeds, all marked, some open.
        for (shown, due, expected) in [
            (0, 0, "sheet-empty"),
            (0, 3, "sheet-unmarked-none"),
            (2, 3, "sheet-unmarked-only"),
        ] {
            assert_eq!(unmarked_notice(shown, due), expected);
        }
    }

    #[test]
    fn notices_keep_sheet_at_the_top_until_they_are_gone() {
        assert!(!should_scroll_to_slot(true, true, false));
        assert!(!should_scroll_to_slot(true, false, true));
        assert!(!should_scroll_to_slot(true, true, true));
        assert!(!should_scroll_to_slot(false, false, false));
        assert!(should_scroll_to_slot(true, false, false));
    }

    fn row(id: u64, mark: Option<Mark>) -> SheetRow {
        SheetRow {
            deed_id: DeedId(id),
            mark,
            added_later: false,
            target: None,
        }
    }

    #[test]
    fn slot_progress_counts_every_mark_including_misses() {
        assert_eq!(slot_progress(&[]), (0, 0));
        let rows = [
            row(1, None),
            row(2, Some(Mark::Done)),
            row(3, Some(Mark::Missed)),
            row(4, Some(Mark::Count(0))),
            row(5, Some(Mark::partial_without_target())),
            row(6, Some(Mark::Count(120))),
        ];
        assert_eq!(slot_progress(&rows), (5, 6));
    }

    #[test]
    fn a_complete_slot_folds_until_opened_and_a_hand_choice_wins() {
        assert!(slot_open(false, None));
        assert!(!slot_open(true, None));
        assert!(slot_open(true, Some(true)));
        assert!(!slot_open(false, Some(false)));
        assert!(!slot_open(true, Some(false)));
        assert!(slot_open(false, Some(true)));
    }

    #[test]
    fn the_fold_all_button_opens_only_when_every_slot_is_folded() {
        assert!(!fold_all_opens(&[true, false, true]));
        assert!(!fold_all_opens(&[true]));
        assert!(fold_all_opens(&[false, false]));
        // With no slots there is nothing to open.
        assert!(!fold_all_opens(&[]));
    }

    fn keys(names: &[&str]) -> Vec<SlotKey> {
        names
            .iter()
            .map(|name| SlotKey((*name).to_owned()))
            .collect()
    }

    #[test]
    fn the_current_slot_is_the_wanted_one_or_the_next_one_with_deeds() {
        let order = keys(&[
            "fajr", "morning", "dhuhr", "asr", "maghrib", "isha", "night",
        ]);
        let shown = keys(&["fajr", "dhuhr", "isha"]);
        assert_eq!(nearest_slot(&shown, &order, "dhuhr"), Some(&shown[1]));
        assert_eq!(nearest_slot(&shown, &order, "asr"), Some(&shown[2]));
        assert_eq!(nearest_slot(&shown, &order, "morning"), Some(&shown[1]));
        // Past the last slot with deeds, the last one stays current.
        assert_eq!(nearest_slot(&shown, &order, "night"), Some(&shown[2]));
        assert_eq!(nearest_slot(&[], &order, "night"), None);
    }
}
