# One shared CLI for recurring agent jobs

Written 2026-10-30. The owner approved the plan the same day. The first version is in `crates/xtask`; the agent instructions are in `.claude/skills/werdy-cli/SKILL.md` and section 7 of `AGENTS.md`. Unknown facts are written as `___`.

## The problem

Agents in this repo kept writing throwaway scripts for jobs that come back every week: a one off `build-release-apk.ps1`, temp files for issue bodies, polling loops typed inline to wait for a bot review. Each new session learned the same steps again, and each copy was slightly different. None of these scripts had tests.

## Where the idea comes from

The owner pointed to poteto's `poteto-mode` skill in the pstack plugin ([cursor/plugins, pstack/skills/poteto-mode](https://github.com/cursor/plugins/blob/main/pstack/skills/poteto-mode/SKILL.md)). The pstack README is signed by poteto ([x.com/poteto](https://x.com/poteto)), who works on React and the React compiler. The skill ships no CLI of its own. Two of its principles fit our problem:

* **Build the lever.** For any nontrivial job, build the tool that does or proves it (a codemod, script, generator, or a skill that agents follow) instead of working by hand. A reviewer can run the tool again.
* **Encode lessons in structure.** When the same instruction is given twice, turn it into a lint, a check or a script.

`cargo xtask` is how werdy applies both: the lever is one tested CLI, and the lesson ("do not write your own script") is written into a skill and `AGENTS.md`.

Also looked at and not used: the GitHub user `poteto` has no repo called `lauren`. poteto's [noodle](https://github.com/poteto/noodle) is a Go tool that schedules agents in separate worktrees and merges their work on its own. werdy needs fixed commands instead, and here the owner merges.

## The design

A small Rust binary in the workspace, `crates/xtask`, run as `cargo xtask <job>` through an alias in `.cargo/config.toml`. The xtask pattern is common in Rust projects ([matklad/cargo-xtask](https://github.com/matklad/cargo-xtask)).

Why this shape:

* **Same command for every agent and the owner.** Claude, Codex and a person all type `cargo xtask gates`. No PowerShell versus bash split.
* **Rust, so it is tested like the rest.** Each job splits into a pure part (parse, decide) with unit tests and a thin part that runs `cargo`, `dx`, `gh` or the Android tools.
* **No new tool to install.** Cargo, `gh` and the Android SDK are already used.
* **One place to fix a step.** When a step changes, one function changes, and every agent picks it up.

### Commands in the first version

| Command | Job | Wraps |
| --- | --- | --- |
| `cargo xtask gates [--android] [--dry-run]` | the steps in `ci.yml`: fmt, clippy, tests, wasm clippy; with `--android` the Android clippy and debug APK build | `cargo`, `dx` |
| `cargo xtask apk inspect <file>` | package, version, SDK levels, CPU types, signer; fails when phones would refuse the APK, when the signature does not verify, or when it has the debug key | `aapt2`, `apksigner` |
| `cargo xtask pr comments <n> [--all] [--full]` | review threads, unresolved first, with short summaries | `gh api graphql` |
| `cargo xtask pr wait-review <n> [--timeout-mins N]` | wait for CodeRabbit's check run on the newest commit | `gh pr view`, `gh api` |

Rules every command follows: print each outside command before running it, return an error with a plain message instead of panicking, never print a secret, and write nothing into the repo.

### How Claude and Codex both use it

* **Claude Code** loads `.claude/skills/werdy-cli/SKILL.md` when a job matches its description.
* **Codex** reads `AGENTS.md`, whose section 7 lists the commands and points to the skill. Whether Codex also loads `.claude/skills/` directly: `___`.
* **Everyone** can run `cargo xtask help`, so the CLI documents itself even when an agent skips the skill.

The skill is written in this repo, so git tracks it. It is not in `skills-lock.json`, which only records skills pulled from other repos with `npx skills`.

### How it is tested

* **Parsers, with saved output.** `aapt2` and `apksigner` text, GitHub's review threads JSON and check runs JSON are kept as samples in the tests. The `aapt2` sample is real output from the v0.1.0 APK, which held only emulator code (#84). Signer names in the samples are made up.
* **The gate list against CI.** A test reads `.github/workflows/ci.yml` and fails when a gate command is no longer there, so the two cannot drift apart.
* **The argument parser.** Unknown flags and bad numbers are errors, not silently ignored.
* **CI.** The crate is a workspace member, so `cargo test --workspace` in CI runs these tests too. Calls to `gh`, `dx` and the Android tools are not run in CI; they were checked by hand on 2026-10-30 against real APKs and PRs #80 and #82.

## Next

1. `cargo xtask apk release`: move `scripts/build-android-release.ps1` (PR #81) into Rust after that PR merges. The keystore stays outside every checkout, encrypted on the owner's PC; the command reads it only through the `WERDY_ANDROID_*` environment variables, never prints a password, and runs `apk inspect` on what it built.
2. Retire the stray files agents left in the repo root (`s1.png` to `s11.png`, `*.log`).
3. New recurring jobs become subcommands through an issue, one at a time.
