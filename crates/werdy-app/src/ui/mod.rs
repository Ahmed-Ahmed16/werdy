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
use crate::state::{AppState, SaveStatus, Screen};
use crate::{demo, platform, starter};

const TOKENS_CSS: Asset = asset!("/assets/tokens.css");
const APP_CSS: Asset = asset!("/assets/app.css");
const FONT_CSS: &str = "https://fonts.googleapis.com/css2?family=IBM+Plex+Sans+Arabic:wght@400;500;600;700&display=swap";

fn demo_today() -> Result<werdy_core::WerdyDay, String> {
    let settings = demo::demo_settings().map_err(|error| error.to_string())?;
    platform::today_with_settings(&settings).map_err(|error| error.to_string())
}

fn start_app_state(
    mode: platform::DataMode,
    store: &dyn werdy_store::Store,
) -> Result<AppState, String> {
    let today = if mode == platform::DataMode::Demo {
        demo_today()?
    } else {
        platform::today().map_err(|error| error.to_string())?
    };
    let seed_failed = mode == platform::DataMode::Demo
        && !matches!(store.load(), Ok(Some(_)))
        && demo::demo_records(today)
            .and_then(|mut demo_data| {
                demo_data.demo_generation = Some(platform::demo_generation());
                store.save(&demo_data).map_err(Into::into)
            })
            .is_err();
    let mut state = AppState::start(store, today, starter::slot_order());
    if let Ok(day) = platform::today_with_settings(&state.data.settings) {
        state.update_today(day);
    }
    if seed_failed {
        state.save_status = SaveStatus::SaveFailed;
    }
    state.show_welcome_on_open(store);
    Ok(state)
}

fn reset_demo(store: &dyn werdy_store::Store) -> Result<AppState, String> {
    let today = demo_today()?;
    let mut records = demo::demo_records(today).map_err(|error| error.to_string())?;
    records.demo_generation = Some(platform::demo_generation());
    store.save(&records).map_err(|error| error.to_string())?;
    Ok(AppState::start(store, today, starter::slot_order()))
}

#[cfg(test)]
mod tests {
    use werdy_core::Mark;
    use werdy_store::{MemoryStore, Store};

    use super::{reset_demo, start_app_state};
    use crate::platform::DataMode;
    use crate::state::Tap;

    #[test]
    fn reset_demo_replaces_stale_tabs_without_restoring_marks_or_deleted_deeds()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = MemoryStore::default();
        let mut first = start_app_state(DataMode::Demo, &store)?;
        let mut second = start_app_state(DataMode::Demo, &store)?;
        assert!(second.data == first.data);
        let marked = first.data.deeds[0].id;
        let deleted = first.data.deeds[1].id;
        first.tap(marked, Tap::Done);
        first.delete_deed_confirmed(deleted, true)?;
        first.save_to(&store);
        assert_eq!(first.mark(marked), Some(Mark::Done));

        second = reset_demo(&store)?;
        let fresh = store.load()?.ok_or("missing demo records")?;
        first.reload_from(&store);
        second.reload_from(&store);

        assert!(first.data == fresh, "stale tab kept old demo data");
        assert!(second.data == fresh, "reset tab changed fresh demo data");
        assert!(
            store.load()?.as_ref() == Some(&fresh),
            "store changed after reload"
        );
        assert_eq!(first.mark(marked), None);
        assert!(fresh.deeds.iter().any(|deed| deed.id == deleted));
        Ok(())
    }

    #[test]
    fn stale_demo_save_after_reset_keeps_fresh_seed() -> Result<(), Box<dyn std::error::Error>> {
        let store = MemoryStore::default();
        let mut stale = start_app_state(DataMode::Demo, &store)?;
        let deed_id = stale.data.deeds[0].id;
        stale.tap(deed_id, Tap::Done);
        stale.save_to(&store);

        let fresh = reset_demo(&store)?;
        stale.save_to(&store);

        assert!(stale.data == fresh.data, "stale save kept old demo data");
        assert!(
            store.load()?.as_ref() == Some(&fresh.data),
            "stale save changed demo store"
        );
        assert_eq!(stale.mark(deed_id), None);
        Ok(())
    }
}

/// Root component: loads texts and data, then shows the shell.
#[component]
pub fn App() -> Element {
    let texts = use_hook(|| Rc::new(Texts::load()));
    let mode = use_hook(platform::data_mode);
    let store = use_hook(|| platform::open_store(mode));
    let initial = use_hook(|| start_app_state(mode, store.as_ref()));
    use_context_provider(|| texts.clone());

    rsx! {
        document::Title { "werdy" }
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "" }
        document::Stylesheet { href: FONT_CSS }
        document::Stylesheet { href: TOKENS_CSS }
        document::Stylesheet { href: APP_CSS }
        match initial {
            Ok(initial) => rsx! { Shell { initial, store: StoreHandle(store), mode } },
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
fn Shell(initial: AppState, store: StoreHandle, mode: platform::DataMode) -> Element {
    let mut state = use_signal(|| initial);
    let mut wide_layout = use_signal(platform::wide_layout);
    let mut last_screen = use_signal(|| state.read().screen);
    use_effect(move || {
        let screen = state.read().screen;
        if last_screen() != screen {
            last_screen.set(screen);
            platform::scroll_to_top();
        }
    });
    let mut note_day = use_signal(|| None::<werdy_core::WerdyDay>);
    let store = use_signal(|| store.0);
    let _watch = use_hook(move || {
        Rc::new(platform::watch_changes(
            mode,
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
            move || {
                let wide = platform::wide_layout();
                if wide_layout() != wide {
                    wide_layout.set(wide);
                }
            },
        ))
    });
    use_context_provider(|| context::Actions::new(state, store));
    use_context_provider(|| state);

    let settings = &state.read().data.settings;
    let language = Language::from_code(&settings.language);
    let texts = use_context::<Rc<Texts>>();
    let demo_banner = texts.get(language, "demo-banner", &[]);
    let demo_reset = texts.get(language, "demo-reset", &[]);
    let demo_leave = texts.get(language, "demo-leave", &[]);
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
                if mode == platform::DataMode::Demo {
                    div { class: "demo-banner", role: "note",
                        span { {demo_banner} }
                        button {
                            class: "demo-action",
                            onclick: move |_| {
                                let saved = store.read().clone();
                                match reset_demo(saved.as_ref()) {
                                    Ok(fresh) => state.set(fresh),
                                    Err(_) => state.write().save_status = SaveStatus::SaveFailed,
                                }
                            },
                            {demo_reset}
                        }
                        a { class: "demo-action", href: platform::app_address(), {demo_leave} }
                    }
                }
                summary::StatusBanner {}
                match state.read().screen {
                    Screen::FirstRunLocation => rsx! { first_run::LocationStep {} },
                    Screen::SetPicker => rsx! { first_run::SetPicker {} },
                    Screen::Sheet => rsx! {
                        header::DayHeader { on_open_note: move |day| note_day.set(Some(day)) }
                        div { class: "sheet-aside",
                            summary::TodayCard {}
                            if wide_layout() { summary::WeekCard {} }
                        }
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
