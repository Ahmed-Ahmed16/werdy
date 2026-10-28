# Google Play Data safety draft

For owner review, not submission. Scope: the Android app with optional Google
sign in, Supabase sync and watcher sharing. Includes optional paths, not just
use without an account. Repo facts only. Current Play Console wording and its
exceptions have not been checked: ___. The category mappings below are proposed
answers based on the payload, not a verified interpretation of Google's rules.

## Overall answers

| Form item | Draft answer | Repo fact |
| --- | --- | --- |
| Does the app collect data? | Yes, when sign in or sharing is used. | `docs/context/account-sync.md` and `crates/werdy-sync/src/rows.rs`: account sessions and owner records enter Supabase. |
| Does the app share data? | Describe watcher transfers explicitly. Conservative draft: Yes for the types listed below. Whether Play excludes sharing explicitly initiated by the owner: ___. | `supabase/migrations/20261029000000_sharing.sql`: an authorized watcher receives the owner's change stream; sharing lists return email and account id. |
| Is all collected data encrypted in transit? | Yes for implemented account, sync and sharing traffic to the configured HTTPS endpoint. Final answer for all release dependencies: ___. | `crates/werdy-app/src/account/config.rs` uses HTTPS; `crates/werdy-sync/src/supabase.rs` rejects plain HTTP except loopback development origins. `crates/werdy-sync/src/supabase/auth.rs` uses that transport. A release network audit has not been run. |
| Can people request deletion? | Working request route: ___. Do not answer Yes until supplied and tested. | `crates/werdy-app/src/ui/account.rs` offers sign out, not account deletion. `docs/context/account-sync.md` says sign out keeps history. |
| Account creation method | Google sign in. No app password flow found. | `crates/werdy-app/src/account/signin.rs`, `crates/werdy-sync/src/supabase/auth.rs`. |
| Account deletion URL | ___. Publication gap. | No public deletion page or app account deletion action is present. The planned privacy URL is not a deletion endpoint. |
| Can someone use the app without an account? | Yes. | ADR 0015 in `docs/decisions.md` and `docs/context/account-sync.md`. |
| Can someone request some data be deleted without deleting the account? | App permits deleting deeds and notes. Complete server erasure request route: ___. | `crates/werdy-app/src/ui/deeds.rs` and `crates/werdy-app/src/ui/note.rs`; server retains earlier operations in `supabase/migrations/20261028000000_sync.sql`. |
| Independent security review or certification | ___. Do not claim one. | No certification evidence found in the reviewed repo files. |

## Data type answers

For every Yes below: data is associated with an account, retained beyond the
request, and therefore not processed only ephemerally. Collection is optional
for basic deed tracking because sign in is optional. Sharing with a watcher is
separately optional. No advertising, marketing, personalization or analytics
purpose is established by these payloads. Provider logging remains `___`.

| Data type | Collected | Shared with watcher | Purpose and required status | Repo fact |
| --- | --- | --- | --- | --- |
| Personal info: email address | Yes | Yes, to the other person in an active sharing relationship | Account management and app functionality. Needed for the implemented Google account path, optional for basic use. | `crates/werdy-sync/src/supabase/auth.rs` reads email; sharing SQL returns `u.email` in both person lists. |
| Personal info: user ids | Yes | Yes | Account management and app functionality. Optional account path. | Session `user_id` in auth source; sync rows belong to `auth.users.id`; sharing lists return the other person's id. |
| Personal info: religious or philosophical beliefs | Proposed Yes, because worship choices and records reveal religious practice | Yes | App functionality through optional sync and sharing. | Starter sets in `data/starter-sets.toml`; deed names, rules and marks in `crates/werdy-sync/src/rows.rs`. Do not omit this merely because no religion profile field exists. |
| App activity: other user generated content | Yes: deed names, notes, schedules, targets, marks and excuse credits | Yes | App functionality, including sync and the chosen watched view. Optional account path. | `Change::Deed`, `Rule`, `Mark` and `Note` in `crates/werdy-sync/src/rows.rs`; watch RPC returns those operations. |
| App activity: app interactions | Proposed Yes for saved choices and edits, not a separate analytics feed | Yes for synced settings and chosen sets | App functionality. Optional account path. | `Setting` and `ChosenSet` changes, operation ids and server receipts in rows source and sync SQL. |
| Device or other ids | Proposed Yes for the random install id. It is not an advertising or hardware id. | Yes, in operation metadata | App functionality: distinguish installations, order edits and avoid duplicate operations. Optional account path. | `docs/context/account-sync.md` specifies a random install id; watch RPC returns `pending` operations with install identity. |
| Health and fitness: health info | Conditional: a person can enter health details in notes or deed names. Final category choice: ___. | Yes if included in synced text | App functionality, at the person's choice. No health measurement API established here. | `DayNote` text is unrestricted in meaning; demo notes include illness in `crates/werdy-app/src/demo.rs` and `locales/ar/common.ftl`. The Health category also exists in `locales/en/deeds.ftl`. |
| Location: approximate and precise | No for the app's saved location fields. Provider inferred location from network requests: ___. | No for saved location fields | Local Fajr calculation only. Optional place choice. | ADR 0010; `Settings` in `crates/werdy-store/src/lib.rs`; `Setting` in `crates/werdy-sync/src/rows.rs` excludes location and prayer method. Typed location in a name or note follows the text content path instead. |
| Personal info: name and other profile details | Dedicated app profile collection: ___. Do not assert Google or Supabase stores only email. | Sharing RPC lists expose id, email and creation time only | Provider metadata handling: ___. | Auth parser reads id and email, but the Google ID token is passed to Supabase. Repo does not establish all provider metadata retention. |
| Financial info, contacts, messages, photos, videos, audio, files and docs, calendar events, browsing history | No dedicated collection path found. Any such details typed into a day note follow its content path. | No dedicated transfer found; notes still shared | No purpose established for dedicated collection. | Reviewed records in `crates/werdy-store/src/lib.rs` and sync wire types in `crates/werdy-sync/src/rows.rs` contain no such fields. Hijri schedule settings are app settings, not access to a device calendar. |
| App info and performance: crash logs, diagnostics and other performance data | No remote reporting path found in reviewed app and sync sources. Provider logs: ___. | No dedicated watcher transfer | No remote analytics purpose established | `docs/context/account-sync.md` describes local logs of clock offsets and limits without content or credentials. It does not establish a remote log service. |

## Recipients and sensitive content

1. Google authenticates the chosen account. The app sends a Google ID token and
   nonce to Supabase. This is credentials handling, not a claim that Google
   receives deed records. Evidence: `crates/werdy-sync/src/supabase/auth.rs`.
2. Supabase provides account, sync and sharing functions. The data remains on
   its server beyond the request. Classification as a service provider rather
   than form reportable sharing must be confirmed by the owner: ___. Evidence:
   `docs/context/supabase-sync.md` and both migrations in `supabase/migrations/`.
3. The chosen watcher receives the sync operation history, not only the fields
   visible on screen. That includes earlier note and deed values, settings and
   install metadata. Read only prevents writes, not receipt of those values.
   Evidence: `werdy_watch_pull` in the sharing migration and
   `crates/werdy-sync/src/watch.rs`.
4. The interface requests Google Fonts in `crates/werdy-app/src/ui/mod.rs`.
   Network metadata, IP handling, SDK behavior and provider retention for the
   release build: ___. Do not equate no dedicated analytics code with no data
   handled by providers. Web hosting on Pages is separate from Android hosting.

## Deletion, retention and release blockers

* Deleting a deed or day note removes it from the current view. Sync stores
  tombstones and retains earlier operations. Do not call item deletion complete
  erasure. Evidence: `crates/werdy-sync/src/rows.rs` and the sync migration.
* Account deletion cascades to linked sync and sharing rows, but no app action
  initiates it. External request route, controller contact, response time and
  live deletion verification: ___. Evidence: account UI and foreign keys in both
  migrations.
* Normal server retention and log retention: ___. No expiry process is defined
  for sync history. Local history survives sign out. Evidence:
  `docs/context/account-sync.md` and `docs/context/supabase-sync.md`.
* Requested #25 policy: encrypted daily backup, 30 day retention. Implementation,
  deployed schedule, destination, restore evidence and deletion from backups:
  ___. ADR 0027 confirms a planned scheduled dump, not those operational facts.
* Publish the completed [privacy policy](privacy-policy.md), supply the deletion
  URL, check the actual Console categories and exceptions, audit release network
  traffic, then submit answers matching the shipped build. No provider was
  contacted and no form was submitted for this ticket.
