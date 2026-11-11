use super::context::{Actions, use_i18n};
use crate::account::{
    runtime::{AccountHandle, AccountView},
    sharing::{Reply, Request},
};
use crate::state::{AppState, Screen};
use dioxus::prelude::*;
use std::{rc::Rc, time::Duration};
use werdy_store::{MemoryStore, Store};
use werdy_sync::{
    SentWatchRequest, ShareCode, SharePerson, SyncError, WATCH_REQUEST_LIMIT, WatchDirection,
    WatchRequest, WatchView, normalize_email,
};

#[derive(Clone, Copy, Default)]
pub struct ReadOnly(pub bool);
pub fn read_only() -> bool {
    try_consume_context::<ReadOnly>().is_some_and(|value| value.0)
}

#[derive(Clone, Default)]
pub struct Sharing {
    watchers: Vec<SharePerson>,
    watched: Vec<SharePerson>,
    incoming: Vec<WatchRequest>,
    prompt_dismissed: bool,
    outgoing: Vec<SentWatchRequest>,
    request_email: String,
    request_sent: bool,
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
    fn prompt_visible(&self) -> bool {
        !self.prompt_dismissed && !self.incoming.is_empty()
    }

    /// The sheet shows its cards only once the prompt is gone, so one request
    /// never appears twice on screen.
    fn cards_visible(&self, view: ReadOnly) -> bool {
        !view.0 && !self.incoming.is_empty() && !self.prompt_visible()
    }

    fn dismiss_prompt(&mut self) {
        self.prompt_dismissed = true;
    }

    pub fn reset_account(&mut self, generation: Option<u64>) {
        // A new sign in shows waiting requests again, so Later is forgotten here.
        *self = Self {
            generation,
            ..Self::default()
        };
    }

    fn send_request(&mut self, direction: WatchDirection) -> Option<Request> {
        self.request_sent = false;
        match normalize_email(&self.request_email) {
            Ok(email) => Some(Request::SendWatchRequest(email, direction)),
            Err(_) => {
                self.error = Some("sharing-email-invalid");
                None
            }
        }
    }

    /// A sharing request and account deletion both refresh the session, so
    /// neither may start while the other runs.
    fn idle(&self) -> bool {
        !self.busy && !self.deleting
    }

    fn receive(&mut self, request: &Request, result: Result<Reply, SyncError>) -> bool {
        self.busy = false;
        match result {
            Ok(Reply::Lists(watchers, watched, incoming, outgoing)) => {
                self.watchers = watchers;
                self.watched = watched;
                self.incoming = incoming;
                self.outgoing = outgoing;
            }
            Ok(Reply::Code(code)) => self.code = Some(code),
            Ok(Reply::View(view)) => self.view = Some(*view),
            Ok(Reply::Changed) => {
                if matches!(request, Request::SendWatchRequest(..)) {
                    self.request_email.clear();
                    self.request_sent = true;
                }
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
            Err(SyncError::WatchRequestLimit)
                if matches!(request, Request::SendWatchRequest(..)) =>
            {
                self.error = Some("sharing-request-limit");
            }
            Err(_)
                if matches!(
                    request,
                    Request::SendWatchRequest(..)
                        | Request::Accept(_)
                        | Request::Decline(_)
                        | Request::Cancel(_)
                ) =>
            {
                self.error = Some("sharing-request-error");
            }
            Err(_) => self.error = Some("sharing-error"),
        }
        false
    }
}

fn request_consent_key(direction: WatchDirection) -> &'static str {
    match direction {
        WatchDirection::WatchMe => "sharing-request-consent",
        WatchDirection::WatchThem => "sharing-request-consent-watch-them",
    }
}

fn sent_direction_key(direction: WatchDirection) -> &'static str {
    match direction {
        WatchDirection::WatchMe => "sharing-request-sent-watch-me",
        WatchDirection::WatchThem => "sharing-request-sent-watch-them",
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
                    sharing.write().reset_account(generation);
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
    if !sharing.peek().idle() {
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
    if !sharing.peek().idle() {
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
            Ok(()) => sharing.write().reset_account(None),
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
/// The "Their Google email" field and send buttons, shared by the sharing
/// section and first run. `watch_them` adds "Ask to watch them"; `standalone`
/// shows progress and send errors here, for screens without the section's own.
pub fn WatchRequestForm(
    #[props(default)] watch_them: bool,
    #[props(default)] standalone: bool,
) -> Element {
    let i18n = use_i18n();
    let store = use_context::<AccountHandle>().0;
    let mut sharing = use_context::<Signal<Sharing>>();
    let (request_email, request_sent, busy, error) = {
        let request_state = sharing.read();
        (
            request_state.request_email.clone(),
            request_state.request_sent,
            request_state.busy,
            request_state.error,
        )
    };
    let send_error = error.filter(|key| {
        matches!(
            *key,
            "sharing-email-invalid"
                | "sharing-request-limit"
                | "sharing-request-error"
                | "sharing-error"
        )
    });
    rsx! {
        label { r#for:"watch-email", {i18n.t("sharing-request-email")} }
        input { id:"watch-email",dir:"ltr",value:"{request_email}",disabled:busy,
            oninput:move |event| {
                let mut sharing = sharing.write();
                sharing.request_email = event.value();
                sharing.request_sent = false;
            }
        }
        div { class:"settings-options",
            button { class:"secondary-button",disabled:busy,onclick:{let store=store.clone();move |_| {
                let request = sharing.write().send_request(WatchDirection::WatchMe);
                if let Some(request) = request { run(sharing,store.clone(),request); }
            }},{i18n.t("sharing-request-send")} }
            if watch_them {
                button { class:"secondary-button",disabled:busy,onclick:{let store=store.clone();move |_| {
                    let request = sharing.write().send_request(WatchDirection::WatchThem);
                    if let Some(request) = request { run(sharing,store.clone(),request); }
                }},{i18n.t("sharing-request-send-watch-them")} }
            }
        }
        if request_sent { p {role:"status",{i18n.t("sharing-request-sent")}} }
        if standalone {
            if busy { p {role:"status",{i18n.t("sharing-loading")}} }
            if let Some(error) = send_error {
                p {role:"alert",{i18n.t_with(error, &[("limit", fluent_bundle::FluentValue::from(WATCH_REQUEST_LIMIT))])}}
            }
        }
    }
}

#[component]
pub fn WatchRequestCard(
    request: WatchRequest,
    #[props(default)] on_later: Option<EventHandler>,
) -> Element {
    let i18n = use_i18n();
    let sharing = use_context::<Signal<Sharing>>();
    let store = use_context::<AccountHandle>().0;
    let disabled = !sharing.read().idle();
    rsx! {
        section { class: "settings-card watch-request-card", role: "region", aria_label: i18n.t("sharing-requests-region"),
            p { {i18n.t_with(request_consent_key(request.direction), &[("email", fluent_bundle::FluentValue::from(request.email.unwrap_or_else(||i18n.t("account-email-unknown"))))])} }
            div { class: "settings-options",
                button { class: "secondary-button", disabled, onclick: {let store=store.clone();let id=request.id.clone();move |_|run(sharing,store.clone(),Request::Accept(id.clone()))}, {i18n.t("sharing-request-accept")} }
                button { class: "secondary-button", disabled, onclick: {let id=request.id.clone();move |_|run(sharing,store.clone(),Request::Decline(id.clone()))}, {i18n.t("sharing-request-decline")} }
                if let Some(on_later) = on_later {
                    button { class: "secondary-button", onclick: move |_| on_later.call(()), {i18n.t("sharing-request-later")} }
                }
            }
            if let Some(error) = sharing.read().error {
                p { role: "alert", {i18n.t_with(error, &[("limit", fluent_bundle::FluentValue::from(WATCH_REQUEST_LIMIT))])} }
            }
        }
    }
}

#[component]
pub fn WatchRequestPrompt() -> Element {
    let i18n = use_i18n();
    let mut sharing = use_context::<Signal<Sharing>>();
    let requests = sharing.read();
    if !requests.prompt_visible() {
        return rsx! {};
    }
    rsx! {
        section { class: "settings-card watch-request-prompt", role: "region", aria_label: i18n.t("sharing-requests-region"),
            h2 { {i18n.t("sharing-requests")} }
            for request in requests.incoming.clone() {
                WatchRequestCard { key: "{request.id}", request, on_later: move |_| sharing.write().dismiss_prompt() }
            }
        }
    }
}

#[component]
pub fn SheetWatchRequests() -> Element {
    let sharing = use_context::<Signal<Sharing>>();
    let requests = sharing.read();
    if !requests.cards_visible(ReadOnly(read_only())) {
        return rsx! {};
    }
    rsx! {
        for request in requests.incoming.clone() {
            WatchRequestCard { key: "{request.id}", request }
        }
    }
}

#[component]
pub fn SharingSection() -> Element {
    let i18n = use_i18n();
    let status = use_context::<Signal<AccountView>>();
    let handle = use_context::<AccountHandle>();
    let mut sharing = use_context::<Signal<Sharing>>();
    // A share link fills the field; the watcher still taps Use code (#90).
    let mut input = use_signal(|| shared_code().unwrap_or_default());
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
                    if let Some(svg) = share_qr(&share_link(&code.code)) {
                        div { class:"share-qr",role:"img",aria_label:i18n.t("sharing-qr"),dangerous_inner_html:svg }
                        p { {i18n.t("sharing-qr-help")} }
                    }
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
                WatchRequestForm { watch_them: true }
                if !data.outgoing.is_empty() {
                    h3 {{i18n.t("sharing-waiting")}}
                    for request in data.outgoing {
                        div { key:"{request.id}",
                            p { span {dir:"auto",{request.email}} }
                            p {{i18n.t(sent_direction_key(request.direction))}}
                            p {{i18n.t_with("sharing-request-until", &[("date", fluent_bundle::FluentValue::from(crate::account::runtime::display_time(request.expires_at).unwrap_or_default()))])}}
                            button {class:"secondary-button",disabled:data.busy,onclick:{let store=store.clone();let id=request.id.clone();move |_|run(sharing,store.clone(),Request::Cancel(id.clone()))},{i18n.t("sharing-request-cancel")}}
                        }
                    }
                }
                h3 {{i18n.t("sharing-requests")}}
                for request in data.incoming {
                    WatchRequestCard { key: "{request.id}", request }
                }
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
            if let Some(error)=data.error {p {role:"alert",{i18n.t_with(error, &[("limit", fluent_bundle::FluentValue::from(WATCH_REQUEST_LIMIT))])}}}
        }
    }
}

/// Link a watcher opens to get the code filled in (#90).
pub fn share_link(code: &str) -> String {
    format!("{}?share={code}", crate::account::config::APP_URL)
}

/// The `share` code from an address query, or None when it is missing or
/// could not be a share code.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn share_from_query(query: &str) -> Option<String> {
    let code = query
        .trim_start_matches('?')
        .split('&')
        .find_map(|part| part.strip_prefix("share="))?
        .to_ascii_uppercase();
    (!code.is_empty() && code.len() <= 32 && code.bytes().all(|b| b.is_ascii_alphanumeric()))
        .then_some(code)
}

/// The share code in this page's address, on the web.
pub fn shared_code() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.location().search().ok())
            .and_then(|query| share_from_query(&query))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// The share link as an SVG QR. Colours come from CSS through currentColor.
pub fn share_qr(link: &str) -> Option<String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::new(link.as_bytes()).ok()?;
    Some(
        code.render::<svg::Color>()
            .min_dimensions(176, 176)
            .dark_color(svg::Color("currentColor"))
            .light_color(svg::Color("transparent"))
            .build(),
    )
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

    fn waiting_request() -> WatchRequest {
        WatchRequest {
            id: "request".into(),
            email: Some("sender@example.test".into()),
            direction: WatchDirection::WatchMe,
            created_at: 1,
        }
    }

    #[test]
    fn incoming_lists_show_prompt_and_own_sheet_cards() {
        let mut sharing = Sharing::default();
        assert!(!sharing.prompt_visible());
        assert!(!sharing.cards_visible(ReadOnly(false)));
        sharing.receive(
            &Request::Lists,
            Ok(Reply::Lists(
                vec![],
                vec![],
                vec![waiting_request()],
                vec![],
            )),
        );
        assert!(sharing.prompt_visible());
        assert!(!sharing.cards_visible(ReadOnly(false)));
        sharing.dismiss_prompt();
        assert!(sharing.cards_visible(ReadOnly(false)));
        assert!(!sharing.cards_visible(ReadOnly(true)));
    }

    #[test]
    fn later_hides_prompt_until_the_next_sign_in_but_keeps_sheet_cards() {
        let mut sharing = Sharing {
            incoming: vec![waiting_request()],
            ..Sharing::default()
        };
        sharing.dismiss_prompt();
        sharing.receive(
            &Request::Lists,
            Ok(Reply::Lists(
                vec![],
                vec![],
                vec![waiting_request()],
                vec![],
            )),
        );
        assert!(!sharing.prompt_visible());
        assert!(sharing.cards_visible(ReadOnly(false)));
        sharing.reset_account(Some(2));
        sharing.receive(
            &Request::Lists,
            Ok(Reply::Lists(
                vec![],
                vec![],
                vec![waiting_request()],
                vec![],
            )),
        );
        assert!(sharing.prompt_visible());
        assert!(!sharing.cards_visible(ReadOnly(false)));
    }

    #[test]
    fn answering_last_request_hides_prompt_and_cards_after_refresh() {
        for request in [
            Request::Accept("request".into()),
            Request::Decline("request".into()),
        ] {
            let mut sharing = Sharing {
                incoming: vec![waiting_request()],
                ..Sharing::default()
            };
            assert!(sharing.receive(&request, Ok(Reply::Changed)));
            assert!(sharing.prompt_visible());
            sharing.receive(
                &Request::Lists,
                Ok(Reply::Lists(vec![], vec![], vec![], vec![])),
            );
            assert!(!sharing.prompt_visible());
            assert!(!sharing.cards_visible(ReadOnly(false)));
        }
    }

    #[test]
    fn answering_one_request_keeps_the_others_visible() {
        let remaining = WatchRequest {
            id: "remaining".into(),
            direction: WatchDirection::WatchThem,
            ..waiting_request()
        };
        let mut sharing = Sharing {
            incoming: vec![waiting_request(), remaining.clone()],
            ..Sharing::default()
        };
        assert!(sharing.receive(&Request::Accept("request".into()), Ok(Reply::Changed)));
        sharing.receive(
            &Request::Lists,
            Ok(Reply::Lists(
                vec![],
                vec![],
                vec![remaining.clone()],
                vec![],
            )),
        );
        assert_eq!(sharing.incoming, vec![remaining]);
        assert!(sharing.prompt_visible());
        sharing.dismiss_prompt();
        assert!(sharing.cards_visible(ReadOnly(false)));
    }

    #[test]
    fn send_and_cancel_refresh_waiting_requests() {
        let mut sharing = Sharing::default();
        let sent = werdy_sync::SentWatchRequest {
            id: "request".into(),
            email: "receiver@example.test".into(),
            direction: WatchDirection::WatchMe,
            created_at: 1,
            expires_at: 2592000001,
        };
        assert!(sharing.receive(
            &Request::SendWatchRequest(sent.email.clone(), sent.direction),
            Ok(Reply::Changed)
        ));
        sharing.receive(
            &Request::Lists,
            Ok(Reply::Lists(vec![], vec![], vec![], vec![sent.clone()])),
        );
        assert_eq!(sharing.outgoing, vec![sent.clone()]);
        assert!(sharing.receive(&Request::Cancel(sent.id), Ok(Reply::Changed)));
        sharing.receive(
            &Request::Lists,
            Ok(Reply::Lists(vec![], vec![], vec![], vec![])),
        );
        assert!(sharing.outgoing.is_empty());
    }

    #[test]
    fn request_limit_keeps_typed_email_and_shows_specific_message() {
        let mut sharing = Sharing {
            request_email: " Receiver@Example.test ".into(),
            ..Sharing::default()
        };
        let request = sharing.send_request(WatchDirection::WatchThem).unwrap();
        assert!(!sharing.receive(&request, Err(SyncError::WatchRequestLimit)));
        assert_eq!(sharing.error, Some("sharing-request-limit"));
        assert_eq!(sharing.request_email, " Receiver@Example.test ");
        assert!(!sharing.request_sent);
        assert!(sharing.idle());
    }

    #[test]
    fn waiting_text_explains_direction_expiry_and_variable_limit() {
        use crate::i18n::{Language, Texts};
        use fluent_bundle::FluentValue;
        let texts = Texts::load();
        for (language, watch_me, watch_them, until, limit) in [
            (
                Language::English,
                "You asked them to watch you",
                "You asked to watch them",
                "Until DATE",
                "You have 4 requests waiting. Cancel one to send another.",
            ),
            (
                Language::Arabic,
                "طلبت منهم متابعتك",
                "طلبت متابعتهم",
                "حتى DATE",
                "لديك 4 طلبات بانتظار الرد. ألغِ أحدها لإرسال طلب آخر.",
            ),
        ] {
            for (key, args, expected) in [
                (
                    sent_direction_key(WatchDirection::WatchMe),
                    vec![],
                    watch_me,
                ),
                (
                    sent_direction_key(WatchDirection::WatchThem),
                    vec![],
                    watch_them,
                ),
                (
                    "sharing-request-until",
                    vec![("date", FluentValue::from("DATE"))],
                    until,
                ),
                (
                    "sharing-request-limit",
                    vec![("limit", FluentValue::from(4))],
                    limit,
                ),
            ] {
                assert_eq!(
                    texts
                        .get(language, key, &args)
                        .replace(['\u{2068}', '\u{2069}'], ""),
                    expected
                );
            }
        }
    }

    #[test]
    fn accepting_either_direction_refreshes_the_matching_people_list() {
        for direction in [WatchDirection::WatchMe, WatchDirection::WatchThem] {
            let mut sharing = Sharing::default();
            let request = werdy_sync::WatchRequest {
                id: "request".into(),
                email: Some("owner@example.test".into()),
                direction,
                created_at: 1,
            };
            sharing.receive(
                &Request::Lists,
                Ok(Reply::Lists(vec![], vec![], vec![request], vec![])),
            );
            assert!(sharing.receive(&Request::Accept("request".into()), Ok(Reply::Changed)));
            let person = SharePerson {
                id: "person".into(),
                email: Some("person@example.test".into()),
                created_at: 2,
            };
            let (watchers, watched) = match direction {
                WatchDirection::WatchMe => (vec![], vec![person]),
                WatchDirection::WatchThem => (vec![person], vec![]),
            };
            sharing.receive(
                &Request::Lists,
                Ok(Reply::Lists(
                    watchers.clone(),
                    watched.clone(),
                    vec![],
                    vec![],
                )),
            );
            assert!(sharing.incoming.is_empty());
            assert_eq!(sharing.watchers, watchers);
            assert_eq!(sharing.watched, watched);
        }
    }

    #[test]
    fn failed_send_keeps_email_for_retry_and_success_clears_it() {
        for direction in [WatchDirection::WatchMe, WatchDirection::WatchThem] {
            let mut sharing = Sharing {
                request_email: " A@B ".into(),
                ..Sharing::default()
            };
            let request = sharing.send_request(direction).unwrap();
            assert!(
                matches!(&request, Request::SendWatchRequest(email, sent_direction) if email == "a@b" && *sent_direction == direction)
            );
            sharing.receive(&request, Err(SyncError::Http(503)));
            assert_eq!(sharing.request_email, " A@B ");
            assert_eq!(sharing.error, Some("sharing-request-error"));
            assert!(sharing.idle());
            assert!(sharing.send_request(direction).is_some());
            assert!(sharing.receive(&request, Ok(Reply::Changed)));
            assert!(sharing.request_email.is_empty());
            assert!(sharing.request_sent);
            sharing.request_email = "bad".into();
            assert!(sharing.send_request(direction).is_none());
            assert_eq!(sharing.error, Some("sharing-email-invalid"));
        }
    }

    #[test]
    fn incoming_direction_explains_who_will_see_the_sheet() {
        assert_eq!(
            request_consent_key(WatchDirection::WatchMe),
            "sharing-request-consent"
        );
        assert_eq!(
            request_consent_key(WatchDirection::WatchThem),
            "sharing-request-consent-watch-them"
        );
    }

    #[test]
    fn share_link_carries_the_code_and_reads_back_from_the_address() {
        let link = share_link("ABCDEFGH23");
        assert_eq!(
            link,
            "https://ahmed-ahmed16.github.io/werdy/?share=ABCDEFGH23"
        );
        assert_eq!(
            share_from_query("?demo&share=abcdefgh23").as_deref(),
            Some("ABCDEFGH23")
        );
    }

    #[test]
    fn address_without_a_usable_code_fills_nothing() {
        assert_eq!(share_from_query(""), None);
        assert_eq!(share_from_query("?share="), None);
        assert_eq!(share_from_query("?share=AB%3Cscript"), None);
        assert_eq!(
            share_from_query(&format!("?share={}", "A".repeat(33))),
            None
        );
    }

    #[test]
    fn share_qr_takes_its_colours_from_css() {
        // The CSS frame sets qr-ink and qr-paper; fixed colours here would
        // break scanning in the dark theme.
        let svg = share_qr(&share_link("ABCDEFGH23")).expect("a short link fits in a QR");
        assert!(svg.contains("currentColor"));
        assert!(svg.contains("transparent"));
        assert!(!svg.contains("#000"));
    }
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

    #[test]
    fn sharing_requests_and_account_deletion_never_overlap() {
        assert!(Sharing::default().idle());
        let busy = Sharing {
            busy: true,
            ..Sharing::default()
        };
        let deleting = Sharing {
            deleting: true,
            ..Sharing::default()
        };
        assert!(!busy.idle());
        assert!(!deleting.idle());
    }
}
