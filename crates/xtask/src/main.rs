//! werdy's shared task runner. Agents and people run `cargo xtask <job>` for
//! recurring jobs instead of writing one off scripts.
//! The skill in `.claude/skills/werdy-cli/` tells agents when to use it.

mod apk;
mod cli;
mod gates;
mod pr;
mod process;
mod release;

use std::process::ExitCode;

use cli::Command;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Help => {
            print!("{}", cli::HELP);
            Ok(())
        }
        Command::Gates { android, dry_run } => gates::run(android, dry_run),
        Command::ApkInspect { path } => apk::inspect(&path),
        Command::PrComments { number, all, full } => pr::comments(number, all, full),
        Command::PrWaitReview {
            number,
            timeout_mins,
        } => pr::wait_review(number, timeout_mins),
        Command::ReleaseCheckTag { tag } => release::run_check_tag(&tag),
    }
}
