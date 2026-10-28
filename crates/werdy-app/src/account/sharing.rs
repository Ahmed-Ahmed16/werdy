//! Sharing requests use the account token, but never the owner's sync engine.
//! Account deletion uses the same authenticated request path.
use super::{AccountStore, config, runtime};
use werdy_sync::{ShareCode, SharePerson, SupabaseAuth, SupabaseBackend, SyncError, WatchView};

pub enum Request {
    Lists,
    Create,
    Redeem(String),
    Revoke(String),
    Leave(String),
    Read(String, Box<WatchView>),
    DeleteAccount,
}
pub enum Reply {
    Lists(Vec<SharePerson>, Vec<SharePerson>),
    Code(ShareCode),
    Changed,
    View(Box<WatchView>),
}

pub async fn execute(
    store: &AccountStore,
    request: &Request,
    generation: u64,
) -> Result<Reply, SyncError> {
    let mut session = store
        .account
        .borrow()
        .session
        .clone()
        .ok_or(SyncError::Http(401))?;
    store
        .write_session(&session)
        .map_err(|_| SyncError::Backend("account storage failed".into()))?;
    let auth = SupabaseAuth::new(config::SUPABASE_URL, config::SUPABASE_KEY)?;
    if session.needs_refresh(crate::platform::change_time() / 1000, false) {
        session = auth
            .refresh(&session, crate::platform::change_time() / 1000)
            .await?;
        runtime::save_refresh(store, &session, generation)?;
    }
    let mut backend = SupabaseBackend::new(
        config::SUPABASE_URL,
        config::SUPABASE_KEY,
        &session.access_token,
    )?;
    let result = perform(&backend, request).await;
    if !matches!(result, Err(SyncError::Http(401))) {
        return result;
    }
    session = auth
        .refresh(&session, crate::platform::change_time() / 1000)
        .await?;
    runtime::save_refresh(store, &session, generation)?;
    backend.set_access_token(&session.access_token)?;
    perform(&backend, request).await
}

async fn perform(backend: &SupabaseBackend, request: &Request) -> Result<Reply, SyncError> {
    match request {
        Request::Lists => Ok(Reply::Lists(
            backend.list_watchers().await?,
            backend.list_watched().await?,
        )),
        Request::Create => Ok(Reply::Code(backend.create_share_code().await?)),
        Request::Redeem(code) => {
            backend.redeem_share_code(code).await?;
            Ok(Reply::Changed)
        }
        Request::Revoke(id) => {
            backend.revoke_watcher(id).await?;
            Ok(Reply::Changed)
        }
        Request::Leave(id) => {
            backend.leave_share(id).await?;
            Ok(Reply::Changed)
        }
        Request::Read(id, view) => {
            let mut view = view.clone();
            // Pages keep incomplete deeds deferred. The UI is updated only after
            // the bounded stream is complete, and every page checks the share.
            loop {
                let page = backend.watch_pull(id, view.cursor(), 500).await?;
                let more = page.has_more;
                view.apply(page.batch())?;
                if !more {
                    break;
                }
            }
            Ok(Reply::View(view))
        }
        Request::DeleteAccount => {
            backend.delete_account().await?;
            Ok(Reply::Changed)
        }
    }
}
