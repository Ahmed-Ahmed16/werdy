//! Screen components. They show what the app state returns and send taps
//! back to it; they hold no scoring.

mod context;
mod deeds;
mod first_run;
mod header;
mod icons;
mod marks;
mod note;
mod rail;
mod reports;
mod settings;
mod sheet;
mod summary;

use std::rc::Rc;

use dioxus::prelude::*;
use werdy_store::Theme;

use crate::i18n::{Language, Texts};
use crate::state::{AppState, Screen};
use crate::{platform, starter};

const TOKENS_CSS: Asset = asset!("/assets/tokens.css");
const APP_CSS: Asset = asset!("/assets/app.css");
const FONT_CSS: &str = "https://fonts.googleapis.com/css2?family=IBM+Plex+Sans+Arabic:wght@400;500;600;700&display=swap";

/// Root component: loads texts and data, then shows the shell.
#[component]
pub fn App() -> Element {
    let texts = use_hook(|| Rc::new(Texts::load()));
    let store = use_hook(platform::open_store);
    let initial = use_hook(|| {
        platform::today().map(|today| {
            let mut state = AppState::start(store.as_ref(), today, starter::slot_order());
            if let Ok(day) = platform::today_with_settings(&state.data.settings) {
                state.update_today(day);
            }
            state.show_welcome_on_open(store.as_ref());
            state
        })
    });
    use_context_provider(|| texts.clone());

    rsx! {
        document::Title { "werdy" }
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "" }
        document::Stylesheet { href: FONT_CSS }
        document::Stylesheet { href: TOKENS_CSS }
        document::Stylesheet { href: APP_CSS }
        match initial {
            Ok(initial) => rsx! { Shell { initial, store: StoreHandle(store) } },
            Err(_) => rsx! {
                p { class: "fatal", dir: "rtl", lang: "ar",
                    {texts.get(Language::Arabic, "app-date-failed", &[])}
                }
            },
        }
    }
}

/// Store wrapper so it can be a component prop. Two handles are equal when
/// they point at the same store.
#[derive(Clone)]
pub struct StoreHandle(Rc<dyn werdy_store::Store>);

impl PartialEq for StoreHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Page frame: side rail, header, status, sheet and today card.
#[component]
fn Shell(initial: AppState, store: StoreHandle) -> Element {
    let mut state = use_signal(|| initial);
    let mut note_day = use_signal(|| None::<werdy_core::WerdyDay>);
    let store = use_signal(|| store.0);
    let _watch = use_hook(move || {
        Rc::new(platform::watch_changes(
            move || {
                let settings = state.read().data.settings.clone();
                if let Ok(today) = platform::today_with_settings(&settings) {
                    let changed = state.read().today != today;
                    if changed {
                        state.write().update_today(today);
                    }
                }
            },
            move || {
                let saved = store.read().clone();
                state.write().reload_from(saved.as_ref());
            },
        ))
    });
    use_context_provider(|| context::Actions::new(state, store));
    use_context_provider(|| state);

    let settings = &state.read().data.settings;
    let language = Language::from_code(&settings.language);
    let theme = match settings.theme {
        Theme::System => "system",
        Theme::Light => "light",
        Theme::Dark => "dark",
    };

    rsx! {
        div {
            class: "werdy-root",
            dir: language.dir(),
            lang: language.code(),
            "data-theme": theme,
            if state.read().data.first_run_complete { rail::Rail {} }
            main { class: match state.read().screen {
                Screen::FirstRunLocation | Screen::SetPicker => "onboarding-content",
                Screen::Sheet => "panes",
                Screen::Reports => "report-panes",
                Screen::Deeds | Screen::Editor(_) | Screen::Settings => "page-content",
            },
                summary::StatusBanner {}
                match state.read().screen {
                    Screen::FirstRunLocation => rsx! { first_run::LocationStep {} },
                    Screen::SetPicker => rsx! { first_run::SetPicker {} },
                    Screen::Sheet => rsx! {
                        header::DayHeader { on_open_note: move |day| note_day.set(Some(day)) }
                        summary::TodayCard {}
                        sheet::Sheet {}
                    },
                    Screen::Reports => rsx! { reports::ReportsScreen {} },
                    Screen::Deeds => rsx! { deeds::MyDeeds {} },
                    Screen::Settings => rsx! { settings::SettingsScreen {} },
                    Screen::Editor(id) => rsx! { deeds::DeedEditor { key: "{id:?}", id } },
                }
            }
            if state.read().data.first_run_complete { rail::BottomNav {} }
            if let Some(day) = note_day() {
                note::NoteDialog {
                    key: "{day:?}",
                    day,
                    on_close: move |_| {
                        platform::close_note_dialog();
                        platform::focus_element("note-open");
                        note_day.set(None);
                    }
                }
            }
        }
    }
}
