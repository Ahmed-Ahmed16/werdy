//! Turns the words after `cargo xtask` into a command. No side effects.

pub const DEFAULT_TIMEOUT_MINS: u64 = 20;

pub const HELP: &str = "\
cargo xtask <job>

  gates [--android] [--dry-run]
      Run the CI gates locally: fmt, clippy, tests, wasm clippy.
      --android also runs the Android clippy and the debug APK build.
      --dry-run prints the commands without running them.

  apk inspect <file.apk>
      Print package, version, SDK levels, CPU types and signer.
      Exits with an error when phones would refuse the APK, when the
      signature does not verify, or when it carries the debug key.

  pr comments <number> [--all] [--full]
      Print review threads, unresolved first. --all adds resolved
      threads. --full prints whole comment bodies.

  pr wait-review <number> [--timeout-mins N]
      Wait until CodeRabbit finishes on the PR's latest commit.
      Checks every 30 seconds; gives up after N minutes (default 20).

  release check-tag <tag>
      Check that the tag is v + the version in crates/werdy-app/Cargo.toml
      and print the version. The release workflow runs it first.

  help
      Show this text.
";

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Gates { android: bool, dry_run: bool },
    ApkInspect { path: String },
    PrComments { number: u32, all: bool, full: bool },
    PrWaitReview { number: u32, timeout_mins: u64 },
    ReleaseCheckTag { tag: String },
}

pub fn parse(args: &[String]) -> Result<Command, String> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["help"] | ["--help"] | ["-h"] => Ok(Command::Help),
        ["gates", flags @ ..] => {
            check_flags(flags, &["--android", "--dry-run"])?;
            Ok(Command::Gates {
                android: flags.contains(&"--android"),
                dry_run: flags.contains(&"--dry-run"),
            })
        }
        ["apk", "inspect", path] => Ok(Command::ApkInspect {
            path: (*path).to_string(),
        }),
        ["pr", "comments", number, flags @ ..] => {
            check_flags(flags, &["--all", "--full"])?;
            Ok(Command::PrComments {
                number: pr_number(number)?,
                all: flags.contains(&"--all"),
                full: flags.contains(&"--full"),
            })
        }
        ["pr", "wait-review", number] => Ok(Command::PrWaitReview {
            number: pr_number(number)?,
            timeout_mins: DEFAULT_TIMEOUT_MINS,
        }),
        ["pr", "wait-review", number, "--timeout-mins", minutes] => Ok(Command::PrWaitReview {
            number: pr_number(number)?,
            timeout_mins: minutes
                .parse()
                .map_err(|_| format!("--timeout-mins wants a whole number, got {minutes}"))?,
        }),
        ["release", "check-tag", tag] => Ok(Command::ReleaseCheckTag {
            tag: (*tag).to_string(),
        }),
        _ => Err(format!(
            "unknown command `{}`. Run `cargo xtask help`.",
            words.join(" ")
        )),
    }
}

fn check_flags(flags: &[&str], allowed: &[&str]) -> Result<(), String> {
    match flags.iter().find(|flag| !allowed.contains(flag)) {
        Some(flag) => Err(format!(
            "unknown flag {flag}. Allowed here: {}",
            allowed.join(", ")
        )),
        None => Ok(()),
    }
}

fn pr_number(text: &str) -> Result<u32, String> {
    text.trim_start_matches('#')
        .parse()
        .map_err(|_| format!("expected a PR number, got {text}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_words(line: &str) -> Result<Command, String> {
        let args: Vec<String> = line.split_whitespace().map(String::from).collect();
        parse(&args)
    }

    #[test]
    fn no_words_shows_help() {
        assert_eq!(parse_words(""), Ok(Command::Help));
    }

    #[test]
    fn gates_reads_both_flags_in_any_order() {
        assert_eq!(
            parse_words("gates --dry-run --android"),
            Ok(Command::Gates {
                android: true,
                dry_run: true
            })
        );
    }

    #[test]
    fn unknown_flag_is_an_error_not_ignored() {
        let error = parse_words("gates --andriod").unwrap_err();
        assert!(error.contains("--andriod"));
    }

    #[test]
    fn pr_number_may_start_with_a_hash() {
        assert_eq!(
            parse_words("pr comments #82 --all"),
            Ok(Command::PrComments {
                number: 82,
                all: true,
                full: false
            })
        );
    }

    #[test]
    fn wait_review_uses_default_timeout_unless_given() {
        assert_eq!(
            parse_words("pr wait-review 80"),
            Ok(Command::PrWaitReview {
                number: 80,
                timeout_mins: DEFAULT_TIMEOUT_MINS
            })
        );
        assert_eq!(
            parse_words("pr wait-review 80 --timeout-mins 5"),
            Ok(Command::PrWaitReview {
                number: 80,
                timeout_mins: 5
            })
        );
    }

    #[test]
    fn release_check_tag_takes_one_tag() {
        assert_eq!(
            parse_words("release check-tag v0.1.2"),
            Ok(Command::ReleaseCheckTag {
                tag: "v0.1.2".to_string()
            })
        );
        assert!(parse_words("release check-tag").is_err());
    }

    #[test]
    fn bad_numbers_are_errors() {
        assert!(parse_words("pr comments abc").is_err());
        assert!(parse_words("pr wait-review 80 --timeout-mins soon").is_err());
    }
}
