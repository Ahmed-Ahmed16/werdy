# AGENTS.md: guide for AI agents and contributors

werdy is an open source, clean room tracker for daily Islamic deeds (and life work), written in Rust. Think of a budget tracker, but for deeds: every day, week and month you can see what you did, what you half did, and what you missed. Read this file first, then `docs/`.

**Status: pre code.** Nothing is built yet. The next steps are a grill session, the design, the spec, and then the skeleton commit.

## 1. Orientation

| Read | Why |
| --- | --- |
| `docs/context/werd-behaviour.md` | What the existing Werd app does, and the measured statistics bug that werdy exists to fix |
| `docs/decisions.md` | Decisions already made, and why. Do not reopen one without a new reason |
| `docs/context/owner.md` | Who owns this repo and how he wants agents to work with him |

## 2. Golden rules

1. **Clean room.** We study the Werd app (and others) for *behaviour and look only*: what a person sees and does. Never copy its code, assets or text, never call its backend, never use its data. Implement from observation and our own design.
2. **Statistics are the product.** Every number shown must come from a pure, tested function. A wrong number is a bug that blocks a release. The five scoring rules in `docs/context/werd-behaviour.md` are the first tests.
3. **Tests are the gate.** Every change comes with tests. A bug fix starts with a failing test.
4. **Never panic on input.** Code returns errors; it does not crash on bad data.
5. **Never fabricate.** An unknown fact is written as `___` and flagged, never guessed.
6. **One ticket at a time.** New ideas go to an issue, not into the current change.

## 3. Writing rules (all prose: docs, comments, commits, PRs, UI copy)

* No em dashes, no en dashes, no hyphens joining words in prose ("local first", not the hyphenated form). Hyphens stay only where syntax needs them: paths, code, flags, crate names, ISO dates.
* Plain words, short sentences. No buzzwords, no polished filler.

## 4. Working with the owner (learning first)

The owner is learning Rust and AI assisted engineering through this project.
* Before anything nontrivial, explain the approach and the why in plain language.
* Flag "you should type this" moments: small, critical pieces he writes himself with guidance.
* After a task, he should be able to explain every change in the diff.
* End every task with a short summary: what was done, files changed, what is verified versus pending, the next action.

## 5. Workflow (planned)

Align (`grill-with-docs`) → design → spec (`to-spec`) → tickets (`to-tickets`) → implement one ticket (`tdd`) → guards (`clean-code-guard`, `test-guard`, `docs-guard`) → two model PR review (`debate-review`, `babysit-pr`). The owner merges. Automation grows only as trust in the tests grows.

## 6. Tool kit

Agent skills installed on 2026-10-27 with `npx skills@latest add <repo> -a claude-code --copy`, into this project only (`.claude/skills/`, pinned in `skills-lock.json`). They are plain copies: update them by hand with `npx skills@latest update -p`, and review a skill before using it, since it runs with full agent permissions.

| Source | Skills | Used for |
| --- | --- | --- |
| [mattpocock/skills](https://github.com/mattpocock/skills) | `grill-with-docs`, `grilling`, `domain-modeling`, `wayfinder`, `to-spec`, `to-tickets`, `triage`, `implement-spec`, `implement`, `tdd`, `codebase-design`, `pr`, `prototype`, `research`, `setup-matt-pocock-skills` | Align, spec, tickets, test first implementation. `pr`, `prototype` and `research` are installed because the others call them |
| [amElnagdy/guard-skills](https://github.com/amElnagdy/guard-skills) | `clean-code-guard`, `test-guard`, `docs-guard` | Review gates after code, tests or docs change |
| [amElnagdy/review-skills](https://github.com/amElnagdy/review-skills) | `debate-review`, `babysit-pr` | Two model PR review and following a PR through bot reviews |
| [amElnagdy/delegate-skills](https://github.com/amElnagdy/delegate-skills) | `delegate-setup` and the `*-delegate` skills for each supported CLI | Required by `review-skills`. **Not configured yet:** `delegate-setup` runs only after the owner says which other model CLIs he has |

Not installed: Pocock's `code-review` (its name clashes with Claude Code's own `/code-review`, and `debate-review` covers the job).

**REA (morluto/rea), owner runs it.** Its setup is interactive, so the owner runs `npx rea-agents setup` himself in a terminal and declines the Hopper install: Werd's web app is plain JavaScript, so no native disassembler is needed. Clean room limit: use REA to observe behaviour (screens, flows, what a request does), never to copy code or text into werdy. Whoever reads Werd's code does not write the matching werdy code from it.

## Agent skills

### Issue tracker

GitHub issues on `Ahmed-Ahmed16/werdy`, planned (the remote does not exist yet). See `docs/agents/issue-tracker.md`.

### Triage labels

The five default labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single context: glossary in `CONTEXT.md`, ADRs in `docs/decisions.md`. See `docs/agents/domain.md`.
