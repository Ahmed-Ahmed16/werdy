# werdy app

## Browser tests

From the repo root, install the runner version matching `wasm-bindgen` in
`Cargo.lock` (currently `0.2.129`). Put Chrome and its matching `chromedriver`
on `PATH`. In PowerShell:

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
$env:CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER='wasm-bindgen-test-runner'
$env:CHROMEDRIVER=(Get-Command chromedriver).Source
cargo test --locked -p werdy-app --bin werdy-app --target wasm32-unknown-unknown
```

Tests select `run_in_browser`; the runner uses headless Chrome. Geolocation
is stubbed; timers and localStorage use the browser. Native workspace tests
need no runner. See the [runner guide](https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/browsers.html#appendix-testing-in-headless-browsers-without-wasm-pack).

## Release build

Use PowerShell 7, JDK 17 (`keytool` on `PATH`), the Android SDK and NDK,
and dx 0.7.10. Run the commands below from the repo root.

**You should type this:** create your release key yourself. Keep it outside
every checkout, at `$HOME/.android/werdy-release.jks`. Do this once:

```powershell
New-Item -ItemType Directory -Path "$HOME/.android" -Force | Out-Null
keytool -genkeypair -v -keystore "$HOME/.android/werdy-release.jks" -storetype JKS -alias werdy -keyalg RSA -keysize 4096 -validity 10000
```

Answer the identity prompts. Save the store password and key password in your
password manager. Pressing Enter at the key password prompt uses the store
password for both. No password belongs in a command, shell history, or git.
See the [keytool reference](https://docs.oracle.com/en/java/javase/17/docs/specs/man/keytool.html).

Keep an encrypted backup of the keystore on a separate drive or trusted backup
service, outside the repo. Store its recovery password separately. Check that
the backup can be restored before distributing the app. Keep this same key for
future APK updates.

Set the signing inputs for this shell only. These prompts hide the passwords
and keep them out of shell history:

```powershell
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-msvc'
$env:WERDY_ANDROID_KEYSTORE = "$HOME/.android/werdy-release.jks"
$env:WERDY_ANDROID_STORE_PASSWORD = Read-Host 'Keystore password' -MaskInput
$env:WERDY_ANDROID_KEY_PASSWORD = Read-Host 'Key password for alias werdy' -MaskInput
```

Build both signed APK and AAB:

```powershell
try {
    ./scripts/build-android-release.ps1
} finally {
    Remove-Item Env:WERDY_ANDROID_STORE_PASSWORD -ErrorAction SilentlyContinue
    Remove-Item Env:WERDY_ANDROID_KEY_PASSWORD -ErrorAction SilentlyContinue
}
```

To build only one format, use `./scripts/build-android-release.ps1 -PackageType apk`
or `./scripts/build-android-release.ps1 -PackageType aab` with the same env vars.
Outputs go to `target/android-release/`: `app-release.apk` and `werdy-*.aab`.
Existing outputs are overwritten only when dx copies a completed bundle.
After a failed build, old outputs may still exist. Do not distribute them as
the new release.

The wrapper runs this dx command, with an absolute output path:

```powershell
dx bundle --platform android --release --package werdy-app --no-default-features --features mobile --package-types apk --package-types aab --out-dir target/android-release
```

Use the wrapper, since dx 0.7.10 reads signing fields only from config.
Its [Android build source](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/src/build/android.rs)
selects `assembleRelease` only when release mode and `[bundle.android]` are
both present. AAB uses `bundleRelease`. Its
[Gradle template](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/assets/android/gen/app/build.gradle.kts.hbs)
puts `jks_file`, `jks_password`, `key_alias` and `key_password` directly into
Gradle strings. It does not expand env vars for these fields.

The wrapper saves the original config in `.android-signing/Dioxus.toml.backup`,
stages a copy of the key there, and temporarily appends `[bundle.android]` to
`crates/werdy-app/Dioxus.toml`. Passwords use Kotlin Unicode escapes inside TOML
literal strings, so quotes, dollar signs and HTML characters stay literal.
The key path works with the template's fixed seven parent directory prefix
because the wrapper forces `CARGO_TARGET_DIR` to `target-release-signing/`.
It also sets an isolated `GRADLE_USER_HOME` and disables the Gradle daemon.
In `finally`, it restores the original config and environment and removes the
staged key, generated Gradle config and caches. Each run rebuilds in a fresh
target folder. No signing values are committed to `Dioxus.toml`.

Do not edit the config, run another build, or stage files in this worktree
while the wrapper runs. A forced process stop or power loss can skip cleanup.
If the wrapper reports `Signing workspace exists`, first ensure the previous
build and Gradle processes have stopped. Restore
`.android-signing/Dioxus.toml.backup` over `crates/werdy-app/Dioxus.toml` if the
backup exists. Then remove `.android-signing/` and `target-release-signing/`
from this worktree only. Check `git diff -- crates/werdy-app/Dioxus.toml` is
empty before retrying. Never stage the temporary signing config.

Read the release certificate fingerprint, entering the store password at the
prompt:

```powershell
keytool -list -v -keystore "$HOME/.android/werdy-release.jks" -alias werdy
```

**Owner steps:** copy the `SHA1` certificate fingerprint, then:

1. Open the existing Google Cloud project used for werdy sign in. In Google
   Auth Platform, open Clients, then Create client. Select Android.
2. Name it `werdy release`. Set Package name to `com.ahmedhossam.werdy` and
   paste the release fingerprint into the `SHA-1` field. Create the client
   and copy its client ID. Keep the existing debug client.
3. Open the Supabase project dashboard, Authentication, Sign In / Providers,
   Google. Append the new Android client ID to the allowed Client IDs list,
   separated by a comma. Keep the existing Web client ID first, and preserve
   the existing debug client ID and Web client secret. Save.
4. The Credential Manager bridge still uses the Web client ID as its server
   client ID. Do not replace it with the Android client ID. Test Google sign
   in on an installed release APK after completing these owner steps.

See [Google's client setup](https://codelabs.developers.google.com/sign-in-with-google-android)
and [Supabase's Google provider guide](https://supabase.com/docs/guides/auth/social-sign-in/auth-google).
If Google Play signs delivered APKs with a different app signing key, register
that certificate fingerprint too. Find it under App integrity in Play Console;
the local upload key fingerprint alone does not cover those APKs. See
[Google's certificate guide](https://developers.google.com/android/guides/client-auth).

Check the wrapper without dx or a keystore:

```powershell
pwsh -NoProfile -File scripts/test-android-release.ps1
```

Real release packaging, signatures and release sign in remain unverified until
the owner creates the key and runs the build and device checks.
