//! Release notes selected without storage or UI work.

use serde::Deserialize;
pub use werdy_store::AppVersion as Version;

pub const MAX_VERSIONS: usize = 3;
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ReleaseNotes {
    pub version: String,
    pub notes: Vec<String>,
}

#[derive(Deserialize)]
struct ReleaseData {
    releases: Vec<ReleaseNotes>,
}

pub fn load() -> Result<Vec<ReleaseNotes>, String> {
    parse(include_str!("../../../data/whats-new.json"))
}

fn parse(text: &str) -> Result<Vec<ReleaseNotes>, String> {
    let data: ReleaseData = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let mut versions = std::collections::BTreeSet::new();
    for release in &data.releases {
        let version = Version::parse(&release.version).ok_or("invalid release version")?;
        if !versions.insert(version) {
            return Err("duplicate release version".into());
        }
    }
    Ok(data.releases)
}

pub fn notes_to_show(
    releases: &[ReleaseNotes],
    seen: Option<&str>,
    current: &str,
    max_versions: usize,
) -> Vec<ReleaseNotes> {
    let Some(current) = Version::parse(current) else {
        return Vec::new();
    };
    let seen = seen.and_then(Version::parse);
    let mut selected: Vec<_> = releases
        .iter()
        .filter_map(|release| {
            let version = Version::parse(&release.version)?;
            (version <= current
                && seen.is_none_or(|seen| version > seen)
                && !release.notes.is_empty())
            .then_some((version, release))
        })
        .collect();
    selected.sort_by_key(|(version, _)| std::cmp::Reverse(*version));
    selected
        .into_iter()
        .take(max_versions)
        .map(|(_, release)| release.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn releases() -> Vec<ReleaseNotes> {
        ["0.11.0", "0.10.0", "0.9.3", "0.8.0", "0.7.0"]
            .into_iter()
            .map(|version| ReleaseNotes {
                version: version.into(),
                notes: vec!["note".into()],
            })
            .collect()
    }

    #[test]
    fn versions_parse_strictly_and_order_numerically() {
        assert!(Version::parse("0.10.0") > Version::parse("0.9.3"));
        for bad in [
            "1.2",
            "1.2.3.4",
            "a.b.c",
            "",
            " 1.2.3",
            "+1.2.3",
            "4294967296.0.0",
        ] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn missed_notes_obey_seen_current_and_cap() {
        let releases = releases();
        for (seen, current, cap, expected) in [
            (None, "0.10.0", 3, vec!["0.10.0", "0.9.3", "0.8.0"]),
            (Some("0.9.3"), "0.10.0", 3, vec!["0.10.0"]),
            (
                Some("0.7.0"),
                "0.11.0",
                3,
                vec!["0.11.0", "0.10.0", "0.9.3"],
            ),
            (Some("0.10.0"), "0.10.0", 3, vec![]),
            (Some("1.0.0"), "0.10.0", 3, vec![]),
            (Some("bad"), "0.10.0", 3, vec!["0.10.0", "0.9.3", "0.8.0"]),
            (None, "bad", 3, vec![]),
            (None, "0.10.0", 0, vec![]),
        ] {
            let shown = notes_to_show(&releases, seen, current, cap);
            assert_eq!(
                shown.iter().map(|r| r.version.as_str()).collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn empty_notes_do_not_consume_cap_and_input_is_sorted() {
        let mut releases = releases();
        releases[0].notes.clear();
        releases.reverse();
        let shown = notes_to_show(&releases, None, "0.11.0", 1);
        assert_eq!(shown[0].version, "0.10.0");
        assert!(notes_to_show(&releases[4..], None, "0.11.0", 3).is_empty());
    }

    #[test]
    fn bundled_releases_are_valid_ordered_translated_and_cover_current_version()
    -> Result<(), Box<dyn std::error::Error>> {
        let releases = load()?;
        let texts = crate::i18n::Texts::load();
        let mut previous = None;
        for release in &releases {
            let version = Version::parse(&release.version).ok_or("bad version")?;
            assert!(previous.is_none_or(|p| p > version));
            previous = Some(version);
            for key in &release.notes {
                for language in [
                    crate::i18n::Language::Arabic,
                    crate::i18n::Language::English,
                ] {
                    assert_ne!(texts.get(language, key, &[]), *key);
                }
            }
        }
        assert!(
            releases
                .iter()
                .any(|r| r.version == env!("CARGO_PKG_VERSION"))
        );
        Ok(())
    }

    #[test]
    fn malformed_release_data_returns_error() {
        for text in [
            "bad",
            r#"{"releases":[{"version":"bad","notes":[]}]}"#,
            r#"{"releases":[{"version":"1.0.0","notes":[]},{"version":"1.0.0","notes":[]}]}"#,
        ] {
            assert!(parse(text).is_err());
        }
    }
}
