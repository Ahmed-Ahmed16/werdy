//! PostgREST RPC transport. Credentials stay in memory and are never formatted
//! into errors. Token refresh is the caller's job.

use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderValue};
use reqwest::{Client, Request, StatusCode, Url};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Backend, Batch, Pending, SyncError, invalid, rows};

pub(crate) mod auth;
pub(crate) mod sharing;

/// Client for the sync and sharing RPCs in `supabase/migrations`.
/// Uses only a public publishable key and the owner's current access token.
/// No Debug or Serialize implementation can expose these credentials.
pub struct SupabaseBackend {
    client: Client,
    rpc_url: Url,
    publishable_key: HeaderValue,
    authorization: HeaderValue,
}

#[derive(Serialize)]
struct PushRequest<'a> {
    batch: &'a [Pending],
}

#[derive(Serialize)]
struct PullRequest {
    after_cursor: u64,
}

#[derive(Serialize)]
struct TimeRequest {}

impl SupabaseBackend {
    /// Accepts an HTTPS project origin. HTTP is allowed only for loopback tests.
    /// Credentials, paths, queries and fragments in the base URL are rejected.
    pub fn new(
        base_url: &str,
        publishable_key: &str,
        access_token: &str,
    ) -> Result<Self, SyncError> {
        let mut rpc_url = Url::parse(base_url).map_err(|_| invalid("invalid Supabase URL"))?;
        let loopback = matches!(
            rpc_url.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]")
        );
        if !(rpc_url.scheme() == "https" || rpc_url.scheme() == "http" && loopback)
            || rpc_url.host_str().is_none()
            || !rpc_url.username().is_empty()
            || rpc_url.password().is_some()
            || !matches!(rpc_url.path(), "" | "/")
            || rpc_url.query().is_some()
            || rpc_url.fragment().is_some()
        {
            return Err(invalid("Supabase URL must be a secure project origin"));
        }
        if !publishable_key.starts_with("sb_publishable_") || publishable_key.len() > 1_024 {
            return Err(invalid("Supabase requires a public publishable key"));
        }
        rpc_url.set_path("/rest/v1/rpc/");
        let builder = Client::builder();
        // Native redirects could forward the apikey header to another host.
        // Browser fetch controls redirects itself. The caller must supply the
        // trusted project origin directly.
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.redirect(reqwest::redirect::Policy::none());
        Ok(Self {
            client: builder.build().map_err(|_| transport_error())?,
            rpc_url,
            publishable_key: secret_header(publishable_key)?,
            authorization: authorization(access_token)?,
        })
    }

    /// Replaces the JWT after refresh. Invalid input leaves the old token intact.
    pub fn set_access_token(&mut self, access_token: &str) -> Result<(), SyncError> {
        self.authorization = authorization(access_token)?;
        Ok(())
    }

    fn request(&self, function: &str, arguments: &impl Serialize) -> Result<Request, SyncError> {
        let url = self
            .rpc_url
            .join(function)
            .map_err(|_| invalid("invalid RPC path"))?;
        let builder = self
            .client
            .post(url)
            .header("apikey", self.publishable_key.clone())
            .header(AUTHORIZATION, self.authorization.clone())
            .header(ACCEPT, "application/json")
            .timeout(Duration::from_secs(30))
            .json(arguments);
        builder.build().map_err(|_| transport_error())
    }

    async fn rpc<T: DeserializeOwned>(
        &self,
        function: &str,
        arguments: &(impl Serialize + Sync),
    ) -> Result<T, SyncError> {
        let response = self
            .client
            .execute(self.request(function, arguments)?)
            .await
            .map_err(|_| transport_error())?;
        check_status(response.status())?;
        let body = response.bytes().await.map_err(|_| transport_error())?;
        parse_response(&body)
    }
}

impl Backend for SupabaseBackend {
    async fn push(&mut self, changes: &[Pending]) -> Result<Batch, SyncError> {
        if changes.len() > 1_024 {
            return Err(invalid("sync batch exceeds 1024 operations"));
        }
        for pending in changes {
            rows::validate_pending(pending)?;
        }
        self.rpc("werdy_sync_push", &PushRequest { batch: changes })
            .await
    }

    async fn pull(&mut self, cursor: u64) -> Result<Batch, SyncError> {
        self.rpc(
            "werdy_sync_pull",
            &PullRequest {
                after_cursor: cursor,
            },
        )
        .await
    }

    async fn server_time(&mut self) -> Result<u64, SyncError> {
        self.rpc("werdy_sync_server_time", &TimeRequest {}).await
    }
}

fn secret_header(secret: &str) -> Result<HeaderValue, SyncError> {
    let mut header =
        HeaderValue::from_str(secret).map_err(|_| invalid("invalid credential header"))?;
    header.set_sensitive(true);
    Ok(header)
}

fn authorization(token: &str) -> Result<HeaderValue, SyncError> {
    if token.is_empty()
        || token.len() > 16_384
        || token.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(invalid("invalid access token"));
    }
    secret_header(&format!("Bearer {token}"))
}

fn transport_error() -> SyncError {
    SyncError::Backend("Supabase transport failed".to_owned())
}

fn check_status(status: StatusCode) -> Result<(), SyncError> {
    if !status.is_success() {
        return Err(SyncError::Http(status.as_u16()));
    }
    Ok(())
}

fn parse_response<T: DeserializeOwned>(body: &[u8]) -> Result<T, SyncError> {
    serde_json::from_slice(body).map_err(|_| invalid("invalid Supabase RPC response"))
}

#[cfg(test)]
mod tests;
