use super::{AccountStore, Checkpoint, config};
use crate::state::AppState;
use dioxus::prelude::*;
use std::{rc::Rc, time::Duration};
use werdy_store::Store;
use werdy_sync::{Session, SupabaseAuth, SupabaseBackend, SyncError};

#[derive(Clone)]
pub struct AccountHandle(pub Rc<AccountStore>);

#[derive(Clone, Default, PartialEq)]
pub struct AccountView {
    pub email: Option<String>,
    pub signed_in: bool,
    pub last_sync: Option<u64>,
    pub error: Option<&'static str>,
    pub busy: bool,
}

pub fn view(store: &AccountStore) -> AccountView {
    let account = store.account.borrow();
    AccountView {
        email: account
            .session
            .as_ref()
            .and_then(|session| session.email.clone()),
        signed_in: account.session.is_some(),
        last_sync: account
            .checkpoint
            .as_ref()
            .and_then(|saved| saved.last_sync),
        error: account.error,
        busy: account.signing_in || account.schedule.running,
    }
}

pub fn use_background(mut state: Signal<AppState>) {
    let handle = use_context::<AccountHandle>();
    let initial = handle.0.clone();
    let mut status = use_signal(|| view(&initial));
    use_context_provider(|| status);
    use_future(move || {
        let store = handle.0.clone();
        async move {
            loop {
                let flight =
                    {
                        let mut account = store.account.borrow_mut();
                        if account.session.is_some()
                            && !account.sharing_busy
                            && account.schedule.start(elapsed_ms())
                        {
                            account.session.clone().zip(account.checkpoint.clone()).map(
                                |(session, checkpoint)| (session, checkpoint, account.generation),
                            )
                        } else {
                            None
                        }
                    };
                if let Some((session, checkpoint, generation)) = flight {
                    let store = store.clone();
                    let active = store.clone();
                    let task = spawn(async move {
                        let outcome = attempt(&store, session, checkpoint, generation).await;
                        if store.account.borrow().generation != generation {
                            return;
                        }
                        let success = match outcome {
                            Ok(completed) => {
                                let latest = state.read().data.clone();
                                let needs_save =
                                    store.load().map(|saved| saved.as_ref() != Some(&latest));
                                let saved =
                                    needs_save
                                        .and_then(|changed| {
                                            if changed { store.save(&latest) } else { Ok(()) }
                                        })
                                        .and_then(|()| {
                                            store.complete(
                                                &completed,
                                                generation,
                                                crate::platform::change_time(),
                                            )
                                        });
                                match saved {
                                    Ok(Some(records)) => {
                                        state.write().accept_sync(records);
                                        true
                                    }
                                    Ok(None) => false,
                                    Err(_) => {
                                        store.account.borrow_mut().error =
                                            Some("account-storage-error");
                                        false
                                    }
                                }
                            }
                            Err(error) => {
                                store.account.borrow_mut().error = Some(error_key(&error));
                                false
                            }
                        };
                        let mut account = store.account.borrow_mut();
                        if success {
                            account.error = None;
                        }
                        account.schedule.finish(elapsed_ms(), success);
                    });
                    *active.active_task.borrow_mut() = Some(task);
                }
                let next = view(&store);
                if *status.peek() != next {
                    status.set(next);
                }
                sleep(Duration::from_secs(1)).await;
            }
        }
    });
}

async fn attempt(
    store: &AccountStore,
    mut session: Session,
    mut checkpoint: Checkpoint,
    generation: u64,
) -> Result<Checkpoint, SyncError> {
    let auth = SupabaseAuth::new(config::SUPABASE_URL, config::SUPABASE_KEY)?;
    // Retry saving a rotated token if the previous attempt hit a storage error.
    store
        .write_session(&session)
        .map_err(|_| SyncError::Backend("account storage failed".into()))?;
    if session.needs_refresh(crate::platform::change_time() / 1000, false) {
        session = auth
            .refresh(&session, crate::platform::change_time() / 1000)
            .await?;
        save_refresh(store, &session, generation)?;
    }
    if store.account.borrow().generation != generation {
        return Err(SyncError::Invalid("account changed".into()));
    }
    let mut backend = SupabaseBackend::new(
        config::SUPABASE_URL,
        config::SUPABASE_KEY,
        &session.access_token,
    )?;
    let mut records = werdy_store::from_json(&checkpoint.records)
        .map_err(|_| SyncError::Invalid("invalid sync checkpoint".into()))?;
    let mut synced = checkpoint
        .engine
        .sync(&mut backend, &mut records, crate::platform::change_time)
        .await;
    if matches!(synced, Err(SyncError::Http(401))) {
        session = auth
            .refresh(&session, crate::platform::change_time() / 1000)
            .await?;
        save_refresh(store, &session, generation)?;
        backend.set_access_token(&session.access_token)?;
        synced = checkpoint
            .engine
            .sync(&mut backend, &mut records, crate::platform::change_time)
            .await;
    }
    let offset = synced?;
    for error in checkpoint.engine.local_only_errors() {
        warn!("Sync keeps a legacy row local: {error}");
    }
    if offset.milliseconds.abs() > 120_000 {
        warn!("Sync clock offset is {} ms", offset.milliseconds);
    }
    checkpoint.records = werdy_store::to_json(&records)
        .map_err(|_| SyncError::Invalid("invalid synced records".into()))?;
    Ok(checkpoint)
}

pub(crate) fn save_refresh(
    store: &AccountStore,
    session: &Session,
    generation: u64,
) -> Result<(), SyncError> {
    if store.account.borrow().generation != generation {
        return Err(SyncError::Invalid("account changed".into()));
    }
    // Keep a rotated refresh token in memory even if saving it fails.
    store.account.borrow_mut().session = Some(session.clone());
    store
        .write_session(session)
        .map_err(|_| SyncError::Backend("account storage failed".into()))
}

pub fn error_key(error: &SyncError) -> &'static str {
    match error {
        SyncError::Http(400 | 401 | 403) => "account-session-error",
        _ => "account-sync-error",
    }
}

pub fn elapsed_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.performance())
            .map_or(0, |clock| clock.now() as u64)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        u64::try_from(
            START
                .get_or_init(std::time::Instant::now)
                .elapsed()
                .as_millis(),
        )
        .unwrap_or(u64::MAX)
    }
}

pub async fn sleep(duration: Duration) {
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::TimeoutFuture::new(duration.as_millis().min(u128::from(u32::MAX)) as u32)
        .await;
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
}

pub fn display_time(milliseconds: u64) -> Option<String> {
    let seconds = milliseconds / 1000;
    let day = werdy_core::WerdyDay::from_unix_days(i64::try_from(seconds / 86400).ok()?).ok()?;
    let (year, month, date) = day.date_parts();
    Some(format!(
        "{year:04}-{month:02}-{date:02} {:02}:{:02} UTC",
        seconds / 3600 % 24,
        seconds / 60 % 60
    ))
}
