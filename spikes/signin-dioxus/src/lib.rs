pub mod config;
pub mod nonce;
pub mod supabase;
pub mod token;

use serde::Deserialize;

// Keep secrets out of Debug and UI formatting. Gate 3 will consume these fields.
pub struct Credentials {
    pub id_token: String,
    pub raw_nonce: String,
}

#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Reply {
    Pending,
    Ok {
        #[serde(rename = "idToken")]
        id_token: String,
    },
    Error {
        kind: String,
        message: String,
    },
}

pub fn reply(json: &str) -> Result<Reply, String> {
    // Do not include the JSON in errors: a malformed response might contain a token.
    serde_json::from_str(json).map_err(|_| "Invalid credential bridge response".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_replies_preserve_success_and_failure() -> Result<(), String> {
        assert!(matches!(reply(r#"{"state":"pending"}"#)?, Reply::Pending));
        assert!(matches!(reply(r#"{"state":"ok","idToken":"secret"}"#)?,
            Reply::Ok { id_token } if id_token == "secret"));
        assert!(
            matches!(reply(r#"{"state":"error","kind":"cancel","message":"Closed"}"#)?,
            Reply::Error { kind, message } if kind == "cancel" && message == "Closed")
        );
        for invalid in [
            "{}",
            r#"{"state":"unknown"}"#,
            r#"{"state":"ok"}"#,
            "secret",
        ] {
            let Err(message) = reply(invalid) else {
                return Err("Invalid reply accepted".into());
            };
            assert!(!message.contains("secret"));
        }
        Ok(())
    }
}
