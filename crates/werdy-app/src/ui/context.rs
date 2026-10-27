//! Shared handles for components: text lookup and state changes.

use std::rc::Rc;

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_store::Store;

use crate::i18n::{Language, Texts};
use crate::state::AppState;
use crate::state::EditorError;
use werdy_core::{Error, ExcuseCredit, Schedule, WerdyDay};

/// Text lookup in the current language.
#[derive(Clone)]
pub struct I18n {
    texts: Rc<Texts>,
    /// Current interface language.
    pub language: Language,
}

impl I18n {
    /// Text for a key without variables.
    pub fn t(&self, key: &str) -> String {
        self.texts.get(self.language, key, &[])
    }

    /// Text for a key with variables.
    pub fn t_with(&self, key: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        self.texts.get(self.language, key, args)
    }

    pub fn schedule_sentence(&self, schedule: &Schedule) -> String {
        crate::i18n::schedule_sentence(&self.texts, self.language, schedule)
    }
}

/// Reads texts and the language from context. The component renders again when
/// the language changes.
pub fn use_i18n() -> I18n {
    let texts = use_context::<Rc<Texts>>();
    let state = use_context::<Signal<AppState>>();
    let language = Language::from_code(&state.read().data.settings.language);
    I18n { texts, language }
}

/// Changes to the app state. Data changes are saved right away.
#[derive(Clone, Copy)]
pub struct Actions {
    state: Signal<AppState>,
    store: Signal<Rc<dyn Store>>,
}

impl Actions {
    /// Creates the handle from the state and store signals.
    pub fn new(state: Signal<AppState>, store: Signal<Rc<dyn Store>>) -> Self {
        Self { state, store }
    }

    /// Changes the owner's data, then saves it.
    pub fn change(self, change: impl FnOnce(&mut AppState)) {
        let store = self.store.read().clone();
        let mut state = self.state;
        let mut state = state.write();
        if state.change_on_day(crate::platform::today(), change) {
            state.save_to(store.as_ref());
        }
    }

    pub fn change_deed<T>(
        self,
        change: impl FnOnce(&mut AppState) -> Result<T, EditorError>,
    ) -> Result<T, EditorError> {
        let store = self.store.read().clone();
        let mut state = self.state;
        let mut state = state.write();
        let today = crate::platform::today().map_err(|_| {
            state.save_status = crate::state::SaveStatus::DateFailed;
            EditorError::RuleChange
        })?;
        state.update_today(today);
        let changed = change(&mut state)?;
        state.save_to(store.as_ref());
        Ok(changed)
    }

    fn change_note(
        self,
        day: WerdyDay,
        change: impl FnOnce(&mut AppState) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let store = self.store.read().clone();
        let mut state = self.state;
        let mut state = state.write();
        let today = crate::platform::today().inspect_err(|_| {
            state.save_status = crate::state::SaveStatus::DateFailed;
        })?;
        state.update_today(today);
        if day > today {
            return Err(Error::InvalidDate);
        }
        change(&mut state)?;
        state.save_to(store.as_ref());
        Ok(())
    }

    /// Validates and saves a note for the chosen day.
    pub fn save_note(self, day: WerdyDay, text: &str, credit: ExcuseCredit) -> Result<(), Error> {
        self.change_note(day, |state| state.save_day_note(day, text, credit))
    }

    /// Deletes a note for the chosen day.
    pub fn delete_note(self, day: WerdyDay) -> Result<(), Error> {
        self.change_note(day, |state| {
            state.delete_day_note(day);
            Ok(())
        })
    }

    /// Changes only what is on screen, such as the day shown. Nothing is saved.
    pub fn view(self, change: impl FnOnce(&mut AppState)) {
        let mut state = self.state;
        change(&mut state.write());
    }
}

/// Reads the actions handle from context.
pub fn use_actions() -> Actions {
    use_context::<Actions>()
}
