//! `apk inspect`: what is inside a built APK, and would a phone take it.
//! Reads the output of the SDK's `aapt2` and `apksigner`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::process::{capture, capture_ok};

/// The phone CPU. Without it phones refuse the APK as "app not supported" (#84).
const PHONE_ABI: &str = "arm64-v8a";
const DEBUG_SIGNER: &str = "CN=Android Debug";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Badging {
    pub package: Option<String>,
    pub version_name: Option<String>,
    pub version_code: Option<String>,
    pub min_sdk: Option<String>,
    pub target_sdk: Option<String>,
    pub abis: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Signing {
    pub verified: bool,
    pub schemes: Vec<String>,
    pub signer: Option<String>,
    pub sha256: Option<String>,
}

/// Reads `aapt2 dump badging` output.
pub fn parse_badging(text: &str) -> Badging {
    let mut badging = Badging::default();
    for line in text.lines() {
        if line.starts_with("package:") {
            badging.package = attribute(line, "name");
            badging.version_name = attribute(line, "versionName");
            badging.version_code = attribute(line, "versionCode");
        } else if let Some(value) = line.strip_prefix("minSdkVersion:") {
            badging.min_sdk = Some(value.trim_matches('\'').to_string());
        } else if let Some(value) = line.strip_prefix("targetSdkVersion:") {
            badging.target_sdk = Some(value.trim_matches('\'').to_string());
        } else if let Some(values) = line.strip_prefix("native-code:") {
            badging.abis = quoted_values(values);
        }
    }
    badging
}

/// Reads `apksigner verify --print-certs -v` output.
pub fn parse_signing(text: &str) -> Signing {
    let mut signing = Signing::default();
    for line in text.lines().map(str::trim) {
        if line == "Verifies" {
            signing.verified = true;
        } else if let Some(rest) = line.strip_prefix("Verified using ")
            && rest.ends_with(": true")
            && let Some((scheme, _)) = rest.split_once(" scheme")
        {
            signing.schemes.push(scheme.to_string());
        } else if let Some(value) = line.strip_prefix("Signer #1 certificate DN: ") {
            signing.signer = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("Signer #1 certificate SHA-256 digest: ") {
            signing.sha256 = Some(value.to_string());
        }
    }
    signing
}

/// Reasons not to ship this APK. Empty means none were found.
pub fn problems(badging: &Badging, signing: &Signing) -> Vec<String> {
    let mut problems = Vec::new();
    if !badging.abis.iter().any(|abi| abi == PHONE_ABI) {
        problems.push(format!(
            "no {PHONE_ABI} code, so phones refuse it as \"app not supported\" (#84)"
        ));
    }
    if !signing.verified {
        problems.push("the signature does not verify".to_string());
    }
    if signing
        .signer
        .as_deref()
        .is_some_and(|signer| signer.contains(DEBUG_SIGNER))
    {
        problems.push("signed with the debug key, not the release key".to_string());
    }
    problems
}

/// Picks the newest build tools folder name, such as `36.0.0`. Names that are
/// not plain version numbers (previews) are skipped.
pub fn newest_version(names: &[String]) -> Option<&String> {
    names
        .iter()
        .filter_map(|name| version_key(name).map(|key| (key, name)))
        .max()
        .map(|(_, name)| name)
}

pub fn inspect(path: &str) -> Result<(), String> {
    if !Path::new(path).is_file() {
        return Err(format!("no APK at {path}"));
    }
    let tools = build_tools_dir()?;
    let aapt2 = tools.join(if cfg!(windows) { "aapt2.exe" } else { "aapt2" });
    let apksigner = tools.join(if cfg!(windows) {
        "apksigner.bat"
    } else {
        "apksigner"
    });

    let badging = parse_badging(&capture_ok(&aapt2, ["dump", "badging", path])?);
    // apksigner exits with an error for unsigned APKs; its output still says why.
    let verify = capture(&apksigner, ["verify", "--print-certs", "-v", path])?;
    let signing = parse_signing(&format!("{}\n{}", verify.stdout, verify.stderr));

    print!("{}", report(path, &badging, &signing));
    let problems = problems(&badging, &signing);
    for problem in &problems {
        println!("PROBLEM: {problem}");
    }
    if problems.is_empty() {
        println!("No problems found.");
        Ok(())
    } else {
        Err(format!("{} problem(s) found in {path}", problems.len()))
    }
}

fn report(path: &str, badging: &Badging, signing: &Signing) -> String {
    let show = |value: &Option<String>| value.clone().unwrap_or_else(|| "unknown".to_string());
    let list = |values: &[String]| {
        if values.is_empty() {
            "none".to_string()
        } else {
            values.join(", ")
        }
    };
    format!(
        "APK:         {path}\n\
         Package:     {}\n\
         Version:     {} (code {})\n\
         SDK:         min {}, target {}\n\
         CPU types:   {}\n\
         Verified:    {} (schemes: {})\n\
         Signer:      {}\n\
         SHA-256:     {}\n",
        show(&badging.package),
        show(&badging.version_name),
        show(&badging.version_code),
        show(&badging.min_sdk),
        show(&badging.target_sdk),
        list(&badging.abis),
        if signing.verified { "yes" } else { "no" },
        list(&signing.schemes),
        show(&signing.signer),
        show(&signing.sha256),
    )
}

fn build_tools_dir() -> Result<PathBuf, String> {
    let sdk = env::var_os("ANDROID_HOME").ok_or("ANDROID_HOME is not set")?;
    let dir = Path::new(&sdk).join("build-tools");
    let names: Vec<String> = fs::read_dir(&dir)
        .map_err(|error| format!("cannot read {}: {error}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let newest = newest_version(&names)
        .ok_or_else(|| format!("no build tools found in {}", dir.display()))?;
    Ok(dir.join(newest))
}

/// The value of `key='value'` in an aapt2 line.
fn attribute(line: &str, key: &str) -> Option<String> {
    let start = line.find(&format!(" {key}='"))? + key.len() + 3;
    let length = line[start..].find('\'')?;
    Some(line[start..start + length].to_string())
}

/// `'a' 'b'` into `["a", "b"]`.
fn quoted_values(text: &str) -> Vec<String> {
    text.split('\'')
        .skip(1)
        .step_by(2)
        .map(String::from)
        .collect()
}

fn version_key(name: &str) -> Option<Vec<u32>> {
    name.split('.').map(|part| part.parse().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed real output from the v0.1.0 APK, which held only emulator code.
    const BADGING_X86_ONLY: &str = "\
package: name='com.ahmedhossam.werdy' versionCode='1' versionName='0.1.0' platformBuildVersionName='15' platformBuildVersionCode='35' compileSdkVersion='35' compileSdkVersionCodename='15'
minSdkVersion:'24'
targetSdkVersion:'34'
uses-permission: name='android.permission.INTERNET'
application-label:'werdy'
native-code: 'x86_64'
";

    const BADGING_PHONE: &str = "\
package: name='com.ahmedhossam.werdy' versionCode='2' versionName='0.1.1'
minSdkVersion:'24'
targetSdkVersion:'35'
native-code: 'arm64-v8a'
";

    // Real layout; the signer and digest are made up.
    const SIGNED_RELEASE: &str = "\
Verifies
Verified using v1 scheme (JAR signing): false
Verified using v2 scheme (APK Signature Scheme v2): true
Verified using v3 scheme (APK Signature Scheme v3): false
Number of signers: 1
Signer #1 certificate DN: CN=Test Owner, O=WERDY, C=EG
Signer #1 certificate SHA-256 digest: 00aa11bb
Signer #1 key algorithm: RSA
";

    const SIGNED_DEBUG: &str = "\
Verifies
Verified using v2 scheme (APK Signature Scheme v2): true
Signer #1 certificate DN: C=US, O=Android, CN=Android Debug
";

    const UNSIGNED: &str = "\
DOES NOT VERIFY
ERROR: Missing META-INF/MANIFEST.MF
";

    #[test]
    fn badging_gives_package_versions_sdks_and_cpu_types() {
        assert_eq!(
            parse_badging(BADGING_X86_ONLY),
            Badging {
                package: Some("com.ahmedhossam.werdy".to_string()),
                version_name: Some("0.1.0".to_string()),
                version_code: Some("1".to_string()),
                min_sdk: Some("24".to_string()),
                target_sdk: Some("34".to_string()),
                abis: vec!["x86_64".to_string()],
            }
        );
    }

    #[test]
    fn several_cpu_types_are_all_kept() {
        let badging = parse_badging("native-code: 'arm64-v8a' 'x86_64'\n");
        assert_eq!(badging.abis, ["arm64-v8a", "x86_64"]);
    }

    #[test]
    fn signing_gives_schemes_signer_and_digest() {
        assert_eq!(
            parse_signing(SIGNED_RELEASE),
            Signing {
                verified: true,
                schemes: vec!["v2".to_string()],
                signer: Some("CN=Test Owner, O=WERDY, C=EG".to_string()),
                sha256: Some("00aa11bb".to_string()),
            }
        );
    }

    #[test]
    fn signed_phone_apk_has_no_problems() {
        let problems = problems(
            &parse_badging(BADGING_PHONE),
            &parse_signing(SIGNED_RELEASE),
        );
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn emulator_only_apk_is_a_problem() {
        let problems = problems(
            &parse_badging(BADGING_X86_ONLY),
            &parse_signing(SIGNED_RELEASE),
        );
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("arm64-v8a"));
    }

    #[test]
    fn unsigned_and_debug_signed_apks_are_problems() {
        let phone = parse_badging(BADGING_PHONE);
        let unsigned = problems(&phone, &parse_signing(UNSIGNED));
        assert_eq!(unsigned, ["the signature does not verify"]);
        let debug = problems(&phone, &parse_signing(SIGNED_DEBUG));
        assert_eq!(debug, ["signed with the debug key, not the release key"]);
    }

    #[test]
    fn newest_build_tools_compares_numbers_not_text() {
        let names = ["9.0.0", "36.0.0", "34.0.0", "37.0.0-rc1"].map(String::from);
        assert_eq!(newest_version(&names).map(String::as_str), Some("36.0.0"));
        assert_eq!(newest_version(&[]), None);
    }
}
