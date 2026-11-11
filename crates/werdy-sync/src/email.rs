use crate::{SyncError, invalid};

/// Match Google email addresses without applying alias rules.
pub fn normalize_email(input: &str) -> Result<String, SyncError> {
    let email = input.trim().to_lowercase();
    let mut parts = email.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    if !(3..=254).contains(&email.chars().count())
        || local.is_empty()
        || domain.is_empty()
        || parts.next().is_some()
        || email.chars().any(char::is_whitespace)
    {
        return Err(invalid("invalid email"));
    }
    Ok(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_normalization_checks_shape_and_length() {
        for input in ["a@b", "A@B", "  A@B  "] {
            assert_eq!(normalize_email(input).unwrap(), "a@b");
        }
        for input in ["", "ab", "@b", "a@", "a@@b", "a @b", "a@b\nc"] {
            assert!(normalize_email(input).is_err(), "{input:?}");
        }
        assert!(normalize_email(&format!("{}@b", "a".repeat(252))).is_ok());
        assert!(normalize_email(&format!("{}@b", "a".repeat(253))).is_err());
    }
}
