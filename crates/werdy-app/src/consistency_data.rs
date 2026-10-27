//! Owner notice choices. Text and verified sources are supplied by the owner.

use serde::Deserialize;

#[derive(Deserialize)]
pub struct NoticeData {
    pub welcome_gap_days: usize,
    pub milestones: Vec<usize>,
    pub sayings: Vec<Saying>,
}

#[derive(Deserialize)]
pub struct Saying {
    pub text_ar: String,
    pub text_en: String,
    pub source: String,
    pub verified_by: String,
}

impl Saying {
    pub fn is_verified(&self) -> bool {
        !self.text_ar.trim().is_empty()
            && !self.text_en.trim().is_empty()
            && !self.source.trim().is_empty()
            && !self.verified_by.trim().is_empty()
    }
}

pub fn notices() -> Result<NoticeData, serde_json::Error> {
    serde_json::from_str(include_str!("../../../data/consistency.json"))
}

#[cfg(test)]
mod tests {
    use super::{Saying, notices};

    #[test]
    fn every_saying_has_text_source_and_verifier() -> Result<(), serde_json::Error> {
        let data = notices()?;
        assert!(data.sayings.iter().all(Saying::is_verified));
        Ok(())
    }

    #[test]
    fn blank_source_or_verifier_is_rejected() {
        let text = |source: &str, verified_by: &str| Saying {
            text_ar: "text".to_owned(),
            text_en: "text".to_owned(),
            source: source.to_owned(),
            verified_by: verified_by.to_owned(),
        };
        assert!(!text("", "owner").is_verified());
        assert!(!text("source", "").is_verified());
    }
}
