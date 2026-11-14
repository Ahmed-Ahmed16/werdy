//! Numeric app versions shared by storage and release notices.

/// Three numeric parts, ordered from major to patch.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AppVersion(u32, u32, u32);

impl AppVersion {
    /// Accepts only three nonempty ASCII decimal parts that fit in u32.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let mut next = || {
            let part = parts.next()?;
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            part.parse::<u32>().ok()
        };
        let version = Self(next()?, next()?, next()?);
        parts.next().is_none().then_some(version)
    }
}

/// Keeps the greater valid version. An invalid value loses to a valid one.
pub fn greater_app_version(stored: Option<&str>, local: Option<&str>) -> Option<String> {
    let first = stored.and_then(AppVersion::parse);
    let second = local.and_then(AppVersion::parse);
    if second > first {
        local
    } else {
        stored.or(local)
    }
    .map(str::to_owned)
}
