# Handover: werdy, ready for to-spec (2026-10-27)

Repo `D:\werdy`, branch `main`, all work committed locally (no remote yet; pushing needs Ahmed's OK).

## Read first for the spec
* `AGENTS.md` (rules, Tool kit, Agent skills), `CONTEXT.md` (glossary), `docs/decisions.md` (ADRs 0001 to 0025).
* `docs/context/werd-behaviour.md` (five scoring rules = first tests), `docs/context/ux-research.md`, `docs/context/android-testing.md`, `docs/design/tokens.md`.
* Approved design canvas (ADR 0005): https://claude.ai/artifact/WmyN8xF2sjWMGV4AJYTk5y
* Showcase for friends: https://claude.ai/artifact/CHpHG79w13mo4CzhsLru44 (private until Ahmed shares it).

## Open `___` items the spec must raise
* Streak design (parked, ADR 0012).
* Supabase free tier limits and terms (ADR 0016), check before the spec closes.
* Ahmed's Excel/PDF deed tracker for starter sets (ADR 0019).
* Google sign in setup on Android and web (not researched yet); it drives the Dioxus vs Tauri 2 spike (ADR 0002, 0017).
* Font: IBM Plex Sans Arabic vs Noto Sans Arabic; Hijri date display.

## How Ahmed works
* Short direct chat; repo prose with no dashes joining words; never fabricate (`___`); explain before building; summary at the end of every task.
* Respectful tone toward Werd in any public copy (no "honest" or right vs wrong framing).
* Build for change: tokens, Fluent files per language per feature, data driven lists (ADR 0024, 0025).
* Pocock skills may now be started by the agent, only when Ahmed asks.

## Next
`/to-spec`, then `/to-tickets`, then the one day Google sign in spike (needs Rust, NDK, env vars, a tablet emulator; see android-testing.md).
