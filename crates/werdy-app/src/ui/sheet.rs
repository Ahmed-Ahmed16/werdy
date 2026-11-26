//! The daily sheet: one surface of foldable slots, with a strip of slot chips.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{RowTag, SheetRow, SheetSlot, SlotKey, WerdyDay};
use werdy_store::DeedName;

use super::context::{use_actions, use_i18n};
use super::icons::{CheckIcon, FoldChevron};
use super::marks::{CountEntry, CountTag, MarkButton};
use crate::platform;
use crate::sheet_view::{FoldChoices, SlotProgress, fold_all_opens, nearest_slot, section_views};
use crate::starter::place_name_key;
use crate::state::{AppState, Tap};

/// The day's sections in sheet order, with an optional unmarked filter.
#[component]
pub fn Sheet() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let clock_slot = use_context::<Signal<&'static str>>();
    let actions = use_actions();
    let (sheet, full_sheet, today, open_count) = {
        let app = state.read();
        (
            app.sheet_on_screen(),
            app.showing_unmarked().then(|| app.sheet()),
            app.today,
            app.summary().open,
        )
    };
    let mut choices = use_signal(|| FoldChoices::new(today));
    use_effect(move || {
        let today = state.read().today;
        if !choices.peek().is_for(today) {
            choices.write().reset_for(today);
        }
    });
    let (welcome, milestone, is_today) = {
        let app = state.read();
        if app.is_today() && !super::sharing::read_only() {
            (app.welcome_visible, app.milestone_notice(), true)
        } else {
            (false, None, app.is_today())
        }
    };
    let whats_new = {
        let app = state.read();
        if app.is_today() && !super::sharing::read_only() {
            app.whats_new.clone()
        } else {
            Vec::new()
        }
    };
    let has_whats_new = !whats_new.is_empty();
    let saying = if welcome {
        crate::consistency_data::notices()
            .ok()
            .and_then(|data| data.sayings.into_iter().find(|entry| entry.is_verified()))
    } else {
        None
    };
    let viewed_day = sheet.day;
    let wanted = if is_today { Some(clock_slot()) } else { None };
    let sections = section_views(sheet, full_sheet.as_ref(), wanted);
    let keys: Vec<_> = sections
        .iter()
        .map(|section| section.slot.slot.clone())
        .collect();
    let current = sections
        .iter()
        .find(|section| section.current)
        .map(|section| section.slot.slot.clone());
    use_effect(use_reactive(
        (&current, &i18n.language, &viewed_day),
        |(current, _, _)| {
            if current.is_some() {
                platform::reveal_current_chip();
            }
        },
    ));

    let opens: Vec<bool> = sections
        .iter()
        .map(|section| {
            let slot = &section.slot;
            choices
                .read()
                .open(today, viewed_day, &slot.slot, slot.all_entered)
        })
        .collect();
    let open_all = fold_all_opens(&opens);

    let mut pending_jump = use_signal(|| None::<SlotKey>);
    use_effect(move || {
        if let Some(key) = pending_jump() {
            pending_jump.set(None);
            platform::scroll_slot(&key.0);
        }
    });

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
        if !should_scroll_to_slot(
            app.is_today(),
            welcome || has_whats_new,
            milestone.is_some(),
        ) {
            return;
        }
        last_scroll_day.set(Some(day));
        let shown: Vec<SlotKey> = app
            .sheet_on_screen()
            .slots
            .into_iter()
            .map(|slot| slot.slot)
            .collect();
        let wanted = *clock_slot.peek();
        if let Some(slot) = nearest_slot(&shown, &app.slot_order, wanted) {
            platform::scroll_slot(&slot.0);
        }
    });

    rsx! {
        section { class: "sheet",
            super::sharing::SheetWatchRequests {}
            if has_whats_new {
                section { class: "sheet-notice whats-new", role: "region", aria_label: i18n.t("whats-new-region"),
                    h2 { {i18n.t("whats-new-title")} }
                    for release in &whats_new {
                        div {
                            h3 { {i18n.t_with("whats-new-version", &[("version", FluentValue::from(i18n.num(&release.version)))])} }
                            ul {
                                for note in &release.notes {
                                    li { {i18n.t(note)} }
                                }
                            }
                        }
                    }
                    button { id: "whats-new-close", class: "notice-dismiss",
                        onclick: move |_| actions.change(AppState::acknowledge_whats_new),
                        {i18n.t("whats-new-close")}
                    }
                }
            }
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
            if let Some(full_sheet) = &full_sheet {
                div { class: "sheet-notice sheet-filter", role: "status",
                    p {
                        {i18n.t(unmarked_notice(sections.len(), full_sheet.slots.len()))}
                    }
                    button { class: "notice-dismiss",
                        onclick: move |_| actions.view(AppState::toggle_unmarked),
                        {i18n.t("sheet-show-all")}
                    }
                }
            } else if sections.is_empty() {
                p { class: "sheet-empty", {i18n.t("sheet-empty")} }
            }
            if !sections.is_empty() {
                nav { class: "slot-strip", aria_label: i18n.t("sheet-slots"),
                    onmounted: move |_| observe_strip_height(),
                    for (section, open) in sections.iter().zip(opens.iter().copied()) {
                        SlotChip {
                            key: "{section.slot.slot.0}",
                            slot: section.slot.slot.clone(),
                            progress: section.progress,
                            current: section.current,
                            on_jump: move |key: SlotKey| {
                                if !open {
                                    choices.write().set(today, viewed_day, [&key], true);
                                }
                                pending_jump.set(Some(key));
                            },
                        }
                    }
                }
                div { class: "sheet-tools",
                    if is_today && full_sheet.is_none() && open_count > 0 && !super::sharing::read_only() {
                        button { class: "text-button open-only",
                            onclick: move |_| actions.view(AppState::toggle_unmarked),
                            {i18n.t_with("sheet-open-only", &[("count", FluentValue::from(open_count))])}
                        }
                    }
                    button { class: "text-button fold-all",
                        onclick: move |_| {
                            choices.write().set(today, viewed_day, &keys, open_all);
                        },
                        {i18n.t(if open_all { "sheet-open-all" } else { "sheet-fold-all" })}
                    }
                }
                div { class: "sheet-surface",
                    for (section, open) in sections.into_iter().zip(opens) {
                        SlotGroup {
                            key: "{viewed_day:?}-{section.slot.slot.0}",
                            slot: section.slot,
                            progress: section.progress,
                            open,
                            on_toggle: move |key: SlotKey| {
                                choices.write().set(today, viewed_day, [&key], !open);
                            },
                        }
                    }
                }
            }
        }
    }
}

fn observe_strip_height() {
    // Sticky headings and scroll targets both need the height after chips wrap.
    let _ = document::eval(
        r#"
        const strip = document.querySelector(".slot-strip");
        if (strip) {
            const sheet = strip.parentElement;
            const update = () => sheet.style.setProperty("--strip-block", `${strip.offsetHeight}px`);
            update();
            const observer = new ResizeObserver(() => {
                if (strip.isConnected) {
                    update();
                } else {
                    observer.disconnect();
                }
            });
            observer.observe(strip);
        }
        "#,
    );
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

/// Text key for a row's quiet schedule tag.
fn row_tag_key(tag: RowTag) -> &'static str {
    match tag {
        RowTag::ThisWeek => "sheet-this-week",
        RowTag::WhenItComesUp => "sheet-when-it-comes-up",
    }
}

/// A chip in the slot strip: name and marked count. Tapping it opens the slot
/// and scrolls to it.
#[component]
fn SlotChip(
    slot: SlotKey,
    progress: SlotProgress,
    current: bool,
    on_jump: EventHandler<SlotKey>,
) -> Element {
    let i18n = use_i18n();
    let name = i18n.t(&place_name_key(&slot.0));
    let args = [
        ("slot", FluentValue::from(name)),
        ("marked", FluentValue::from(progress.marked)),
        ("total", FluentValue::from(progress.total)),
    ];

    rsx! {
        button {
            class: "slot-chip",
            "data-current": if current { "true" } else { "false" },
            "data-complete": if progress.complete { "true" } else { "false" },
            aria_current: if current { "time" } else { "false" },
            aria_label: i18n.t_with("slot-chip-label", &args),
            onclick: move |_| on_jump.call(slot.clone()),
            if progress.complete { span { class: "chip-done", CheckIcon {} } }
            span { {i18n.t_with("slot-chip", &args)} }
        }
    }
}

/// One place on the sheet, a time slot or a group (0045): a thin heading
/// that stays in view, and its deed rows.
#[component]
fn SlotGroup(
    slot: SheetSlot,
    progress: SlotProgress,
    open: bool,
    on_toggle: EventHandler<SlotKey>,
) -> Element {
    let i18n = use_i18n();
    let name = i18n.t(&place_name_key(&slot.slot.0));
    let count = i18n.t_with(
        "slot-count",
        &[
            ("marked", FluentValue::from(progress.marked)),
            ("total", FluentValue::from(progress.total)),
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
                        if progress.complete { span { class: "chip-done", CheckIcon {} } }
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
        li {
            div { class: "deed-row", role: "group", aria_labelledby: "deed-name-{deed_id.0}",
                div { class: "deed-identity",
                    span { class: "deed-name", id: "deed-name-{deed_id.0}", "{name}" }
                    if let Some(target) = target {
                        CountTag { deed_id, mark, target }
                    }
                    if let Some(tag) = row.tag {
                        span { class: "schedule-tag", {i18n.t(row_tag_key(tag))} }
                    }
                    if row.added_later {
                        span {
                            class: "late-sign",
                            aria_label: i18n.t("mark-added-later"),
                            {i18n.t("mark-added-later")}
                        }
                    }
                }
                div { class: "deed-controls", dir: "ltr",
                    MarkButton { deed_id, tap: Tap::Missed, mark, target }
                    if let Some(target) = target {
                        CountEntry { deed_id, mark, target }
                    } else {
                        MarkButton { deed_id, tap: Tap::Partial, mark, target: None }
                    }
                    MarkButton { deed_id, tap: Tap::Done, mark, target }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{should_scroll_to_slot, unmarked_notice};

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
}
