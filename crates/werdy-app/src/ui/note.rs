//! Day note editor for the day on screen.

use dioxus::prelude::*;
use werdy_core::{Error, ExcuseCredit, WerdyDay};

use super::context::{use_actions, use_i18n};
use super::header::date_text;
use super::icons::CrossIcon;
use crate::{
    platform,
    state::{AppState, NoteError},
};

#[component]
pub fn NoteDialog(day: WerdyDay, on_close: EventHandler<()>) -> Element {
    let i18n = use_i18n();
    let actions = use_actions();
    let state = use_context::<Signal<AppState>>();
    let existing = state.read().data.day_notes.get(&day).cloned();
    use_effect(platform::open_note_dialog);
    let has_note = existing.is_some();
    let mut text = use_signal(|| {
        existing
            .as_ref()
            .map_or_else(String::new, |note| note.text().to_owned())
    });
    let mut credit = use_signal(|| {
        existing
            .as_ref()
            .map_or(ExcuseCredit::None, |note| note.credit())
    });
    let mut error = use_signal(|| None::<String>);
    let explanation = match credit() {
        ExcuseCredit::None => "note-credit-none-explain",
        ExcuseCredit::Half => "note-credit-half-explain",
        ExcuseCredit::Full => "note-credit-full-explain",
    };
    let empty_error = i18n.t("note-empty-error");
    let length_error = i18n.t("note-length-error");
    let date_error = i18n.t("note-date-error");
    let save_error = i18n.t("note-save-error");
    let delete_date_error = date_error.clone();
    let delete_save_error = save_error.clone();

    rsx! {
        dialog { id: "note-dialog", class: "note-dialog", aria_labelledby: "note-title",
                onkeydown: move |event| {
                    if event.key() == Key::Escape {
                        event.prevent_default();
                        on_close.call(());
                    }
                },
                div { class: "note-dialog-head",
                    div {
                        h2 { id: "note-title", {i18n.t("note-title")} }
                        p { class: "note-date", {date_text(&i18n, day)} }
                    }
                    button { class: "icon-button", aria_label: i18n.t("note-close"), onclick: move |_| on_close.call(()), CrossIcon {} }
                }
                label { class: "editor-label", r#for: "note-text", {i18n.t("note-text-label")} }
                textarea { id: "note-text", class: "note-text", rows: "4", value: "{text}", oninput: move |event| text.set(event.value()) }
                fieldset { class: "note-credit",
                    legend { class: "editor-label", {i18n.t("note-credit-label")} }
                    div { class: "note-choices",
                        CreditChoice { choice: ExcuseCredit::None, selected: credit(), on_pick: move |choice| credit.set(choice) }
                        CreditChoice { choice: ExcuseCredit::Half, selected: credit(), on_pick: move |choice| credit.set(choice) }
                        CreditChoice { choice: ExcuseCredit::Full, selected: credit(), on_pick: move |choice| credit.set(choice) }
                    }
                    p { class: "note-explanation", {i18n.t(explanation)} }
                }
                if let Some(message) = error() { p { class: "editor-error", role: "alert", "{message}" } }
                div { class: "note-actions",
                    button { class: "primary-button", onclick: move |_| {
                        match actions.save_note(day, &text(), credit()) {
                            Ok(()) => on_close.call(()),
                            Err(NoteError::TooLong) => error.set(Some(length_error.clone())),
                            Err(NoteError::Invalid(Error::EmptyDayNote)) => error.set(Some(empty_error.clone())),
                            Err(NoteError::Invalid(Error::InvalidDate)) => error.set(Some(date_error.clone())),
                            Err(_) => error.set(Some(save_error.clone())),
                        }
                    }, {i18n.t("note-save")} }
                    button { class: "secondary-button", onclick: move |_| on_close.call(()), {i18n.t("note-cancel")} }
                    if has_note {
                        button { class: "danger-button", onclick: move |_| {
                            match actions.delete_note(day) {
                                Ok(()) => on_close.call(()),
                                Err(NoteError::Invalid(Error::InvalidDate)) => error.set(Some(delete_date_error.clone())),
                                Err(_) => error.set(Some(delete_save_error.clone())),
                            }
                        }, {i18n.t("note-delete")} }
                    }
                }
        }
    }
}

#[component]
fn CreditChoice(
    choice: ExcuseCredit,
    selected: ExcuseCredit,
    on_pick: EventHandler<ExcuseCredit>,
) -> Element {
    let i18n = use_i18n();
    let key = match choice {
        ExcuseCredit::None => "note-credit-none",
        ExcuseCredit::Half => "note-credit-half",
        ExcuseCredit::Full => "note-credit-full",
    };
    rsx! {
        label { class: "note-choice",
            input { r#type: "radio", name: "note-credit", checked: selected == choice,
                onchange: move |_| on_pick.call(choice) }
            span { {i18n.t(key)} }
        }
    }
}
