//! Pure domain model and scoring for werdy. Domain words live in CONTEXT.md.

#![forbid(unsafe_code)]

pub fn workspace_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::workspace_ready;

    #[test]
    fn workspace_is_ready() {
        assert!(workspace_ready());
    }
}
