# Google sign in spike (#13), Dioxus track

Throwaway code. It answers one question for ADR 0029: can Dioxus 0.7 do Google
sign in on Android and the web, ending in a Supabase session? Result: yes on
Android (verified on the emulator). The web build runs and shows the Google
button; the final sign in on the web is still to be done by the owner.

Android uses the system Credential Manager account sheet. The web uses Google
Identity Services (GIS). Both send the Google ID token and the raw nonce to
Supabase (`grant_type=id_token`) through the same `supabase::exchange` function.

## Run it

* Android (emulator or phone):
  `dx build --android --target x86_64-linux-android`, then install
  `target/dx/signin-dioxus/debug/android/app/app/build/outputs/apk/debug/app-debug.apk`
  with `adb install -r`. Needs `JAVA_HOME`, `ANDROID_HOME` and `NDK_HOME`.
* Web: `dx serve --platform web --features web --port 8080`, then open
  http://localhost:8080. That origin is allowed on the Web OAuth client.
* Tests: `cargo test --lib` (Rust) and `node --test tests/web-signin.test.mjs` (the GIS script).

## Results, 2026-10-28

| Gate | Result |
| --- | --- |
| 1. Empty Dioxus app builds on Windows and runs on the emulator | Pass (MSVC toolchain) |
| 2. Native account sheet appears, ID token reaches Rust, UI stays responsive | Pass |
| 3. Rust trades the ID token for a Supabase session | Pass: a Supabase user id and a 1387 byte access token came back |
| Web. GIS sign in on localhost gives the same Supabase user id | Button renders; sign in `___` (owner to click; the GIS popup opens outside the browser tools) |

Screenshot: `gate1-screen.png`. The gate 2 and 3 screenshots show the owner's
email, so they stay on the owner's PC.

## Flow

Android: Rust calls Kotlin through `#[manganis::ffi]`. The calls are synchronous
JNI, so Kotlin starts a coroutine on `Dispatchers.Main` and returns at once; Rust
polls every 300 ms (with a Tokio sleep) for at most 120 seconds, then cancels.
Kotlin asks for `GetGoogleIdOption` first and falls back to
`GetSignInWithGoogleOption` only on `NoCredentialException`.

Nonce: Rust makes 32 random bytes as hex (the raw nonce). Google gets the
SHA 256 hex of it; Supabase gets the raw value, hashes it and compares it with
the nonce inside the token. A token copied from elsewhere fails that check.

Supabase checks Google's signature and the audience (the Web client ID), then
finds or creates the user and returns `access_token`, `refresh_token` and `user`.
Neither token is logged. The UI shows only ids, email and lengths.

## Workarounds, every one

1. **MSVC toolchain on Windows.** The GNU toolchain fails on `windows-sys` with a
   MinGW `dlltool` error. `rust-toolchain.toml` pins `stable-x86_64-pc-windows-msvc`.
2. **Older Android libraries.** dx 0.7.10 generates a Kotlin 2.0.20 compiler,
   which cannot read libraries built with Kotlin 2.2 or later. Pinned:
   credentials 1.3.0, credentials-play-services-auth 1.3.0, googleid 1.1.1,
   kotlinx-coroutines-android 1.8.1. Revisit when dx moves to a newer Kotlin.
3. **Class loader fix (Dioxus issue #5629).** The constructor that manganis
   generates uses JNI `FindClass` from a native thread, which only sees system
   classes, so the app crashed with `ClassNotFoundException`. `new_plugin` in
   `src/signin.rs` loads the class through the activity's own class loader
   (about 20 lines).
4. **`compile_sdk = 35`** in `Dioxus.toml`, needed by Credential Manager.
5. **sdkmanager slash syntax.** The new sdkmanager wants `ndk/28.2.13676358`,
   not `ndk;28.2.13676358`.
6. **Software GPU on the emulator** (`-gpu swiftshader_indirect`) while the
   laptop's discrete GPU was off. Slow but works.

Not needed: the dx Windows linker backslash bug (#5847, #5976, #5661) did not
show up in debug builds. Release builds are untested (`___`).

## Known limits

* A sign in started before the emulator was paused for hours stayed on
  "Waiting..." after Google returned OK. A fresh run worked in about 10 seconds.
  The real app should retry when it comes back to the foreground.
* The app label shows "SigninDioxus" (from the crate name). The real app sets it.
* Only the debug SHA-1 is registered. A release keystore and its Android OAuth
  client are still needed.
* Not tried on a real phone yet.
* The Tauri 2 track was not built: ADR 0029 said to move only if Dioxus failed.

## Non Rust code

Counts include blank lines.

| File | Lines | Why |
| --- | ---: | --- |
| `src/android/src/main/java/.../GoogleSignin.kt` | 91 | Credential Manager has no Rust API |
| `src/android/build.gradle.kts` | 27 | Android library and its dependencies |
| `src/android/consumer-rules.pro` | 2 | Keeps the class name for JNI in release builds |
| `src/android/src/main/AndroidManifest.xml` | 1 | Empty library manifest |
| GIS script inside `src/web_signin.rs` | 33 | GIS is a JavaScript library |
| `tests/web-signin.test.mjs` | 70 | Tests the GIS script with stubs |

About 154 lines of non Rust app code, plus 70 lines of JavaScript tests.
Rust: 549 lines across `src/*.rs`, tests included.

## For the owner

`exchange` in `src/supabase.rs` was planned as your "you should type this"
moment and was written for you while you were out. Retype it yourself from the
comments when you have time.
