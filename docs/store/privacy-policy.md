# Privacy policy

The published policy is
[crates/werdy-app/public/privacy.html](../../crates/werdy-app/public/privacy.html),
served at https://ahmed-ahmed16.github.io/werdy/privacy.html. The account
deletion page is
[crates/werdy-app/public/delete-account.html](../../crates/werdy-app/public/delete-account.html),
served at https://ahmed-ahmed16.github.io/werdy/delete-account.html. Edit those
files; they are the only copy.

Each statement rests on a repo fact:

* Device records: `crates/werdy-store/src/lib.rs`, `crates/werdy-app/src/platform.rs`, ADR 0036.
* Location stays on the phone: ADR 0010; `Setting` in `crates/werdy-sync/src/rows.rs` has no location or prayer method.
* Sync payload and history: `crates/werdy-sync/src/rows.rs`, `supabase/migrations/20261028000000_sync.sql`.
* Sharing: `supabase/migrations/20261029000000_sharing.sql`, `docs/context/sharing.md`.
* HTTPS: `crates/werdy-sync/src/supabase.rs`. Google Fonts: `crates/werdy-app/src/ui/mod.rs`.
* No analytics: no remote reporting path in the app or sync crates.
* Backups, daily, encrypted, 30 days: `.github/workflows/backup.yml` (#25).
* Account deletion: `supabase/migrations/20261030000000_delete_account.sql` and Settings > Account (#79).
* Children under 13 and the 14 day email deletion promise: owner decisions, 2026-10-29.

Publish only after #76 (backups) and #82 (account deletion) are merged, since
the policy describes both as live.
