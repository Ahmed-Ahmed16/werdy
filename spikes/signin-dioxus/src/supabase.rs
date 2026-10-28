//! Gate 3: trade the Google ID token for a Supabase session.

use crate::Credentials;
use crate::config::{SUPABASE_PUBLISHABLE_KEY, SUPABASE_URL};
use serde::{Deserialize, Serialize};

/// What the app keeps after a successful sign in. No Debug, so tokens never reach logs.
pub struct Session {
    pub user_id: String,
    pub email: Option<String>,
    pub access_token: String,
}

/// JSON body Supabase expects for `grant_type=id_token`.
#[derive(Serialize)]
struct IdTokenGrant<'a> {
    provider: &'a str,
    id_token: &'a str,
    nonce: &'a str,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    user: User,
}

#[derive(Deserialize)]
struct User {
    id: String,
    email: Option<String>,
}

/// Supabase checks Google's signature, the audience, and that sha256(raw nonce)
/// matches the nonce inside the token, then finds or creates the user.
pub async fn exchange(credentials: &Credentials) -> Result<Session, String> {
    let url = format!("{SUPABASE_URL}/auth/v1/token?grant_type=id_token");
    let body = IdTokenGrant {
        provider: "google",
        id_token: &credentials.id_token,
        // The raw nonce: Google got the hash, Supabase hashes this to compare.
        nonce: &credentials.raw_nonce,
    };
    let response = reqwest::Client::new()
        .post(url)
        .header("apikey", SUPABASE_PUBLISHABLE_KEY)
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("Could not reach Supabase: {error}"))?;
    // Read the status first: text() takes ownership of the response.
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("Could not read Supabase reply: {error}"))?;
    if !status.is_success() {
        // Error replies carry a code and message, not tokens, so showing them is safe.
        return Err(format!("Supabase answered {status}: {text}"));
    }
    session_from_json(&text)
}

pub fn session_from_json(body: &str) -> Result<Session, String> {
    // Do not echo the body in errors: it holds tokens.
    let parsed: TokenResponse =
        serde_json::from_str(body).map_err(|_| "Unexpected Supabase response".to_string())?;
    Ok(Session {
        user_id: parsed.user.id,
        email: parsed.user.email,
        access_token: parsed.access_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_reads_user_and_token() -> Result<(), String> {
        let body = r#"{"access_token":"jwt","token_type":"bearer","user":{"id":"u1","email":"a@b.c"}}"#;
        let session = session_from_json(body)?;
        assert_eq!(session.user_id, "u1");
        assert_eq!(session.email.as_deref(), Some("a@b.c"));
        assert_eq!(session.access_token, "jwt");
        Ok(())
    }

    #[test]
    fn bad_body_is_an_error_without_secrets() {
        let Err(message) = session_from_json(r#"{"access_token":"secret"}"#) else {
            panic!("missing user accepted");
        };
        assert!(!message.contains("secret"));
    }
}
