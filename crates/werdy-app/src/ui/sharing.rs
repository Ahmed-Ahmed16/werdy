use super::context::{Actions, use_i18n};
use crate::account::{
    runtime::{AccountHandle, AccountView},
    sharing::{Reply, Request},
};
use crate::state::{AppState, Screen};
use dioxus::prelude::*;
use std::{rc::Rc, time::Duration};
use werdy_store::{MemoryStore, Store};
use werdy_sync::{ShareCode, SharePerson, SyncError, WatchView};

#[derive(Clone, Copy, Default)]
pub struct ReadOnly(pub bool);
pub fn read_only() -> bool {
    try_consume_context::<ReadOnly>().is_some_and(|value| value.0)
}

#[derive(Clone, Default)]
pub struct Sharing {
    watchers: Vec<SharePerson>,
    watched: Vec<SharePerson>,
    code: Option<ShareCode>,
    pub selected: Option<SharePerson>,
    view: Option<WatchView>,
    busy: bool,
    error: Option<&'static str>,
    generation: Option<u64>,
    pub deleting: bool,
    pub delete_error: Option<&'static str>,
}

impl Sharing {
    fn receive(&mut self, request: &Request, result: Result<Reply, SyncError>) -> bool {
        self.busy = false;
        match result {
            Ok(Reply::Lists(watchers, watched)) => {
                self.watchers = watchers;
                self.watched = watched;
            }
            Ok(Reply::Code(code)) => self.code = Some(code),
            Ok(Reply::View(view)) => self.view = Some(*view),
            Ok(Reply::Changed) => {
                if let Request::Leave(id) = request {
                    self.watched.retain(|person| &person.id != id);
                }
                if let Request::Revoke(id) = request {
                    self.watchers.retain(|person| &person.id != id);
                }
                return true;
            }
            Err(SyncError::Http(403)) if matches!(request, Request::Read(..)) => {
                if let Request::Read(id, _) = request {
                    self.watched.retain(|person| &person.id != id);
                }
                self.selected = None;
                self.view = None;
                self.error = Some("sharing-revoked");
            }
            Err(_) => self.error = Some("sharing-error"),
        }
        false
    }
}

pub fn use_sharing() -> Signal<Sharing> {
    let mut sharing = use_signal(Sharing::default);
    use_context_provider(|| sharing);
    let account = use_context::<AccountHandle>();
    use_future(move || {
        let account = account.0.clone();
        async move {
            loop {
                let generation = {
                    let account = account.account.borrow();
                    account.session.as_ref().map(|_| account.generation)
                };
                if sharing.peek().generation != generation {
                    sharing.set(Sharing {
                        generation,
                        ..Sharing::default()
                    });
                    if generation.is_some() {
                        run(sharing, account.clone(), Request::Lists);
                    }
                }
                crate::account::runtime::sleep(Duration::from_secs(1)).await;
            }
        }
    });
    sharing
}

struct Flight(Rc<crate::account::AccountStore>, u64);
impl Drop for Flight {
    fn drop(&mut self) {
        let mut account = self.0.account.borrow_mut();
        if account.generation == self.1 {
            account.sharing_busy = false;
        }
    }
}

fn run(mut sharing: Signal<Sharing>, store: Rc<crate::account::AccountStore>, request: Request) {
    if sharing.peek().busy {
        return;
    }
    sharing.write().busy = true;
    sharing.write().error = None;
    let generation = store.account.borrow().generation;
    // Settings unmounts when the watched screen opens. Keep its request alive
    // in the root scope; account generation still rejects replies after sign out.
    dioxus::dioxus_core::spawn_forever(async move {
        if !wait_for_sync(&store, generation).await {
            return;
        }
        store.account.borrow_mut().sharing_busy = true;
        let _flight = Flight(store.clone(), generation);
        let result = crate::account::sharing::execute(&store, &request, generation).await;
        if store.account.borrow().generation != generation {
            return;
        }
        let reload = sharing.write().receive(&request, result);
        if reload {
            drop(_flight);
            run(sharing, store, Request::Lists);
        }
    });
}

/// Serializes token refresh with owner sync. No borrow crosses an await.
/// Returns false when the account changed while waiting.
async fn wait_for_sync(store: &crate::account::AccountStore, generation: u64) -> bool {
    loop {
        if store.account.borrow().generation != generation {
            return false;
        }
        if !store.account.borrow().schedule.is_running() {
            return true;
        }
        crate::account::runtime::sleep(Duration::from_millis(100)).await;
    }
}

/// Deletes the server account under the same guard as sharing requests. The
/// state lives in the root sharing signal, so leaving Settings cannot drop it.
pub fn delete_account(
    mut sharing: Signal<Sharing>,
    state: Signal<AppState>,
    store: Rc<crate::account::AccountStore>,
) {
    if sharing.peek().busy || sharing.peek().deleting {
        return;
    }
    sharing.write().deleting = true;
    sharing.write().delete_error = None;
    let generation = store.account.borrow().generation;
    dioxus::dioxus_core::spawn_forever(async move {
        if !wait_for_sync(&store, generation).await {
            sharing.write().deleting = false;
            return;
        }
        store.account.borrow_mut().sharing_busy = true;
        let flight = Flight(store.clone(), generation);
        let result = crate::account::sharing::execute(&store, &Request::DeleteAccount, generation)
            .await
            .map(|_| ());
        drop(flight);
        let records = state.peek().data.clone();
        match store.settle_delete(result, &records, generation) {
            Ok(()) => sharing.set(Sharing::default()),
            Err(key) => {
                let mut sharing = sharing.write();
                sharing.deleting = false;
                sharing.delete_error = Some(key);
            }
        }
    });
}

fn refresh_view(mut sharing: Signal<Sharing>, store: Rc<crate::account::AccountStore>) {
    let data = sharing.peek().clone();
    let Some(person) = data.selected else {
        return;
    };
    let view = data.view.map(Ok).unwrap_or_else(WatchView::new);
    match view {
        Ok(view) => run(sharing, store, Request::Read(person.id, Box::new(view))),
        Err(_) => sharing.write().error = Some("sharing-error"),
    }
}

#[component]
pub fn SharingSection() -> Element {
    let i18n = use_i18n();
    let status = use_context::<Signal<AccountView>>();
    let handle = use_context::<AccountHandle>();
    let mut sharing = use_context::<Signal<Sharing>>();
    let mut input = use_signal(String::new);
    let mut confirm = use_signal(|| None::<String>);
    let mut copied = use_signal(|| None::<bool>);
    if !handle.0.enabled {
        return rsx! {};
    }
    let data = sharing.read().clone();
    let store = handle.0;
    rsx! {
        section { class: "settings-card",
            h2 { {i18n.t("sharing-title")} }
            if !status.read().signed_in { p { {i18n.t("sharing-signin")} } }
            else {
                p { {i18n.t("sharing-purpose")} }
                button { class:"secondary-button",disabled:data.busy,onclick:{let store=store.clone(); move |_| run(sharing,store.clone(),Request::Create)}, {i18n.t("sharing-create")} }
                if let Some(code) = data.code {
                    p { class:"share-code",dir:"ltr", "{code.code}" }
                    p { {i18n.t("sharing-expires")} " " {crate::account::runtime::display_time(code.expires_at).unwrap_or_default()} }
                    button { class:"secondary-button",onclick: move |_| {
                        let code=code.code.clone(); spawn(async move { copied.set(Some(copy_code(code).await)); });
                    }, {i18n.t("sharing-copy")} }
                    if let Some(copied)=copied() { p {role:"status",{i18n.t(if copied {"sharing-copied"} else {"sharing-copy-failed"})} } }
                }
                h3 { {i18n.t("sharing-watchers")} }
                for person in data.watchers {
                    div {key:"{person.id}",
                        span {dir:"auto",{person.email.unwrap_or_else(||i18n.t("account-email-unknown"))}}
                        if confirm().as_deref()==Some(&person.id) {
                            p {{i18n.t("sharing-confirm")}}
                            button {disabled:data.busy,onclick:{let store=store.clone();let id=person.id.clone();move |_| {run(sharing,store.clone(),Request::Revoke(id.clone()));confirm.set(None);}}, {i18n.t("sharing-revoke")} }
                            button {onclick:move |_| confirm.set(None),{i18n.t("note-cancel")} }
                        } else { button {disabled:data.busy,onclick:move |_| confirm.set(Some(person.id.clone())),{i18n.t("sharing-revoke")} } }
                    }
                }
                label {r#for:"share-code",{i18n.t("sharing-enter")}}
                input {id:"share-code",dir:"ltr",value:"{input}",maxlength:"32",oninput:move |event|input.set(event.value())}
                button {disabled:data.busy,onclick:{let store=store.clone(); move |_| {
                    let code=input();input.set(String::new());run(sharing,store.clone(),Request::Redeem(code));
                }},{i18n.t("sharing-redeem")}}
                h3 {{i18n.t("sharing-watched")}}
                for person in data.watched {
                    div {key:"{person.id}",
                        button {disabled:data.busy,onclick:{let store=store.clone();let person=person.clone(); move |_| {
                            match WatchView::new() {
                                Ok(view)=>{sharing.write().selected=Some(person.clone());sharing.write().view=None;run(sharing,store.clone(),Request::Read(person.id.clone(),Box::new(view)));}
                                Err(_)=>sharing.write().error=Some("sharing-error"),
                            }
                        }},{person.email.clone().unwrap_or_else(||i18n.t("account-email-unknown"))}}
                        button {disabled:data.busy,onclick:{let store=store.clone();move |_|run(sharing,store.clone(),Request::Leave(person.id.clone()))},{i18n.t("sharing-leave")}}
                    }
                }
                button {disabled:data.busy,onclick:move |_|run(sharing,store.clone(),Request::Lists),{i18n.t("sharing-refresh")}}
            }
            if data.busy {p {role:"status",{i18n.t("sharing-loading")}}}
            if let Some(error)=data.error {p {role:"alert",{i18n.t(error)}}}
        }
    }
}

async fn copy_code(code: String) -> bool {
    let mut eval = document::eval(
        r#"const code=await dioxus.recv(); try {
        if(navigator.clipboard) {await navigator.clipboard.writeText(code); dioxus.send(true);}
        else {const field=document.createElement('textarea');field.value=code;document.body.appendChild(field);
        field.select();const ok=document.execCommand('copy');field.remove();dioxus.send(ok);}
    } catch (_) {dioxus.send(false);} "#,
    );
    if eval.send(code).is_err() {
        return false;
    }
    eval.recv::<bool>().await.unwrap_or(false)
}

#[component]
pub fn WatchedScreen() -> Element {
    let i18n = use_i18n();
    let own = use_context::<Signal<AppState>>();
    let theme = match own.read().data.settings.theme {
        werdy_store::Theme::System => "system",
        werdy_store::Theme::Light => "light",
        werdy_store::Theme::Dark => "dark",
    };
    let mut sharing = use_context::<Signal<Sharing>>();
    let handle = use_context::<AccountHandle>();
    let data = sharing.read().clone();
    let polling_store = handle.0.clone();
    use_future(move || {
        let store = polling_store.clone();
        async move {
            loop {
                crate::account::runtime::sleep(Duration::from_secs(30)).await;
                refresh_view(sharing, store.clone());
            }
        }
    });
    rsx! {
        div {class:"werdy-root",dir:i18n.language.dir(),lang:i18n.language.code(),"data-theme":theme,
        main {class:"page-content",
            div {class:"demo-banner",role:"note",
                span { {i18n.t("sharing-readonly")} " " {data.selected.and_then(|p|p.email).unwrap_or_default()} }
                button {onclick:move |_|{sharing.write().selected=None;sharing.write().view=None;},{i18n.t("sharing-back")}}
                button {disabled:data.busy,onclick:move |_|{
                    refresh_view(sharing,handle.0.clone());
                },{i18n.t("sharing-refresh")}}
            }
            p {{i18n.t("sharing-day-boundary")}}
            if let Some(error)=data.error {p {role:"alert",{i18n.t(error)}}}
            if let Some(view)=data.view {
                if let Ok(today)=crate::platform::today() {
                    WatchedContent {view, today, language:i18n.language.code().to_owned()}
                } else {p {role:"alert",{i18n.t("app-date-failed")}}}
            } else {p {role:"status",{i18n.t("sharing-loading")}}}
        }
        }
    }
}

#[component]
fn WatchedContent(view: WatchView, language: String, today: werdy_core::WerdyDay) -> Element {
    let memory = use_hook(|| Rc::new(MemoryStore::default()));
    let mut state =
        use_signal(|| AppState::start(memory.as_ref(), today, crate::starter::slot_order()));
    let store = use_signal(|| {
        let store: Rc<dyn Store> = memory;
        store
    });
    use_context_provider(|| state);
    use_context_provider(|| Actions::read_only(state, store));
    use_context_provider(|| ReadOnly(true));
    let cursor = view.cursor();
    use_effect(use_reactive(
        (&view, &language, &today),
        move |(view, language, today)| {
            let mut records = view.records().clone();
            records.settings.language = language;
            let mut state = state.write();
            state.data = records;
            state.update_today(today);
            state.welcome_visible = false;
        },
    ));
    let i18n = use_i18n();
    rsx! {
        div {"data-cursor":"{cursor}",
            button {onclick:move |_|state.write().show_screen(Screen::Sheet),{i18n.t("nav-sheet")}}
            button {onclick:move |_|state.write().show_screen(Screen::Reports),{i18n.t("nav-reports")}}
            if state.read().screen==Screen::Reports {super::reports::ReportsScreen {}}
            else {
                super::header::DayHeader {on_open_note:move |_|{}}
                super::summary::TodayCard {}
                if let Some(note)=state.read().viewing_note() {
                    p {dir:"auto",{note.text()}}
                    p {{i18n.t(match note.credit(){werdy_core::ExcuseCredit::None=>"note-credit-none",werdy_core::ExcuseCredit::Half=>"note-credit-half",werdy_core::ExcuseCredit::Full=>"note-credit-full"})}}
                }
                super::sheet::Sheet {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revoked_access_drops_view_and_list_but_network_failure_allows_retry() -> Result<(), SyncError>
    {
        let person = SharePerson {
            id: "owner".into(),
            email: Some("owner@example.test".into()),
            created_at: 1,
        };
        let mut sharing = Sharing {
            watched: vec![person.clone()],
            selected: Some(person),
            view: Some(WatchView::new()?),
            busy: true,
            ..Sharing::default()
        };
        let request = Request::Read("owner".into(), Box::new(WatchView::new()?));
        assert!(!sharing.receive(&request, Err(SyncError::Backend("offline".into()))));
        assert!(sharing.view.is_some() && sharing.selected.is_some());
        assert!(!sharing.busy);
        assert!(!sharing.receive(&request, Err(SyncError::Http(403))));
        assert!(sharing.view.is_none() && sharing.selected.is_none() && sharing.watched.is_empty());
        assert_eq!(sharing.error, Some("sharing-revoked"));
        Ok(())
    }
}
