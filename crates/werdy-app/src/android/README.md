# Android reminders and Google sign in

Reminders use Android platform APIs. Sign in adds Credential Manager and Google
Identity. Versions are pinned to credentials 1.3.0, credentials play services auth
1.3.0, googleid 1.1.1 and kotlinx coroutines android 1.8.1, to work with Kotlin
2.0.20. The module targets compile SDK 35, minimum SDK 24 and Java 17. Dioxus 0.7.10
build supplies its Kotlin compiler, currently 2.0.20.

One Rust FFI block describes both classes for this library path. The generated
constructor assumes the Gradle namespace, so Rust uses the shared activity class
loader helper with each class's full package name instead. `GoogleSignin` lives
in `com.ahmedhossam.werdy.signin`. It starts Credential Manager, returns at once,
and is polled every 300 ms, for at most 120 seconds. It cancels when its Rust
request is dropped. Google gets the SHA 256 hex of a random raw nonce. Supabase
gets the raw nonce. Neither error replies nor logs include credentials.

Rust constructs `Reminders(Activity)` with the activity class loader because of
Dioxus issue #5629. String methods become `Result<String, String>` in Rust.
A worker coalesces saves, plans today and tomorrow, formats Fluent text, and calls
`schedule`. The WebView does not wait for alarm replacement or reminder preference writes.
Start, each successful save, and the watcher crossing a werdy day refresh the plan.

The JSON object has `channel_name` and a `notifications` array. Each entry has
`id`, `epoch_millis`, `title` and `body`. An empty array cancels all alarms.
`schedule` validates the incoming list, cancels previous pending intents, saves
the new list with a synchronous SharedPreferences commit, then arms future alarms.
The next refresh retries a failed replacement. Boot restores the last committed list.

Methods return `{"ok":true}`, `{"allowed":true}` or `{"error":"reason"}`.
`requestPermission` queues the Android permission sheet on the main thread and
returns before the answer. Settings polls `permission` while visible. Permission
includes both app access and channel access. The phone's notification settings
may be needed after denial. No permission sheet opens on app start.

The receiver loads the current saved entry before delivery so a stale alarm
cannot post old text or a removed reminder. Stable id hashes give request codes;
intent data separates hash collisions. Notifications use both a stable tag and id.
Tapping opens the app. The channel id is `reminders`, at default importance.
Its visible name comes from Fluent. The small icon is our own vector bell.

`setAndAllowWhileIdle` uses inexact `RTC_WAKEUP` alarms, with no exact alarm
permission. Android can delay or batch delivery in Doze. The boot receiver restores
only future entries after `BOOT_COMPLETED` or `MY_PACKAGE_REPLACED`. Both receivers
stay private; system broadcasts can reach a receiver with `exported="false"`.

Only the saved two day plan survives while the app is closed. Boot restores that
plan, not a new Rust plan. Open the app again to extend it. Clock changes and a
new device time zone need an app restart. A nonexistent clock minute at a DST
jump returns an error; a repeated minute resolves to one occurrence.

## Emulator checks for the orchestrator

1. Build the library through Dioxus and inspect the merged manifest. Confirm the
   bridge class and receivers also survive release shrinking.
2. Enable a prayer slot, check delay choices, remove location, and follow the
   location prompt. Test a clock slot and a deed time. Cancel the deed editor,
   then reopen it and confirm no reminder changed.
3. On API 33 or newer, deny and allow notification permission. Confirm the button
   follows the answer. Disable the channel in system settings too. On API 24 or
   25, no permission sheet should appear and notifications should still post.
4. Set a future reminder, close the app, and check the notification and its tap.
   Mark Done or disable the reminder before delivery and check cancellation.
   Partial must still notify. Repeated saves must leave one alarm per stable id.
5. Check Arabic and English titles, deed names, long lists, and the small icon.
6. Reboot and replace the package. Future saved alarms return, past ones do not.
   Test Doze and remember delivery is inexact. Test DST and a time zone change.
7. Leave the app open across midnight and Fajr, then confirm the plan refreshes.
   Reopen after more than two days and confirm planning starts again.

