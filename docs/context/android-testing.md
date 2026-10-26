# Testing werdy on Android from a Windows PC

Checked on 2026-10-27 on the owner's PC, read only (nothing was installed). Feeds the Dioxus versus Tauri 2 spike (ADR 0002). Values marked `___` are not checked yet.

## What the PC already has

* Android Studio, with the Android SDK in `%LOCALAPPDATA%\Android\Sdk`: platform android-36, build tools 36.0.0, platform tools (adb), and the emulator.
* One emulator: `Pixel_6_API_34`, a phone, API 34, x86_64. No tablet emulator yet.
* Java: Android Studio's bundled JBR (OpenJDK 21).
* Node and npm, and Chrome.

## What is missing for the spike

Needed by both Dioxus and Tauri 2:

* Rust, installed with rustup, plus the four Android targets: `aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android`.
* The Android NDK and the cmdline tools, both from Android Studio's SDK Manager. Tauri's prerequisites suggest NDK 28 or later; the version chosen is `___`.
* Environment variables: `JAVA_HOME` (Android Studio's `jbr` folder), `ANDROID_HOME` (the SDK folder) and `NDK_HOME` (the NDK version folder).
* A tablet emulator (ADR 0022). Creating one downloads a system image, so it needs the owner's OK.

Per stack:

* Tauri 2: the Tauri CLI (`cargo tauri`, version `___`), then `cargo tauri android init` once and `cargo tauri android dev` to run. Source: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), partly verified.
* Dioxus: the `dx` CLI (Dioxus 0.7 line, patch version `___`), then `dx serve --platform android` with an emulator running or a phone connected. Source: [Dioxus mobile guide](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/).
* Not yet researched: Google sign in setup on Android (credential manager, SHA certificate) and on the web (OAuth client for localhost). The spike must cover both.

## Ways to test

* **Emulator, day to day.** Start `Pixel_6_API_34` from Android Studio's Device Manager. Inside the emulator, the PC's localhost is `10.0.2.2` ([Android docs](https://developer.android.com/studio/run/emulator-networking-address)).
* **A real phone or tablet over USB.** Turn on Developer options and USB debugging, check with `adb devices`, then `adb reverse tcp:PORT tcp:PORT` so the device can open `http://localhost:PORT` ([Android docs](https://developer.android.com/develop/ui/views/layout/webapps/access-local-server)). This is the best place for the final Google sign in check, because `localhost` counts as a secure origin and `10.0.2.2` may not.
* **Wireless, Android 11 and later.** Turn on Wireless debugging, then `adb pair IP:PORT` and `adb connect IP:PORT` (two different ports) on the same network ([adb docs](https://developer.android.com/tools/adb)).
* **The web build.** Chrome DevTools device mode on the PC for layout only; a real device through `chrome://inspect` for behaviour ([Chrome docs](https://developer.chrome.com/docs/devtools/remote-debugging)).
* **The design today.** No install needed: open the design canvas link in the browser on any phone or tablet.
