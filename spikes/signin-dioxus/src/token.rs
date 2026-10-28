use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Claims {
    pub email: Option<String>,
    pub sub: String,
}

pub fn display_claims(token: &str) -> Result<Claims, String> {
    // Reject missing, empty and extra segments before indexing external input.
    let segments: Vec<_> = token.split('.').collect();
    if segments.len() != 3 || segments.iter().any(|segment| segment.is_empty()) {
        return Err("Invalid ID token shape".to_string());
    }
    let payload = URL_SAFE_NO_PAD
        .decode(segments[1])
        .map_err(|_| "Invalid ID token payload encoding".to_string())?;
    let claims: Claims =
        serde_json::from_slice(&payload).map_err(|_| "Invalid ID token claims".to_string())?;
    if claims.sub.is_empty() {
        return Err("ID token has empty sub".to_string());
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_is_displayed_without_signature_verification() -> Result<(), String> {
        let payload = URL_SAFE_NO_PAD.encode(r#"{"email":"a@example.com","sub":"123"}"#);
        let claims = display_claims(&format!("header.{payload}.signature"))?;
        assert_eq!(claims.email.as_deref(), Some("a@example.com"));
        assert_eq!(claims.sub, "123");
        Ok(())
    }

    #[test]
    fn malformed_tokens_return_safe_errors() {
        for token in [
            "",
            "a.b",
            "a.b.c.d",
            "a.!.c",
            "a.e30.c",
            "a..c",
            "a.bm90IGpzb24.c",
        ] {
            assert!(display_claims(token).is_err());
        }
    }

    #[test]
    fn absent_email_is_allowed_but_empty_subject_is_rejected() -> Result<(), String> {
        let payload = URL_SAFE_NO_PAD.encode(r#"{"sub":"123"}"#);
        assert!(display_claims(&format!("h.{payload}.s"))?.email.is_none());
        let payload = URL_SAFE_NO_PAD.encode(r#"{"sub":""}"#);
        assert!(display_claims(&format!("h.{payload}.s")).is_err());
        Ok(())
    }
}
