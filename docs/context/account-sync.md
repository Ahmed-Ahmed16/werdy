# Account and background sync

The app works without an account. Settings offers Google sign in for backup
across phones. Sharing remains issue #24. Public ids and the publishable key
live in `crates/werdy-app/src/account/config.rs`.

Android uses Credential Manager in the existing Kotlin library. Rust loads both
bridge classes through the activity class loader and polls sign in every 300 ms.
Web sign in renders the Google Identity Services button through document eval.
Both make 32 random bytes as hex. Google gets the SHA 256 hex of that raw string;
Supabase gets the original string in the ID token grant. Sign in waits at most
120 seconds. Closing the component cancels the Rust wait and ignores late
callbacks. Android also cancels its Credential Manager request.

`werdy-sync::SupabaseAuth` parses access and refresh tokens, expiry, user id and
email. No token or raw provider error is logged. Refresh happens two minutes
before expiry, or once after a 401. A refreshed user id must match the old one.
The rotated refresh token is saved before sync continues. A failed save stops
that attempt and the next attempt retries persistence with the retained token.

## Persistence

On Android, `session.json` and `sync.json` live next to `werdy.json` in the app's
private files folder. Writes use a sibling temporary file and rename, like
`FileStore`. On the web the keys are `werdy.session` and `werdy.sync`. Sessions
contain secrets; they never become part of `OwnerData` or a backend change row.

`sync.json` contains owner identity, a 128 bit random install id, the engine,
last successful sync time and a JSON records checkpoint. That checkpoint keeps
the engine and its owner snapshot atomic despite separate files. A save captures
edits and commits this checkpoint before writing the ordinary records file.
Startup uses the checkpoint for a signed in account, so a crash or failed mirror
write cannot pair an advanced cursor with old records. Session owner identity
must match. Demo mode does not load or save account state.

Owner records schema 12 also keeps each deed's stable wire identity, including
deleted deeds. This is domain identity, with no account id or credential. It
survives sign out so signing in again cannot import the same deeds as new ones.
Older records load with no identities and gain them at first sign in. The store
change is needed because deleting `sync.json` must not delete deed identity.

## Background work

Sync runs after sign in, on a signed in app start, three seconds after the last
save and every five minutes while open. One request sequence runs at a time.
A failed attempt retries after 30 seconds, or after another save. Scheduling
uses elapsed time so the phone's wall clock cannot postpone retries for years.
Wall clock samples are only for token expiry, the last sync display and offset
measurement. Offsets greater than two minutes are logged without credentials.

A request owns a copy of the engine and records. Edits remain usable during
network waits. Completion imports server rows into current id bindings, then
overlays newer queued edits and saves the checkpoint. A new local deed cannot
alias a deed downloaded during the same wait. Screens accept this result without
comparing server revisions to phone times, and reminders are rescheduled.
Push targets 32 operations per request. A deed header and all its pending rules
stay together, even when that group exceeds 32 operations. Earlier accepted
batches remain safe to retry if a later batch fails. Incomplete downloaded deeds
wait for their rules without blocking the pull cursor or applying their marks.

Editors refuse notes over 8192 UTF8 bytes and names over 1024 UTF8 bytes.
Capture checks these limits and keys from 1 to 128 bytes. Oversized rows,
including on first capture, stay in saved local only rows, with related deed operations kept
together. The app logs limit reasons without row contents. Other records keep
syncing. Local only rows overlay server data until corrected, then queue again.

Sign out saves the current records, removes the session, cancels the active
sync task and removes sync state. It keeps local history. A generation check
also prevents a late completion from restoring old account state. Storage
errors are shown instead of claiming that a failed sign out succeeded.

## Checks for the orchestrator

1. Apply the reviewed SQL migration as owner. Build Android with dx outside this
   agent run. Confirm one Gradle library, both JNI bridge classes, the Google
   dependencies and release keep rules. Kotlin must remain 2.0.20 with
   credentials 1.3.0, credentials play services auth 1.3.0, googleid 1.1.1 and
   kotlinx coroutines android 1.8.1.
2. On the emulator, sign in through the native sheet, including a new Google
   account that needs the fallback option. Cancel, retry, background the app
   during sign in and reopen it. Confirm errors contain no credentials.
3. On an allowed web OAuth origin, check the GIS popup in Arabic and English.
   Test script loading failure, cancellation, retry and reload with a saved
   session. Confirm offline screens stay usable. Check storage denied and full.
4. Make separate offline histories on two devices, sign in to the same account
   and confirm both histories appear. Edit during a slow sync, including a new
   deed. Confirm no edit is lost, and reminders follow downloaded changes.
5. Test expiry and a server 401, then confirm the rotated refresh token survives
   restart. Drop connectivity during a push and retry. Confirm no duplicate
   deeds or old winning marks. Change the phone clock and inspect offset logs.
6. Sign out during a slow sync, then restart. Session and sync state must stay
   absent while records remain. Verify demo mode never touches the account.
7. Check multiple tabs and storage events separately before relying on concurrent
   signed in tabs. The coordinator serializes work within one app instance.

Offline tests include `cargo test --workspace`,
`cargo test -p werdy-sync --features supabase` and
`node --test crates/werdy-app/tests/google-signin.test.mjs`.
These do not contact either provider. No emulator or live auth checks were run
while writing this change.
