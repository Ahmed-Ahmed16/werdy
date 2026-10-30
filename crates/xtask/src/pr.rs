//! `pr comments` and `pr wait-review`, through the `gh` CLI.
//! `{owner}` and `{repo}` are filled in by gh from the current checkout.

use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::process::capture_ok;

const CODERABBIT_APP: &str = "coderabbitai";
const POLL_EVERY: Duration = Duration::from_secs(30);
const SUMMARY_LINES: usize = 3;

// One page of 100 threads covers every PR so far.
const THREADS_QUERY: &str = "query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 100) {
        nodes {
          isResolved
          isOutdated
          path
          line
          comments(first: 50) { nodes { author { login } body url } }
        }
      }
    }
  }
}";

#[derive(Debug, PartialEq, Eq)]
pub struct Thread {
    pub path: String,
    pub line: Option<u64>,
    pub resolved: bool,
    pub outdated: bool,
    pub comments: Vec<Comment>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Comment {
    pub author: String,
    pub body: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewState {
    NotStarted,
    InProgress,
    Finished { conclusion: String, title: String },
}

pub fn comments(number: u32, all: bool, full: bool) -> Result<(), String> {
    let json = capture_ok(
        "gh",
        [
            "api".to_string(),
            "graphql".to_string(),
            "-F".to_string(),
            "owner={owner}".to_string(),
            "-F".to_string(),
            "name={repo}".to_string(),
            "-F".to_string(),
            format!("number={number}"),
            "-f".to_string(),
            format!("query={THREADS_QUERY}"),
        ],
    )?;
    print!(
        "{}",
        render_threads(number, &parse_threads(&json)?, all, full)
    );
    Ok(())
}

pub fn wait_review(number: u32, timeout_mins: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(timeout_mins * 60);
    let mut last_state = None;
    loop {
        let sha = capture_ok(
            "gh",
            [
                "pr",
                "view",
                &number.to_string(),
                "--json",
                "headRefOid",
                "-q",
                ".headRefOid",
            ],
        )?;
        let sha = sha.trim();
        let runs = capture_ok(
            "gh",
            [
                "api",
                &format!("repos/{{owner}}/{{repo}}/commits/{sha}/check-runs"),
            ],
        )?;
        let state = coderabbit_state(&runs)?;
        if last_state.as_ref() != Some(&state) {
            println!("{}", describe(&state, sha));
        }
        if matches!(state, ReviewState::Finished { .. }) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "CodeRabbit did not finish on PR #{number} within {timeout_mins} minutes"
            ));
        }
        last_state = Some(state);
        thread::sleep(POLL_EVERY);
    }
}

/// Reads the GraphQL answer from `THREADS_QUERY`.
pub fn parse_threads(json: &str) -> Result<Vec<Thread>, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|error| format!("gh returned bad JSON: {error}"))?;
    let nodes = value
        .pointer("/data/repository/pullRequest/reviewThreads/nodes")
        .and_then(Value::as_array)
        .ok_or("no review threads in gh's answer; is the PR number right?")?;
    Ok(nodes.iter().map(thread_from).collect())
}

/// Finds CodeRabbit's newest check run in a check runs answer.
pub fn coderabbit_state(json: &str) -> Result<ReviewState, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|error| format!("gh returned bad JSON: {error}"))?;
    let runs = value
        .get("check_runs")
        .and_then(Value::as_array)
        .ok_or("no check_runs in gh's answer")?;
    let newest = runs
        .iter()
        .filter(|run| run.pointer("/app/slug").and_then(Value::as_str) == Some(CODERABBIT_APP))
        .max_by_key(|run| text(run, "/started_at"));
    Ok(match newest {
        None => ReviewState::NotStarted,
        Some(run) if text(run, "/status") == "completed" => ReviewState::Finished {
            conclusion: text(run, "/conclusion"),
            title: text(run, "/output/title"),
        },
        Some(_) => ReviewState::InProgress,
    })
}

pub fn render_threads(number: u32, threads: &[Thread], all: bool, full: bool) -> String {
    let open = threads.iter().filter(|thread| !thread.resolved).count();
    let mut shown: Vec<&Thread> = threads
        .iter()
        .filter(|thread| all || !thread.resolved)
        .collect();
    shown.sort_by_key(|thread| (thread.resolved, thread.path.as_str(), thread.line));

    let mut out = format!(
        "PR #{number}: {open} unresolved of {} threads\n",
        threads.len()
    );
    for thread in shown {
        let line = thread
            .line
            .map(|line| format!(":{line}"))
            .unwrap_or_default();
        let mut marks = String::new();
        if thread.resolved {
            marks.push_str(" [resolved]");
        }
        if thread.outdated {
            marks.push_str(" [outdated]");
        }
        out.push_str(&format!("\n{}{line}{marks}\n", thread.path));
        for comment in &thread.comments {
            let body = if full {
                comment.body.trim().to_string()
            } else {
                summary(&comment.body)
            };
            out.push_str(&format!("  {}:\n", comment.author));
            for body_line in body.lines() {
                out.push_str(&format!("    {body_line}\n"));
            }
            out.push_str(&format!("    {}\n", comment.url));
        }
    }
    out
}

/// The first few plain text lines of a comment. Drops HTML tags, HTML
/// comments and whole `<details>` blocks, where CodeRabbit keeps its logs.
/// Text such as `Vec<u32>` loses its angle brackets; `--full` keeps them.
pub fn summary(body: &str) -> String {
    let mut plain = String::new();
    let mut depth = 0usize;
    let mut rest = body;
    while let Some(start) = rest.find('<') {
        let Some(length) = rest[start..].find('>') else {
            break;
        };
        if depth == 0 {
            plain.push_str(&rest[..start]);
        }
        let tag = &rest[start..start + length + 1];
        if tag.starts_with("<details") {
            depth += 1;
        } else if tag.starts_with("</details") {
            depth = depth.saturating_sub(1);
        }
        rest = &rest[start + length + 1..];
    }
    if depth == 0 {
        plain.push_str(rest);
    }
    plain
        .replace("**", "")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(SUMMARY_LINES)
        .collect::<Vec<_>>()
        .join("\n")
}

fn describe(state: &ReviewState, sha: &str) -> String {
    let short = &sha[..sha.len().min(7)];
    match state {
        ReviewState::NotStarted => format!("{short}: CodeRabbit has not started. Waiting."),
        ReviewState::InProgress => format!("{short}: CodeRabbit is reviewing. Waiting."),
        ReviewState::Finished { conclusion, title } if conclusion == "skipped" => format!(
            "{short}: CodeRabbit skipped this commit ({title}). \
             If a review was just asked for by comment, run this again in a minute."
        ),
        ReviewState::Finished { conclusion, title } => {
            format!("{short}: CodeRabbit finished: {conclusion} ({title}).")
        }
    }
}

fn thread_from(node: &Value) -> Thread {
    let comments = node
        .pointer("/comments/nodes")
        .and_then(Value::as_array)
        .map(|nodes| {
            nodes
                .iter()
                .map(|comment| Comment {
                    author: text(comment, "/author/login"),
                    body: text(comment, "/body"),
                    url: text(comment, "/url"),
                })
                .collect()
        })
        .unwrap_or_default();
    Thread {
        path: text(node, "/path"),
        line: node.get("line").and_then(Value::as_u64),
        resolved: node.get("isResolved").and_then(Value::as_bool) == Some(true),
        outdated: node.get("isOutdated").and_then(Value::as_bool) == Some(true),
        comments,
    }
}

/// A string field, or empty when it is missing or null.
fn text(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Same shape as gh's answer for PR #82; bodies shortened.
    const THREADS: &str = r#"{"data":{"repository":{"pullRequest":{"reviewThreads":{"nodes":[
      {"isResolved":true,"isOutdated":true,"path":"src/b.rs","line":null,
       "comments":{"nodes":[{"author":{"login":"coderabbitai"},"body":"Old note","url":"https://x/1"}]}},
      {"isResolved":false,"isOutdated":false,"path":"src/a.rs","line":12,
       "comments":{"nodes":[
         {"author":{"login":"coderabbitai"},"body":"Check the error path","url":"https://x/2"},
         {"author":{"login":"Ahmed-Ahmed16"},"body":"Fixed in abc123","url":"https://x/3"}]}}
    ]}}}}}"#;

    fn check_runs(runs: &str) -> String {
        format!(r#"{{"total_count":2,"check_runs":[{runs}]}}"#)
    }

    const CI_RUN: &str = r#"{"name":"check","status":"completed","conclusion":"success",
      "started_at":"2026-10-30T10:00:00Z","app":{"slug":"github-actions"},"output":{"title":null}}"#;

    #[test]
    fn threads_keep_path_line_state_and_comments() {
        let threads = parse_threads(THREADS).unwrap();
        assert_eq!(threads.len(), 2);
        assert_eq!(threads[0].line, None);
        assert!(threads[0].resolved && threads[0].outdated);
        assert_eq!(threads[1].path, "src/a.rs");
        assert_eq!(threads[1].line, Some(12));
        assert_eq!(threads[1].comments[1].author, "Ahmed-Ahmed16");
    }

    #[test]
    fn wrong_pr_number_is_a_clear_error() {
        let error = parse_threads(r#"{"data":{"repository":{"pullRequest":null}}}"#).unwrap_err();
        assert!(error.contains("PR number"));
    }

    #[test]
    fn render_hides_resolved_threads_unless_all() {
        let threads = parse_threads(THREADS).unwrap();
        let open_only = render_threads(82, &threads, false, false);
        assert!(open_only.starts_with("PR #82: 1 unresolved of 2 threads"));
        assert!(open_only.contains("src/a.rs:12"));
        assert!(!open_only.contains("src/b.rs"));

        let everything = render_threads(82, &threads, true, false);
        let open_at = everything.find("src/a.rs").unwrap();
        let resolved_at = everything.find("src/b.rs [resolved] [outdated]").unwrap();
        assert!(open_at < resolved_at, "unresolved threads come first");
    }

    #[test]
    fn summary_drops_markup_and_details_blocks() {
        let body = "<!-- coderabbit-ai-generated:v1 -->\n\n\
            **<picture><img src=\"x.svg\"></picture> Minor** · **Quick win**\n\
            <details>\n<summary>Supported by static analysis</summary>\n\
            <details><summary>inner</summary>sed -n '1,9p' file</details>\n</details>\n\n\
            **Handle the empty list.**\n\nThe loop reads item 0 first.\nMore text.";
        assert_eq!(
            summary(body),
            "Minor · Quick win\nHandle the empty list.\nThe loop reads item 0 first."
        );
    }

    #[test]
    fn no_coderabbit_run_means_not_started() {
        let state = coderabbit_state(&check_runs(CI_RUN)).unwrap();
        assert_eq!(state, ReviewState::NotStarted);
    }

    #[test]
    fn running_review_is_in_progress() {
        let run = r#"{"name":"CodeRabbit","status":"in_progress","conclusion":null,
          "started_at":"2026-10-30T10:01:00Z","app":{"slug":"coderabbitai"},
          "output":{"title":"Review in progress"}}"#;
        let state = coderabbit_state(&check_runs(&format!("{CI_RUN},{run}"))).unwrap();
        assert_eq!(state, ReviewState::InProgress);
    }

    #[test]
    fn newest_coderabbit_run_wins() {
        let skipped = r#"{"status":"completed","conclusion":"skipped","started_at":"2026-10-30T09:00:00Z",
          "app":{"slug":"coderabbitai"},"output":{"title":"Review skipped"}}"#;
        let done = r#"{"status":"completed","conclusion":"success","started_at":"2026-10-30T11:00:00Z",
          "app":{"slug":"coderabbitai"},"output":{"title":"Review completed"}}"#;
        let state = coderabbit_state(&check_runs(&format!("{skipped},{done}"))).unwrap();
        assert_eq!(
            state,
            ReviewState::Finished {
                conclusion: "success".to_string(),
                title: "Review completed".to_string()
            }
        );
    }
}
