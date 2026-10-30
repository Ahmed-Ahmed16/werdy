//! Release checks for the signed build in `.github/workflows/release.yml`.

use std::fs;

/// The app version from `crates/werdy-app/Cargo.toml`, the first
/// `version = "..."` line, which belongs to `[package]`.
pub fn app_version(cargo_toml: &str) -> Option<&str> {
    cargo_toml
        .lines()
        .find_map(|line| line.trim().strip_prefix("version = \"")?.strip_suffix('"'))
}

/// A release tag must name the version the APK will carry, so the GitHub
/// release, the file name and the version inside the app always agree.
pub fn check_tag(tag: &str, cargo_toml: &str) -> Result<String, String> {
    let version =
        app_version(cargo_toml).ok_or("crates/werdy-app/Cargo.toml has no version line")?;
    if tag.strip_prefix('v') == Some(version) {
        Ok(version.to_string())
    } else {
        Err(format!(
            "tag {tag} does not match the app version {version}; tag v{version} or raise the version first"
        ))
    }
}

pub fn run_check_tag(tag: &str) -> Result<(), String> {
    let path = "crates/werdy-app/Cargo.toml";
    let text = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let version = check_tag(tag, &text)?;
    println!("{version}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARGO: &str = "[package]\nname = \"werdy-app\"\nversion = \"0.1.2\"\nedition = \"2024\"\n\n[dependencies]\nserde = { version = \"1\" }\n";

    #[test]
    fn matching_tag_gives_the_version() {
        assert_eq!(check_tag("v0.1.2", CARGO), Ok("0.1.2".to_string()));
    }

    #[test]
    fn tag_for_another_version_is_refused() {
        let error = check_tag("v0.1.3", CARGO).unwrap_err();
        assert!(error.contains("0.1.2") && error.contains("v0.1.3"));
        assert!(check_tag("0.1.2", CARGO).is_err());
    }

    #[test]
    fn missing_version_is_an_error() {
        assert!(check_tag("v0.1.2", "[package]\nname = \"werdy-app\"\n").is_err());
    }
}
