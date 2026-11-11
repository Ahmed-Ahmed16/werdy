use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use dioxus::{
    document::{self, Eval},
    prelude::{spawn, warn},
};
use serde::Deserialize;
use werdy_core::WerdyDay;
use werdy_store::OwnerData;
use werdy_sync::{PushSubscription, SupabaseAuth, SupabaseBackend};

use crate::{
    account::{AccountStore, config, runtime},
    i18n::Texts,
    reminders,
};

const ENABLED_KEY: &str = "werdy.push.enabled";

#[derive(Clone, Copy, Default, Deserialize)]
pub struct BrowserStatus {
    pub supported: bool,
    pub ios: bool,
    pub standalone: bool,
    pub permission: Permission,
}

#[derive(Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Granted,
    Denied,
    #[default]
    Default,
}

#[derive(Default)]
struct Queue {
    store: Weak<AccountStore>,
    pending: Option<OwnerData>,
    running: bool,
}

thread_local! { static QUEUE: RefCell<Queue> = RefCell::new(Queue::default()); }

pub fn register(store: Rc<AccountStore>) {
    QUEUE.with_borrow_mut(|queue| queue.store = Rc::downgrade(&store));
}

pub fn enabled() -> bool {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(ENABLED_KEY).ok().flatten())
        .is_some_and(|value| value == "true")
}

fn set_enabled(value: bool) -> Result<(), String> {
    let storage = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .ok_or_else(|| "browser storage unavailable".to_owned())?;
    storage
        .set_item(ENABLED_KEY, if value { "true" } else { "false" })
        .map_err(|_| "browser storage unavailable".to_owned())
}

fn browser(operation: &str) -> Eval {
    document::eval(&format!(
        "const operation={operation:?}; const key={:?};\n{}",
        config::VAPID_PUBLIC_KEY,
        include_str!("push.js")
    ))
}

pub async fn status() -> BrowserStatus {
    browser("status")
        .join::<BrowserStatus>()
        .await
        .unwrap_or_default()
}

/// Created in the tap handler so browser permission starts with user activation.
pub fn subscribe_from_tap() -> Eval {
    browser("subscribe")
}

pub async fn finish_subscribe(request: Eval) -> Result<bool, String> {
    let subscription = request
        .join::<Option<PushSubscription>>()
        .await
        .map_err(|_| "push subscription failed".to_owned())?;
    if subscription.is_none() {
        return Ok(false);
    }
    set_enabled(true)?;
    Ok(true)
}

async fn current() -> Result<Option<PushSubscription>, String> {
    browser("current")
        .join::<Option<PushSubscription>>()
        .await
        .map_err(|_| "push subscription lookup failed".to_owned())
}

pub async fn forget(store: &AccountStore) -> Result<(), String> {
    forget_with_intent(store, super::ForgetIntent::Settings).await
}

pub async fn forget_on_sign_out(store: &AccountStore) -> Result<(), String> {
    forget_with_intent(store, super::ForgetIntent::SignOut).await
}

async fn forget_with_intent(
    store: &AccountStore,
    intent: super::ForgetIntent,
) -> Result<(), String> {
    if matches!(intent, super::ForgetIntent::SignOut) {
        set_enabled(false)?;
    }
    if config::VAPID_PUBLIC_KEY.is_empty() || !status().await.supported {
        return Ok(());
    }
    let was_enabled = enabled();
    if matches!(intent, super::ForgetIntent::Settings) {
        set_enabled(false)?;
    }
    let mut unsubscribed = false;
    let result = async {
        if let Some(subscription) = current().await? {
            super::unsubscribe_then_forget(
                async {
                    browser("unsubscribe")
                        .join::<bool>()
                        .await
                        .map_err(|_| "push unsubscribe failed".to_owned())?;
                    unsubscribed = true;
                    Ok(())
                },
                || async {
                    let (backend, _) = backend(store).await?;
                    backend
                        .push_forget(&subscription.endpoint)
                        .await
                        .map_err(|error| error.to_string())
                },
            )
            .await?;
        }
        Ok(())
    }
    .await;
    if result.is_err() && super::restore_push_flag_after_failure(intent, unsubscribed) {
        let _ = set_enabled(was_enabled);
    }
    result
}

pub fn enqueue(owner: &OwnerData) {
    if !super::push_upload_allowed(super::data_mode())
        || config::VAPID_PUBLIC_KEY.is_empty()
        || !enabled()
    {
        return;
    }
    let store = QUEUE.with_borrow_mut(|queue| {
        let store = queue.store.upgrade()?;
        store.account.borrow().session.as_ref()?;
        queue.pending = Some(owner.clone());
        if queue.running {
            return None;
        }
        queue.running = true;
        Some(store)
    });
    if let Some(store) = store {
        spawn(async move {
            loop {
                let next = QUEUE.with_borrow_mut(|queue| {
                    let pending = queue.pending.take();
                    if pending.is_none() {
                        queue.running = false;
                    }
                    pending
                });
                let Some(owner) = next else { break };
                if let Err(error) = replace(&store, &owner).await {
                    warn!("Push reminder replacement failed: {error}");
                }
            }
        });
    }
}

async fn backend(store: &AccountStore) -> Result<(SupabaseBackend, u64), String> {
    let (mut session, generation) = {
        let account = store.account.borrow();
        (
            account
                .session
                .clone()
                .ok_or_else(|| "signed out".to_owned())?,
            account.generation,
        )
    };
    if session.needs_refresh(crate::platform::change_time() / 1000, false) {
        let auth = SupabaseAuth::new(config::SUPABASE_URL, config::SUPABASE_KEY)
            .map_err(|error| error.to_string())?;
        session = auth
            .refresh(&session, crate::platform::change_time() / 1000)
            .await
            .map_err(|error| error.to_string())?;
        runtime::save_refresh(store, &session, generation).map_err(|error| error.to_string())?;
    }
    if store.account.borrow().generation != generation {
        return Err("account changed".into());
    }
    let backend = SupabaseBackend::new(
        config::SUPABASE_URL,
        config::SUPABASE_KEY,
        &session.access_token,
    )
    .map_err(|error| error.to_string())?;
    Ok((backend, generation))
}

fn offset_at(seconds: i64) -> i32 {
    let instant = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(seconds as f64 * 1000.0));
    -(instant.get_timezone_offset() as i32)
}

fn plan(owner: &OwnerData) -> Result<werdy_reminders::Plan, String> {
    let now = js_sys::Date::new_0();
    let date = WerdyDay::new(
        now.get_full_year() as i32,
        (now.get_month() + 1) as u8,
        now.get_date() as u8,
    )
    .map_err(|error| error.to_string())?;
    let moment =
        werdy_reminders::LocalMoment::new(date, now.get_hours() as u8, now.get_minutes() as u8)
            .map_err(|error| error.to_string())?;
    let mut midnights = std::collections::BTreeMap::new();
    let mut date = date.previous().map_err(|error| error.to_string())?;
    for _ in 0..9 {
        midnights.insert(
            date,
            reminders::day_start_with_lookup(date, offset_at)? / 1000,
        );
        date = date.next().map_err(|error| error.to_string())?;
    }
    werdy_reminders::schedule(owner, moment, 7, |date, minute| {
        midnights.get(&date).map_or(i32::MAX, |midnight| {
            offset_at(*midnight + i64::from(minute) * 60)
        })
    })
    .map_err(|error| error.to_string())
}

async fn replace(store: &AccountStore, owner: &OwnerData) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    let subscription = current()
        .await?
        .ok_or_else(|| "push subscription missing".to_owned())?;
    let plan = plan(owner)?;
    let batch = reminders::web_batch(owner, &plan, &Texts::load(), |date, minute| {
        reminders::epoch_with_lookup(date, minute, offset_at)
    })?;
    let (backend, generation) = backend(store).await?;
    if !enabled() || store.account.borrow().generation != generation {
        return Ok(());
    }
    backend
        .push_replace(&subscription, &batch)
        .await
        .map_err(|error| error.to_string())?;
    if !enabled() || store.account.borrow().generation != generation {
        backend
            .push_forget(&subscription.endpoint)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
