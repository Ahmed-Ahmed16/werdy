//! Screen components. They show what the app state returns and send taps
//! back to it; they hold no scoring.

mod account;
mod context;
mod deeds;
mod first_run;
mod header;
mod icons;
#[cfg(any(test, target_arch = "wasm32"))]
mod install;
mod marks;
mod note;
mod rail;
#[cfg(any(target_os = "android", target_arch = "wasm32"))]
mod reminders;
mod reports;
mod settings;
mod sharing;
mod sheet;
mod summary;

use std::rc::Rc;

use dioxus::prelude::*;
use werdy_store::{Store, Theme};

use crate::account::AccountStore;
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

fn start_app_state(mode: platform::DataMode, store: &AccountStore) -> Result<AppState, String> {
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
    // A share link opens Settings, where the code waits in the field (#90).
    if mode == platform::DataMode::Normal
        && state.screen == Screen::Sheet
        && sharing::shared_code().is_some()
    {
        state.show_screen(Screen::Settings);
    }
    if let Ok(day) = platform::today_with_settings(&state.data.settings) {
        state.update_today(day);
    }
    if seed_failed {
        state.save_status = SaveStatus::SaveFailed;
    }
    if mode == platform::DataMode::Normal && store.account.borrow().session.is_none() {
        state.move_level_deeds_to_groups(store);
    }
    state.show_welcome_on_open(store);
    if mode == platform::DataMode::Normal {
        state.show_whats_new_on_open(store);
    }
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
    use std::rc::Rc;
    use werdy_core::{CategoryKey, Deed, DeedId, DeedRule, Mark, Schedule, SlotKey};
    use werdy_store::{MemoryStore, Store};

    use super::{reset_demo, start_app_state};
    use crate::account::{AccountStore, storage::MemoryVault};
    use crate::platform::DataMode;
    use crate::state::Tap;

    fn demo_store() -> AccountStore {
        AccountStore::open(
            Rc::new(MemoryStore::default()),
            Rc::new(MemoryVault::default()),
            false,
        )
    }

    #[test]
    fn opening_moves_legacy_level_deeds_only_for_local_owner_data()
    -> Result<(), Box<dyn std::error::Error>> {
        let start = crate::platform::today()?.previous()?;
        let records = werdy_store::OwnerData {
            deeds: vec![Deed::new(
                DeedId(1),
                start,
                DeedRule {
                    schedule: Schedule::EveryDay,
                    target: None,
                    category: CategoryKey::new("family"),
                    slot: SlotKey::new("any_time"),
                },
            )],
            last_deed_id: 1,
            names: [(
                DeedId(1),
                werdy_store::DeedName::Key("deed-lv-parents".into()),
            )]
            .into(),
            starter_keys: [(DeedId(1), "lv_parents".into())].into(),
            ..Default::default()
        };
        for (mode, signed_in, slot) in [
            (DataMode::Normal, true, "any_time"),
            (DataMode::Normal, false, "heart_and_character"),
            (DataMode::Demo, false, "any_time"),
        ] {
            let inner = Rc::new(MemoryStore::default());
            inner.save(&records)?;
            let store = AccountStore::open(
                inner,
                Rc::new(MemoryVault::default()),
                mode == DataMode::Normal,
            );
            if signed_in {
                store.sign_in(
                    werdy_sync::Session {
                        access_token: "access".into(),
                        refresh_token: "refresh".into(),
                        expires_at: 5000,
                        user_id: "00000000-0000-4000-8000-000000000023".into(),
                        email: None,
                    },
                    &records,
                )?;
            }
            let state = start_app_state(mode, &store)?;
            let saved = store.load()?.ok_or("missing saved records")?;
            assert_eq!(state.data.deeds, saved.deeds);
            let rule = saved.deeds[0].rule_on(state.today).ok_or("missing rule")?;
            assert_eq!(rule.slot.0, slot);
        }
        Ok(())
    }

    #[test]
    fn demo_never_shows_or_records_whats_new() -> Result<(), Box<dyn std::error::Error>> {
        let store = demo_store();
        let state = start_app_state(DataMode::Demo, &store)?;
        assert!(state.whats_new.is_empty());
        assert_eq!(
            store.load()?.ok_or("missing records")?.last_seen_version,
            None
        );
        Ok(())
    }
    #[test]
    fn reset_demo_replaces_stale_tabs_without_restoring_marks_or_deleted_deeds()
    -> Result<(), Box<dyn std::error::Error>> {
        let store = demo_store();
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
        let store = demo_store();
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
    let account_store = use_hook(|| {
        let raw = platform::open_store(mode);
        let vault = crate::account::storage::open()
            .unwrap_or_else(|_| Rc::new(crate::account::storage::UnavailableVault));
        Rc::new(AccountStore::open(
            raw,
            vault,
            mode == platform::DataMode::Normal,
        ))
    });
    use_context_provider(|| crate::account::runtime::AccountHandle(account_store.clone()));
    let account_enabled = account_store.enabled;
    let initial = use_hook(|| {
        start_app_state(mode, account_store.as_ref()).map(|mut state| {
            if !account_enabled && state.screen == Screen::FirstRunWatcher {
                state.continue_first_run_watcher();
            }
            state
        })
    });
    let store: Rc<dyn Store> = account_store;
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

/// The install banner exists only on the web.
fn install_banner() -> Element {
    #[cfg(target_arch = "wasm32")]
    {
        rsx! { install::InstallBanner {} }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rsx! {}
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
    crate::account::runtime::use_background(state);
    use_hook(move || {
        let app = state.read();
        if app.save_status != SaveStatus::LoadFailed {
            platform::reschedule_reminders(&app.data);
        }
    });
    let mut wide_layout = use_signal(platform::wide_layout);
    let mut clock_slot = use_signal(platform::local_slot);
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
                let slot = platform::local_slot();
                if *clock_slot.peek() != slot {
                    clock_slot.set(slot);
                }
                let settings = state.read().data.settings.clone();
                if let Ok(today) = platform::today_with_settings(&settings) {
                    let changed = state.read().today != today;
                    if changed {
                        state.write().update_today(today);
                        platform::reschedule_reminders(&state.read().data);
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
    use_context_provider(|| clock_slot);
    sheet::provide_fold_choices(state.peek().today);
    let sharing = sharing::use_sharing();

    let settings = &state.read().data.settings;
    let language = Language::from_code(&settings.language);
    let texts = use_context::<Rc<Texts>>();
    if sharing.read().selected.is_some() {
        return rsx! { sharing::WatchedScreen {} };
    }
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
                Screen::FirstRunLevel | Screen::FirstRunExtras | Screen::FirstRunOwnDeeds | Screen::FirstRunLocation | Screen::FirstRunWatcher | Screen::FirstRunSummary | Screen::SetPicker => "onboarding-content",
                Screen::Sheet => "panes",
                Screen::Reports => "report-panes",
                Screen::Deeds | Screen::Editor(_) | Screen::Settings => "page-content",
            },
                sharing::WatchRequestPrompt {}
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
                if state.read().screen == Screen::Sheet { {install_banner()} }
                match state.read().screen {
                    Screen::FirstRunLevel => rsx! { first_run::LevelStep {} },
                    Screen::FirstRunExtras => rsx! { first_run::ExtrasStep {} },
                    Screen::FirstRunOwnDeeds => rsx! { first_run::OwnDeedsStep {} },
                    Screen::FirstRunLocation => rsx! { first_run::LocationStep {} },
                    Screen::FirstRunWatcher => rsx! { first_run::WatcherStep {} },
                    Screen::FirstRunSummary => rsx! { first_run::SummaryStep {} },
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
