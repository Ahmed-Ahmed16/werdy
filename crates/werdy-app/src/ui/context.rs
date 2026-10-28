//! Shared handles for components: text lookup and state changes.

use std::rc::Rc;

use dioxus::prelude::*;
use fluent_bundle::FluentValue;
use werdy_store::{Digits, Store};

use crate::i18n::{Language, Texts};
use crate::state::EditorError;
use crate::state::{AppState, NoteError};
use werdy_core::{Consistency, Error, ExcuseCredit, Schedule, WerdyDay};

/// Text lookup in the current language.
#[derive(Clone)]
pub struct I18n {
    texts: Rc<Texts>,
    /// Current interface language.
    pub language: Language,
    digits: Digits,
}

impl I18n {
    /// Text for a key without variables.
    pub fn t(&self, key: &str) -> String {
        crate::i18n::format_digits(&self.texts.get(self.language, key, &[]), self.digits)
    }

    /// Text for a key with variables.
    pub fn t_with(&self, key: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        self.texts
            .get_with_digits(self.language, self.digits, key, args)
    }

    pub fn num(&self, value: impl std::fmt::Display) -> String {
        crate::i18n::format_digits(&value.to_string(), self.digits)
    }

    pub fn schedule_sentence(&self, schedule: &Schedule) -> String {
        crate::i18n::format_digits(
            &crate::i18n::schedule_sentence(&self.texts, self.language, schedule),
            self.digits,
        )
    }

    pub fn consistency_line(&self, value: Consistency) -> String {
        if value.of == 0 {
            return self.t("consistency-no-due");
        }
        self.t_with(
            "consistency-line",
            &[
                ("done", FluentValue::from(value.done)),
                ("due", FluentValue::from(value.of)),
            ],
        )
    }
}

/// Reads texts and the language from context. The component renders again when
/// the language changes.
pub fn use_i18n() -> I18n {
    let texts = use_context::<Rc<Texts>>();
    let state = use_context::<Signal<AppState>>();
    let settings = &state.read().data.settings;
    let language = Language::from_code(&settings.language);
    I18n {
        texts,
        language,
        digits: settings.digits,
    }
}

/// Changes to the app state. Data changes are saved right away.
#[derive(Clone, Copy)]
pub struct Actions {
    read_only: bool,
    state: Signal<AppState>,
    store: Signal<Rc<dyn Store>>,
}

impl Actions {
    /// Creates the handle from the state and store signals.
    pub fn new(state: Signal<AppState>, store: Signal<Rc<dyn Store>>) -> Self {
        Self {
            state,
            store,
            read_only: false,
        }
    }

    pub fn read_only(state: Signal<AppState>, store: Signal<Rc<dyn Store>>) -> Self {
        Self {
            state,
            store,
            read_only: true,
        }
    }

    /// Changes the owner's data, then saves it.
    pub fn change(self, change: impl FnOnce(&mut AppState)) {
        if self.read_only {
            return;
        }
        let store = self.store.read().clone();
        let mut state = self.state;
        let mut state = state.write();
        let today = crate::platform::today_with_settings(&state.data.settings);
        if state.change_on_day(today, change) {
            if let Ok(today) = crate::platform::today_with_settings(&state.data.settings) {
                state.update_today(today);
            }
            state.save_to(store.as_ref());
        }
    }

    pub fn change_deed<T>(
        self,
        change: impl FnOnce(&mut AppState) -> Result<T, EditorError>,
    ) -> Result<T, EditorError> {
        if self.read_only {
            return Err(EditorError::RuleChange);
        }
        let store = self.store.read().clone();
        let mut state = self.state;
        let mut state = state.write();
        let today = crate::platform::today_with_settings(&state.data.settings).map_err(|_| {
            state.save_status = crate::state::SaveStatus::DateFailed;
            EditorError::RuleChange
        })?;
        state.update_today(today);
        let changed = change(&mut state)?;
        if let Ok(today) = crate::platform::today_with_settings(&state.data.settings) {
            state.update_today(today);
        }
        state.save_to(store.as_ref());
        Ok(changed)
    }

    fn change_note(
        self,
        day: WerdyDay,
        change: impl FnOnce(&mut AppState) -> Result<(), NoteError>,
    ) -> Result<(), NoteError> {
        if self.read_only {
            return Err(Error::InvalidDate.into());
        }
        let store = self.store.read().clone();
        let mut state = self.state;
        let mut state = state.write();
        let today = crate::platform::today().inspect_err(|_| {
            state.save_status = crate::state::SaveStatus::DateFailed;
        })?;
        state.update_today(today);
        if day > today {
            return Err(Error::InvalidDate.into());
        }
        change(&mut state)?;
        state.save_to(store.as_ref());
        Ok(())
    }

    /// Validates and saves a note for the chosen day.
    pub fn save_note(
        self,
        day: WerdyDay,
        text: &str,
        credit: ExcuseCredit,
    ) -> Result<(), NoteError> {
        self.change_note(day, |state| state.save_day_note(day, text, credit))
    }

    /// Deletes a note for the chosen day.
    pub fn delete_note(self, day: WerdyDay) -> Result<(), NoteError> {
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

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use fluent_bundle::FluentValue;
    use werdy_store::Digits;

    use super::{I18n, Language, Texts};
    use werdy_core::Consistency;

    #[test]
    #[allow(clippy::unwrap_used, clippy::panic)]
    fn watched_actions_refuse_all_mutations_and_never_save() {
        use dioxus::prelude::*;
        use werdy_store::{MemoryStore, Store};
        fn root() -> Element {
            let memory = use_hook(|| Rc::new(MemoryStore::default()));
            let state = use_signal(|| {
                crate::state::AppState::start(
                    memory.as_ref(),
                    werdy_core::WerdyDay::new(2026, 10, 28).unwrap(),
                    vec![],
                )
            });
            let store = use_signal(|| {
                let store: Rc<dyn Store> = memory.clone();
                store
            });
            let actions = super::Actions::read_only(state, store);
            use_hook(move || {
                let before = state.read().data.clone();
                actions.change(|_| panic!("read only change ran"));
                assert!(
                    actions
                        .change_deed::<()>(|_| panic!("read only deed change ran"))
                        .is_err()
                );
                assert!(
                    actions
                        .save_note(
                            state.read().today,
                            "forbidden",
                            werdy_core::ExcuseCredit::None
                        )
                        .is_err()
                );
                assert!(actions.delete_note(state.read().today).is_err());
                assert!(state.read().data == before);
                assert!(memory.load().unwrap().is_none());
            });
            rsx! {}
        }
        let mut dom = VirtualDom::new(root);
        dom.rebuild_in_place();
    }

    #[test]
    fn fluent_arguments_keep_name_digits_and_format_number_digits() {
        for (digits, count) in [(Digits::Western, "2"), (Digits::ArabicIndic, "٢")] {
            let i18n = I18n {
                texts: Rc::new(Texts::load()),
                language: Language::English,
                digits,
            };
            let name = i18n.t_with(
                "reports-lead-missed",
                &[("deed", FluentValue::from("Read 2 pages"))],
            );
            let due = i18n.t_with("reports-cell-due", &[("due", FluentValue::from(2))]);
            assert!(name.contains("Read 2 pages"));
            assert!(due.contains(count));
        }
    }

    #[test]
    fn notice_and_consistency_counts_use_language_plural_forms() {
        let texts = Rc::new(Texts::load());
        let cases = [
            (1, "يوم واحد", "يوم مستحق", "1 due day"),
            (2, "يومين", "يومين مستحقين", "2 due days"),
            (7, "7 أيام", "7 أيام مستحقة", "7 due days"),
            (11, "11 يومًا", "11 يومًا مستحقًا", "11 due days"),
            (30, "30 يومًا", "30 يومًا مستحقًا", "30 due days"),
            (100, "100 يوم", "100 يوم مستحق", "100 due days"),
        ];
        for language in [Language::Arabic, Language::English] {
            for digits in [Digits::Western, Digits::ArabicIndic] {
                let i18n = I18n {
                    texts: texts.clone(),
                    language,
                    digits,
                };
                for (days, arabic_milestone, arabic_due, english_due) in cases {
                    let milestone =
                        i18n.t_with("sheet-milestone", &[("days", FluentValue::from(days))]);
                    let consistency = i18n.consistency_line(Consistency {
                        done: 0,
                        of: days as usize,
                    });
                    let milestone = milestone.replace(['\u{2068}', '\u{2069}'], "");
                    let consistency = consistency.replace(['\u{2068}', '\u{2069}'], "");
                    match language {
                        Language::Arabic => {
                            let expected_milestone =
                                crate::i18n::format_digits(arabic_milestone, digits);
                            let expected_due = crate::i18n::format_digits(arabic_due, digits);
                            assert!(milestone.contains(&expected_milestone), "{milestone}");
                            assert!(consistency.contains(&expected_due), "{consistency}");
                        }
                        Language::English => {
                            let expected = crate::i18n::format_digits(english_due, digits);
                            assert!(consistency.contains(&expected), "{consistency}");
                            let count = crate::i18n::format_digits(&days.to_string(), digits);
                            assert!(milestone.contains(&count), "{milestone}");
                        }
                    }
                }
            }
        }
    }
}
