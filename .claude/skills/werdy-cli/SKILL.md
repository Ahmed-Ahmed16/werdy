---
name: werdy-cli
description: werdy's shared task runner, `cargo xtask`. Use it for every recurring job in this repo instead of writing a one off script, temp file or inline loop: running the CI gates locally, inspecting an APK (SDK levels, CPU types, signer), reading a PR's review threads, or waiting for CodeRabbit to finish on a PR.
---

# werdy CLI (`cargo xtask`)

The code lives in `crates/xtask`. Run every command from the repo root (or a worktree root). `cargo xtask help` prints the same list.

## Rule

Use these commands for the jobs below. Do not write a `.ps1`, `.sh` or Python script, a temp file in the repo, or a polling loop for them. If a recurring job is missing, propose a new subcommand in an issue (golden rule 6), with tests, instead of a script.

## Commands

| Job | Command | Exit code |
| --- | --- | --- |
| Run the CI gates before a commit or PR | `cargo xtask gates` | 1 at the first failed step |
| Also the Android clippy and debug APK build | `cargo xtask gates --android` | same |
| See the gate commands without running them | `cargo xtask gates --dry-run` | 0 |
| Check an APK before sharing it | `cargo xtask apk inspect <file.apk>` | 1 when there is a problem |
| Read unresolved review threads on a PR | `cargo xtask pr comments <n>` | 1 when gh fails |
| Same, with resolved threads and whole bodies | `cargo xtask pr comments <n> --all --full` | same |
| Wait for CodeRabbit on the PR's newest commit | `cargo xtask pr wait-review <n> [--timeout-mins N]` | 1 on timeout |
| Check a release tag against the app version | `cargo xtask release check-tag <tag>` | 1 when they differ |

## What the output means

* **gates**: prints `[k/total] command` before each step, then `All N steps passed.` The steps are the ones in `.github/workflows/ci.yml`; a unit test keeps the two the same. The headless Chrome tests are not included.
* **apk inspect**: prints package, version, SDK levels, CPU types, signer and certificate SHA-256, then one `PROBLEM:` line for each of: no `arm64-v8a` code (phones refuse it, #84), a signature that does not verify, or the debug key. Needs `ANDROID_HOME`; uses the newest build tools there.
* **pr comments**: one block per thread, `path:line`, marked `[resolved]` or `[outdated]`, with each comment's author, a three line summary and its link. The summary drops HTML and CodeRabbit's `<details>` logs; use `--full` before replying to a comment.
* **release check-tag**: prints the version when the tag is `v` + the version in `crates/werdy-app/Cargo.toml`. The release workflow runs it first. To release, raise the version, merge, then push the tag (ask the owner first).
* **pr wait-review**: checks CodeRabbit's check run every 30 seconds and prints a line when its state changes. "Skipped" means CodeRabbit did not review this commit. On this repo reviews are often asked for by hand; asking is the owner's call, so ask him before posting `@coderabbitai review`.

## Releases

Releases are built by `.github/workflows/release.yml` when a `v*` tag is pushed (#93), using `scripts/build-android-release.ps1` and the keystore from repository secrets. Never ask for or print a keystore password. Pushing a tag publishes a release, so ask the owner first.
