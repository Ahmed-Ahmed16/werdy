//! Supabase owns token verification. Neither provider replies nor credentials
//! are included in an error. Saved sessions contain secrets and stay local.

use super::{SupabaseBackend, check_status, parse_response, transport_error};
use crate::{SyncError, invalid};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    /// Expiry in Unix seconds, as issued by Supabase.
    pub expires_at: u64,
    pub user_id: String,
    pub email: Option<String>,
}

impl Session {
    /// Refresh two minutes early. A 401 also forces a refresh regardless of time.
    pub fn needs_refresh(&self, now_seconds: u64, unauthorized: bool) -> bool {
        unauthorized || now_seconds.saturating_add(120) >= self.expires_at
    }

    pub fn validate(&self) -> Result<(), SyncError> {
        let id = self.user_id.as_bytes();
        if id.len() != 36
            || id.iter().enumerate().any(|(index, byte)| {
                if matches!(index, 8 | 13 | 18 | 23) {
                    *byte != b'-'
                } else {
                    !byte.is_ascii_hexdigit()
                }
            })
            || self.access_token.is_empty()
            || self.refresh_token.is_empty()
            || self.access_token.len() > 16384
            || self.refresh_token.len() > 16384
            || self.email.as_ref().is_some_and(|email| email.len() > 320)
            || self.expires_at == 0
        {
            return Err(invalid("invalid saved session"));
        }
        super::authorization(&self.access_token)?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: Option<u64>,
    expires_in: Option<u64>,
    user: User,
}

#[derive(Deserialize)]
struct User {
    id: String,
    email: Option<String>,
}

pub struct SupabaseAuth {
    transport: SupabaseBackend,
}

impl SupabaseAuth {
    pub fn new(base_url: &str, publishable_key: &str) -> Result<Self, SyncError> {
        Ok(Self {
            transport: SupabaseBackend::new(base_url, publishable_key, "unused")?,
        })
    }

    /// Google receives the SHA 256 digest. Supabase receives the original nonce.
    pub async fn exchange(
        &self,
        id_token: &str,
        raw_nonce: &str,
        now_seconds: u64,
    ) -> Result<Session, SyncError> {
        if id_token.is_empty()
            || id_token.len() > 16384
            || raw_nonce.len() != 64
            || !raw_nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid("invalid Google credential"));
        }
        #[derive(Serialize)]
        struct Grant<'a> {
            provider: &'static str,
            id_token: &'a str,
            nonce: &'a str,
        }
        self.token(
            "id_token",
            &Grant {
                provider: "google",
                id_token,
                nonce: raw_nonce,
            },
            now_seconds,
        )
        .await
    }

    pub async fn refresh(&self, session: &Session, now_seconds: u64) -> Result<Session, SyncError> {
        session.validate()?;
        #[derive(Serialize)]
        struct Grant<'a> {
            refresh_token: &'a str,
        }
        let refreshed = self
            .token(
                "refresh_token",
                &Grant {
                    refresh_token: &session.refresh_token,
                },
                now_seconds,
            )
            .await?;
        if refreshed.user_id != session.user_id {
            return Err(invalid("refreshed session changed owner"));
        }
        Ok(refreshed)
    }

    async fn token(
        &self,
        grant: &str,
        body: &impl Serialize,
        now_seconds: u64,
    ) -> Result<Session, SyncError> {
        let mut url = self.transport.rpc_url.clone();
        url.set_path("/auth/v1/token");
        url.query_pairs_mut().append_pair("grant_type", grant);
        let response = self
            .transport
            .client
            .post(url)
            .header("apikey", self.transport.publishable_key.clone())
            .timeout(std::time::Duration::from_secs(30))
            .json(body)
            .send()
            .await
            .map_err(|_| transport_error())?;
        check_status(response.status())?;
        let body = response.bytes().await.map_err(|_| transport_error())?;
        session_from_json(&body, now_seconds)
    }
}

fn session_from_json(body: &[u8], now_seconds: u64) -> Result<Session, SyncError> {
    let response: TokenResponse = parse_response(body)?;
    let expires_at = match (response.expires_at, response.expires_in) {
        (Some(expiry), _) => expiry,
        (None, Some(seconds)) => now_seconds
            .checked_add(seconds)
            .ok_or_else(|| invalid("invalid session expiry"))?,
        _ => return Err(invalid("missing session expiry")),
    };
    let session = Session {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        expires_at,
        user_id: response.user.id,
        email: response.user.email,
    };
    session.validate()?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_and_refresh_replies_keep_rotated_secrets_and_checked_expiry() -> Result<(), SyncError>
    {
        for (expiry, expected) in [("\"expires_at\":4600", 4600), ("\"expires_in\":3600", 4600)] {
            let body = format!(
                r#"{{"access_token":"access","refresh_token":"rotated","{unused}":null,{expiry},"user":{{"id":"00000000-0000-4000-8000-000000000023","email":"a@example.test"}}}}"#,
                unused = "unused"
            );
            let session = session_from_json(body.as_bytes(), 1000)?;
            assert_eq!(session.refresh_token, "rotated");
            assert_eq!(session.expires_at, expected);
            assert!(!session.needs_refresh(4479, false));
            assert!(session.needs_refresh(4480, false));
            assert!(session.needs_refresh(1000, true));
            assert!(session.needs_refresh(u64::MAX, false));
        }
        Ok(())
    }
    #[test]
    fn malformed_auth_never_echoes_secrets() {
        for body in [
            r#"{"access_token":"secret"}"#,
            r#"{"access_token":"secret","refresh_token":"secret","expires_in":18446744073709551615,"user":{"id":"bad"}}"#,
        ] {
            assert!(
                session_from_json(body.as_bytes(), 1000)
                    .err()
                    .is_some_and(|error| !error.to_string().contains("secret"))
            );
        }
    }
}
