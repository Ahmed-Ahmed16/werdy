//! My deeds and the deed editor.

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_core::{
    DEFAULT_CONSISTENCY_WINDOW, Deed, DeedId, Schedule, active_deed_count, archived_deed_count,
    consistency,
};
use werdy_store::DeedName;

use super::context::{use_actions, use_i18n};
use crate::editor_data;
use crate::state::{AppState, EditorError, Screen};

fn deed_name(state: &AppState, id: DeedId, i18n: &super::context::I18n) -> String {
    match state.deed_name(id) {
        Some(DeedName::Key(key)) => i18n.t(key),
        Some(DeedName::Text(name)) => name.clone(),
        None => i18n.t("deeds-name-unavailable"),
    }
}

#[component]
pub fn MyDeeds() -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let actions = use_actions();
    let mut archived_open = use_signal(|| false);
    let app = state.read();
    let active_count = active_deed_count(&app.data.deeds, app.today);
    let archived: Vec<_> = app
        .data
        .deeds
        .iter()
        .filter(|deed| deed.is_archived_on(app.today))
        .cloned()
        .collect();
    let archived_count = archived_deed_count(&app.data.deeds, app.today);
    let mut slots = app.slot_order.clone();
    for deed in &app.data.deeds {
        if let Some(rule) = deed.rule_on(app.today)
            && !slots.contains(&rule.slot)
        {
            slots.push(rule.slot.clone());
        }
    }
    let groups: Vec<_> = slots
        .iter()
        .filter_map(|slot| {
            let members: Vec<_> = app
                .data
                .deeds
                .iter()
                .filter(|deed| {
                    deed.created_on() <= app.today
                        && !deed.is_archived_on(app.today)
                        && deed
                            .rule_on(app.today)
                            .is_some_and(|rule| rule.slot == *slot)
                })
                .cloned()
                .collect();
            (!members.is_empty()).then(|| (slot.clone(), members))
        })
        .collect();
    let count_text = i18n.t_with(
        "deeds-active-count",
        &[("count", FluentValue::from(active_count as i64))],
    );
    let archived_text = i18n.t_with(
        "deeds-archived",
        &[("count", FluentValue::from(archived_count as i64))],
    );

    rsx! {
        section { class: "deeds-screen",
            header { class: "deeds-header",
                div { h1 { class: "day-title", {i18n.t("deeds-title")} } p { class: "deeds-subtitle", "{count_text}" } }
                button { class: "primary-button", onclick: move |_| actions.view(|state| state.show_screen(Screen::Editor(None))), {i18n.t("deeds-add")} }
            }
            button { class: "ready-set-entry", onclick: move |_| actions.view(|state| state.show_screen(Screen::SetPicker)),
                span { {i18n.t("sets-open-from-deeds")} }
                span { class: "deeds-meta", {i18n.t("sets-open-detail")} }
            }
            button { class: "ready-set-entry", onclick: move |_| actions.view(|state| state.show_screen(Screen::FirstRunLocation)),
                span { {i18n.t("location-open-from-deeds")} }
                span { class: "deeds-meta", {i18n.t("location-open-detail")} }
            }
            button { class: "ready-set-entry", onclick: move |_| actions.view(|state| state.open_settings_from_deeds()),
                span { {i18n.t("nav-settings")} }
            }
            if active_count == 0 { p { class: "sheet-empty", {i18n.t("deeds-empty")} } }
            for (slot, members) in groups {
                section { class: "deeds-group", key: "{slot.0}",
                    h2 { class: "slot-title", {i18n.t(&format!("slot-{}", slot.0))} }
                    div { class: "deeds-list",
                        for deed in members { DeedListRow { key: "{deed.id.0}", deed } }
                    }
                }
            }
            if archived_count > 0 {
                button { class: "archived-toggle", aria_expanded: "{archived_open()}", onclick: move |_| archived_open.toggle(), "{archived_text}" }
                if archived_open() {
                    div { class: "deeds-list",
                        for deed in archived { DeedListRow { key: "archived-{deed.id.0}", deed } }
                    }
                }
            }
        }
    }
}

#[component]
fn DeedListRow(deed: Deed) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let actions = use_actions();
    let mut error = use_signal(|| None::<EditorError>);
    let app = state.read();
    let Some(rule) = deed.rule_on(app.today) else {
        return rsx! {};
    };
    let name = deed_name(&app, deed.id, &i18n);
    let category = i18n.t(&format!("category-{}", rule.category.0));
    let schedule = i18n.schedule_sentence(&rule.schedule);
    let recent = consistency(
        &app.data.records(),
        deed.id,
        app.today,
        DEFAULT_CONSISTENCY_WINDOW,
    );
    let recent_line = i18n.consistency_line(recent);
    let target = rule.target.as_ref().map(|target| {
        let unit = i18n.t(&format!("editor-unit-{}", target.unit()));
        i18n.t_with(
            "deeds-target",
            &[
                ("amount", FluentValue::from(target.amount() as i64)),
                ("unit", FluentValue::from(unit)),
            ],
        )
    });
    let archived = deed.is_archived_on(app.today);
    let not_due = !archived && !deed.is_due_on(app.today, &app.data.settings.calendar);
    let id = deed.id;
    rsx! {
        div { class: "deeds-list-row",
            button { class: "deeds-row-main", onclick: move |_| actions.view(|state| state.show_screen(Screen::Editor(Some(id)))),
                span { class: "deed-name", "{name}" }
                span { class: "deeds-meta", "{category} · {schedule}" }
                span { class: "deeds-meta", "{recent_line}" }
                if let Some(target) = target { span { class: "deeds-meta", "{target}" } }
            }
            if not_due { span { class: "not-due-tag", {i18n.t("deeds-not-due")} } }
            if archived {
                button { class: "secondary-button", onclick: move |_| {
                    if let Err(problem) = actions.change_deed(|state| state.restore_deed(id)) { error.set(Some(problem)); }
                }, {i18n.t("deeds-restore")} }
            }
            if let Some(problem) = error() { p { class: "editor-error", role: "alert", {i18n.t(error_key(problem))} } }
        }
    }
}

fn error_key(error: EditorError) -> &'static str {
    match error {
        EditorError::EmptyName => "editor-error-empty-name",
        EditorError::NoWeekday => "editor-error-no-weekday",
        EditorError::InvalidTarget => "editor-error-invalid-target",
        EditorError::MissingDeed => "editor-error-missing-deed",
        EditorError::RuleChange | EditorError::DeleteNotConfirmed => "editor-error-rule-change",
    }
}

#[component]
pub fn DeedEditor(id: Option<DeedId>) -> Element {
    let i18n = use_i18n();
    let state = use_context::<Signal<AppState>>();
    let actions = use_actions();
    let mut draft = use_signal(|| {
        let app = state.read();
        let mut draft = id.and_then(|id| app.draft_for(id)).unwrap_or_default();
        if let Some(id) = id
            && let Some(DeedName::Key(key)) = app.deed_name(id)
        {
            draft.name = i18n.t(key);
        }
        if id.is_some() {
            draft.original_name = Some(draft.name.clone());
        }
        draft
    });
    let mut error = use_signal(|| None::<EditorError>);
    let mut confirm_delete = use_signal(|| false);
    let (Ok(catalog), Ok(ranges)) = (editor_data::catalog(), editor_data::ranges()) else {
        return rsx! { p { class: "editor-error", role: "alert", {i18n.t("editor-data-failed")} } };
    };
    let categories = catalog.categories.clone();
    let slots = catalog.slots.clone();
    let units = catalog.units.clone();
    let first_range = ranges.first().and_then(|choice| choice.range().ok());
    let selected = draft();
    let archived = {
        let app = state.read();
        id.and_then(|id| app.data.deeds.iter().find(|deed| deed.id == id))
            .is_some_and(|deed| deed.is_archived_on(app.today))
    };

    rsx! {
        section { class: "editor-screen",
            header { class: "editor-header",
                button { class: "secondary-button", onclick: move |_| actions.view(|state| state.show_screen(Screen::Deeds)), {i18n.t("editor-cancel")} }
                h1 { class: "day-title", {i18n.t(if id.is_some() { "editor-edit-title" } else { "editor-add-title" })} }
                button { class: "primary-button", onclick: move |_| {
                    match actions.change_deed(|state| state.save_deed(id, &draft())) {
                        Ok(_) => actions.view(|state| state.show_screen(Screen::Deeds)),
                        Err(problem) => error.set(Some(problem)),
                    }
                }, {i18n.t("editor-save")} }
            }
            if let Some(problem) = error() { p { class: "editor-error", role: "alert", {i18n.t(error_key(problem))} } }
            label { class: "editor-label", r#for: "deed-name", {i18n.t("editor-name")} }
            input { id: "deed-name", class: "editor-input", value: "{selected.name}", oninput: move |event| draft.write().name = event.value() }
            fieldset { class: "editor-field",
                legend { {i18n.t("editor-category")} }
                div { class: "chip-list",
                    for key in categories {
                        button { class: "choice-chip", key: "{key}", aria_pressed: "{selected.category.0 == key}", onclick: move |_| draft.write().category = werdy_core::CategoryKey::new(&key), {i18n.t(&format!("category-{key}"))} }
                    }
                }
            }
            fieldset { class: "editor-field",
                legend { {i18n.t("editor-slot")} }
                div { class: "chip-list",
                    for key in slots {
                        button { class: "choice-chip", key: "{key}", aria_pressed: "{selected.slot.0 == key}", onclick: move |_| draft.write().slot = werdy_core::SlotKey::new(&key), {i18n.t(&format!("slot-{key}"))} }
                    }
                }
            }
            fieldset { class: "editor-field editor-card",
                legend { {i18n.t("editor-schedule")} }
                div { class: "chip-list",
                    button { class: "choice-chip", aria_pressed: "{matches!(selected.schedule, Schedule::EveryDay)}", onclick: move |_| draft.write().schedule = Schedule::EveryDay, {i18n.t("editor-every-day")} }
                    button { class: "choice-chip", aria_pressed: "{matches!(selected.schedule, Schedule::Weekdays(_))}", onclick: move |_| draft.write().weekdays(), {i18n.t("editor-weekdays")} }
                    button { class: "choice-chip", aria_pressed: "{matches!(selected.schedule, Schedule::Hijri(_))}", onclick: move |_| {
                        if let Some(range) = first_range { draft.write().schedule = Schedule::Hijri(range); }
                    }, {i18n.t("editor-hijri")} }
                }
                if let Schedule::Weekdays(days) = &selected.schedule {
                    div { class: "weekday-list",
                        for day in crate::i18n::WEEKDAY_ORDER {
                            button { class: "weekday-chip", aria_pressed: "{days.contains(&day)}", onclick: move |_| draft.write().toggle_weekday(day), {i18n.t(crate::i18n::weekday_key(day))} }
                        }
                    }
                }
                if let Schedule::Hijri(chosen) = &selected.schedule {
                    div { class: "range-list",
                        for choice in ranges {
                            button { class: "choice-chip", key: "{choice.key}", aria_pressed: "{choice.range().ok().as_ref() == Some(chosen)}", onclick: move |_| {
                                if let Ok(range) = choice.range() { draft.write().schedule = Schedule::Hijri(range); }
                            }, {i18n.t(&format!("range-{}", choice.key.replace('_', "-")))} }
                        }
                    }
                }
                p { class: "schedule-sentence", {i18n.schedule_sentence(&selected.schedule)} }
            }
            div { class: "editor-card target-card",
                label { class: "target-toggle",
                    input { r#type: "checkbox", checked: selected.target_enabled, onchange: move |event| draft.write().target_enabled = event.checked() }
                    span { {i18n.t("editor-target")} }
                }
                p { class: "deeds-meta", {i18n.t("editor-target-help")} }
                if selected.target_enabled {
                    div { class: "target-controls",
                        button { class: "step-button", aria_label: i18n.t("editor-target-minus"), onclick: move |_| { let amount = werdy_core::decrease_target_amount(draft().target_amount); draft.write().target_amount = amount; }, "−" }
                        output { {i18n.num(selected.target_amount)} }
                        button { class: "step-button", aria_label: i18n.t("editor-target-plus"), onclick: move |_| { let amount = werdy_core::increase_target_amount(draft().target_amount); draft.write().target_amount = amount; }, "+" }
                        label { class: "editor-label", r#for: "target-unit", {i18n.t("editor-unit")} }
                        select { id: "target-unit", class: "editor-input", value: "{selected.target_unit}", onchange: move |event| draft.write().target_unit = event.value(),
                            for unit in units { option { value: "{unit.key}", {i18n.t(&format!("editor-unit-{}", unit.key))} } }
                        }
                    }
                }
            }
            p { class: "deeds-meta", {i18n.t("editor-from-today")} }
            if let Some(id) = id {
                if !archived { button { class: "secondary-button", onclick: move |_| { if actions.change_deed(|state| state.archive_deed(id)).is_ok() { actions.view(|state| state.show_screen(Screen::Deeds)); } else { error.set(Some(EditorError::RuleChange)); } }, {i18n.t("editor-archive")} } }
                button { class: "danger-button", onclick: move |_| confirm_delete.set(true), {i18n.t("editor-delete")} }
                if confirm_delete() {
                    div { class: "confirm-box", role: "alertdialog", aria_label: i18n.t("editor-confirm-delete"),
                        p { {i18n.t("editor-confirm-delete")} }
                        button { class: "danger-button", onclick: move |_| { if actions.change_deed(|state| state.delete_deed_confirmed(id, true)).is_ok() { actions.view(|state| state.show_screen(Screen::Deeds)); } else { error.set(Some(EditorError::RuleChange)); } }, {i18n.t("editor-confirm")} }
                        button { class: "secondary-button", onclick: move |_| confirm_delete.set(false), {i18n.t("editor-cancel")} }
                    }
                }
            }
        }
    }
}
