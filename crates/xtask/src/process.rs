//! Running outside programs: cargo, dx, gh and the Android build tools.

use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One command in a list of steps, run from a folder under the workspace root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub program: &'static str,
    pub args: Vec<&'static str>,
    pub dir: &'static str,
}

impl Step {
    pub fn new(program: &'static str, args: &[&'static str]) -> Self {
        Self {
            program,
            args: args.to_vec(),
            dir: "",
        }
    }

    pub fn cargo(args: &[&'static str]) -> Self {
        Self::new("cargo", args)
    }

    pub fn in_dir(self, dir: &'static str) -> Self {
        Self { dir, ..self }
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program)?;
        for arg in &self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// How one run of a step ended. `stderr` is what it printed there.
pub struct Outcome {
    pub success: bool,
    pub stderr: String,
}

/// Windows sometimes refuses a file that was only just written: antivirus or
/// Smart App Control is scanning it. The two codes are "Access is denied"
/// (5) and "used by another process" (32).
pub fn blocked_by_windows(stderr: &str) -> bool {
    stderr.contains("(os error 5)") || stderr.contains("(os error 32)")
}

/// Runs the steps in order and stops at the first one that fails.
pub fn run_all(steps: &[Step], dry_run: bool) -> Result<(), String> {
    let root = workspace_root();
    run_all_with(steps, dry_run, cfg!(windows), |step| run_step(step, &root))
}

/// `run_all` with the way a step is run passed in, so tests can fake it.
/// `windows` says whether to retry a step that Windows blocked.
fn run_all_with(
    steps: &[Step],
    dry_run: bool,
    windows: bool,
    mut run: impl FnMut(&Step) -> Result<Outcome, String>,
) -> Result<(), String> {
    let total = steps.len();
    for (index, step) in steps.iter().enumerate() {
        let number = index + 1;
        println!("[{number}/{total}] {step}");
        if dry_run {
            continue;
        }
        let mut outcome = run(step)?;
        if !outcome.success && windows && blocked_by_windows(&outcome.stderr) {
            println!(
                "Windows blocked a new file (os error 5 or 32). Antivirus or Smart App \
                 Control is probably scanning it. Retrying once."
            );
            outcome = run(step)?;
            if !outcome.success && blocked_by_windows(&outcome.stderr) {
                return Err(format!(
                    "step {number} of {total} failed twice because Windows blocked a new \
                     file: {step}. Run it again, or exclude this repo's target folder from \
                     the scanner."
                ));
            }
        }
        if !outcome.success {
            return Err(format!("step {number} of {total} failed: {step}"));
        }
    }
    if dry_run {
        println!("Dry run: nothing was run.");
    } else {
        println!("All {total} steps passed.");
    }
    Ok(())
}

/// Runs one step. Its stderr is passed on as it arrives and also kept.
fn run_step(step: &Step, root: &Path) -> Result<Outcome, String> {
    let mut command = Command::new(step.program);
    command
        .args(&step.args)
        .current_dir(root.join(step.dir))
        .stderr(Stdio::piped());
    // Cargo drops its colors when stderr is a pipe; keep them in a terminal.
    if io::stderr().is_terminal() && env::var_os("CARGO_TERM_COLOR").is_none() {
        command.env("CARGO_TERM_COLOR", "always");
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not start {}: {error}", step.program))?;
    let mut pipe = child
        .stderr
        .take()
        .ok_or_else(|| format!("could not read the output of {}", step.program))?;
    let mut kept = Vec::new();
    let mut buffer = [0u8; 4096];
    while let Ok(count) = pipe.read(&mut buffer) {
        if count == 0 {
            break;
        }
        // A closed stderr must not stop the run; the step still has to finish.
        let _ = io::stderr().write_all(&buffer[..count]);
        kept.extend_from_slice(&buffer[..count]);
    }
    let status = child
        .wait()
        .map_err(|error| format!("could not wait for {}: {error}", step.program))?;
    Ok(Outcome {
        success: status.success(),
        stderr: String::from_utf8_lossy(&kept).into_owned(),
    })
}

pub struct Captured {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a program and keeps what it printed, whether it succeeded or not.
pub fn capture<I, S>(program: impl AsRef<OsStr>, args: I) -> Result<Captured, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let program = program.as_ref();
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("could not start {}: {error}", program.to_string_lossy()))?;
    Ok(Captured {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Like `capture`, but a failed run is an error that carries its stderr.
pub fn capture_ok<I, S>(program: impl AsRef<OsStr>, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let name = program.as_ref().to_string_lossy().into_owned();
    let captured = capture(program, args)?;
    if captured.success {
        Ok(captured.stdout)
    } else {
        Err(format!("{name} failed: {}", captured.stderr.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const DENIED: &str = "error: could not execute process `x.exe`\n\nCaused by:\n  \
        Access is denied. (os error 5)";
    const IN_USE: &str = "The process cannot access the file because it is being used by \
        another process. (os error 32)";

    fn steps() -> Vec<Step> {
        vec![Step::cargo(&["test"]), Step::cargo(&["fmt"])]
    }

    fn outcome(success: bool, stderr: &str) -> Result<Outcome, String> {
        Ok(Outcome {
            success,
            stderr: stderr.to_string(),
        })
    }

    #[test]
    fn both_windows_block_codes_are_recognised() {
        assert!(blocked_by_windows(DENIED));
        assert!(blocked_by_windows(IN_USE));
        assert!(!blocked_by_windows("error[E0308]: mismatched types"));
        assert!(!blocked_by_windows("os error 50"));
    }

    #[test]
    fn a_blocked_step_is_retried_once_and_can_pass() {
        let runs = Cell::new(0);
        let result = run_all_with(&steps(), false, true, |_| {
            runs.set(runs.get() + 1);
            // The first run of the first step is blocked; everything else passes.
            if runs.get() == 1 {
                outcome(false, DENIED)
            } else {
                outcome(true, "")
            }
        });
        assert_eq!(result, Ok(()));
        assert_eq!(runs.get(), 3, "one retry plus two steps");
    }

    #[test]
    fn a_step_blocked_twice_fails_and_says_why() {
        let runs = Cell::new(0);
        let error = run_all_with(&steps(), false, true, |_| {
            runs.set(runs.get() + 1);
            outcome(false, IN_USE)
        })
        .unwrap_err();
        assert_eq!(runs.get(), 2, "retried once, not forever");
        assert!(error.contains("step 1 of 2"));
        assert!(error.contains("Windows blocked"));
    }

    #[test]
    fn a_retry_that_fails_for_another_reason_is_a_plain_failure() {
        let runs = Cell::new(0);
        let error = run_all_with(&steps(), false, true, |_| {
            runs.set(runs.get() + 1);
            if runs.get() == 1 {
                outcome(false, DENIED)
            } else {
                outcome(false, "error: test failed")
            }
        })
        .unwrap_err();
        assert!(!error.contains("Windows blocked"));
        assert!(error.contains("step 1 of 2 failed"));
    }

    #[test]
    fn other_failures_are_not_retried() {
        let runs = Cell::new(0);
        let result = run_all_with(&steps(), false, true, |_| {
            runs.set(runs.get() + 1);
            outcome(false, "error: test failed")
        });
        assert!(result.is_err());
        assert_eq!(runs.get(), 1);
    }

    #[test]
    fn off_windows_nothing_is_retried() {
        let runs = Cell::new(0);
        let result = run_all_with(&steps(), false, false, |_| {
            runs.set(runs.get() + 1);
            outcome(false, DENIED)
        });
        assert!(result.is_err());
        assert_eq!(runs.get(), 1);
    }

    #[test]
    fn a_dry_run_runs_nothing() {
        let result = run_all_with(&steps(), true, true, |_| panic!("must not run"));
        assert_eq!(result, Ok(()));
    }
}
