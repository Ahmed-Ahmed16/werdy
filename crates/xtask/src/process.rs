//! Running outside programs: cargo, dx, gh and the Android build tools.

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Runs the steps in order and stops at the first one that fails.
pub fn run_all(steps: &[Step], dry_run: bool) -> Result<(), String> {
    let root = workspace_root();
    let total = steps.len();
    for (index, step) in steps.iter().enumerate() {
        let number = index + 1;
        println!("[{number}/{total}] {step}");
        if dry_run {
            continue;
        }
        let status = Command::new(step.program)
            .args(&step.args)
            .current_dir(root.join(step.dir))
            .status()
            .map_err(|error| format!("could not start {}: {error}", step.program))?;
        if !status.success() {
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
