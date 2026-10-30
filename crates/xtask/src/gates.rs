//! The CI gates, run locally.

use crate::process::{self, Step};

/// The gates in the order CI runs them. A test checks that each one still
/// appears in `.github/workflows/ci.yml`, so the two cannot drift apart.
/// The headless Chrome tests are left out: they need chromedriver.
pub fn steps(android: bool) -> Vec<Step> {
    let mut steps = vec![
        Step::cargo(&["fmt", "--all", "--", "--check"]),
        Step::cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
        Step::cargo(&["test", "--workspace"]),
        Step::cargo(&[
            "clippy",
            "-p",
            "werdy-app",
            "--target",
            "wasm32-unknown-unknown",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
    ];
    if android {
        steps.push(Step::cargo(&[
            "clippy",
            "-p",
            "werdy-app",
            "--target",
            "x86_64-linux-android",
            "--no-default-features",
            "--features",
            "mobile",
            "--",
            "-D",
            "warnings",
        ]));
        steps.push(
            Step::new(
                "dx",
                &[
                    "build",
                    "--android",
                    "--no-default-features",
                    "--features",
                    "mobile",
                    "--target",
                    "aarch64-linux-android",
                ],
            )
            .in_dir("crates/werdy-app"),
        );
    }
    steps
}

pub fn run(android: bool, dry_run: bool) -> Result<(), String> {
    process::run_all(&steps(android), dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CI: &str = include_str!("../../../.github/workflows/ci.yml");

    #[test]
    fn every_gate_is_a_command_ci_runs() {
        for step in steps(true) {
            let command = step.to_string();
            assert!(CI.contains(&command), "not in ci.yml: {command}");
        }
    }

    #[test]
    fn android_gates_run_only_when_asked() {
        let mentions_android = |step: &Step| step.to_string().contains("android");
        assert!(!steps(false).iter().any(mentions_android));
        assert_eq!(
            steps(true).iter().filter(|s| mentions_android(s)).count(),
            2
        );
    }
}
