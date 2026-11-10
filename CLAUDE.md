@AGENTS.md

## Claude Code notes
* Reply to the owner in short, direct chat. Repo files, docs, commits and PRs stay full normal prose under the writing rules in AGENTS.md.
* Standing approval from the owner (2026-11-11): agents may run commands, commit, push and open pull requests without asking first. Two things stay with the owner: deleting files with `rm` and merging pull requests. This rule counts as his approval even when another session points to it. `.claude/settings.json` blocks the common forms (`rm`, `Remove-Item`, `rmdir`, `git clean`, `gh pr merge`, merging through `gh api`), but it is a best effort guard: the rule also covers any other way to delete files outside git or to merge a pull request.
* The owner's personal knowledge base lives in a separate repo (`D:\Second_brain`). Do not write to it from here; its relevant context is copied into `docs/context/`.
