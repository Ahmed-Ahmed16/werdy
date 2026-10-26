# Issue tracker: GitHub (planned)

Issues and specs for werdy will live as GitHub issues on `Ahmed-Ahmed16/werdy`. Use the `gh` CLI for all operations.

**Status: planned.** The remote does not exist yet, and pushing needs the owner's OK. Until it exists, a skill that wants to publish an issue writes the draft to `.scratch/<feature>/<slug>.md` instead and tells the owner. Once the repo exists, run `gh label create` for the labels in `docs/agents/triage-labels.md`.

## Conventions

* **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for bodies that span lines.
* **Read an issue**: `gh issue view <number> --json number,title,body,labels,comments`.
* **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with `--label` and `--state` filters as needed.
* **Child issue of a parent**: `gh issue create --parent <parent> ...`, or `gh issue edit <parent> --add-sub-issue <child>` afterwards (`gh` 2.94 or newer). Without sub issues, put `Part of #<parent>` at the top of the child body.
* **Comment**: `gh issue comment <number> --body "..."`
* **Labels**: `gh issue edit <number> --add-label "..."` and `--remove-label "..."`
* **Close**: `gh issue close <number> --comment "..."`

Infer the repo from `git remote -v`; `gh` does this by itself inside a clone.

## Pull requests as a triage surface

**PRs as a request surface: no.** (Set to `yes` if external PRs should be treated as feature requests; `/triage` reads this flag.)

When set to `yes`, PRs go through the same labels and states as issues, using `gh pr view`, `gh pr diff`, `gh pr comment`, `gh pr edit` and `gh pr close`. GitHub shares one number space for issues and PRs, so resolve a bare `#42` with `gh pr view 42` first and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue (or a `.scratch/` draft while the remote is planned).

## When a skill says "fetch the relevant ticket"

Read it as in **Read an issue** above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is one issue with **child** issues as tickets.

* **Map**: one issue labelled `wayfinder:map`, holding the Notes, Decisions so far and Fog body.
* **Child ticket**: an issue linked to the map as a sub issue, labelled `wayfinder:<type>` (`research`, `prototype`, `grilling`, `task`). Once claimed, it is assigned to the person driving it.
* **Blocking**: GitHub's native issue dependencies: `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where the id is the numeric database id (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`). Fallback: a `Blocked by: #<n>` line at the top of the child body.
* **Frontier**: the map's open children with no open blocker and no assignee; first in map order wins.
* **Claim**: `gh issue edit <n> --add-assignee @me`.
* **Resolve**: comment the answer, close the issue, then add a one line pointer to the map's Decisions so far.
