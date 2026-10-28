# werdy app

## App name and icons

From the repo root on Windows, run:

```powershell
./scripts/make-icons.ps1
```

PowerShell and Windows System.Drawing are the only requirements. Commit the
generated files with the source. The script reads `assets/icon/icon.svg` and
checks its green and white against `assets/tokens.css`. It renders the SVG's
circles and polyline, with smoothing, into the web PNGs and Android launcher
PNGs at 48, 72, 96, 144 and 192 px. It also copies the SVG to the web favicon
and makes the Android vector foreground and solid background. Keep source
artwork within the central circle of radius 33 in the 108 unit canvas so
Android launcher masks cannot cut it off. This artwork fits within radius 27.
The renderer supports this SVG's rect, circles and polyline only. Extend it
before adding other SVG shapes.

`[application].name` in `Dioxus.toml` is the name source for this script. It
generates `assets/icon/AndroidManifest.xml` with one application label; the
launcher activity inherits it. Rerun the script after changing the name.
The existing `app-name` Fluent messages use the same brand in both languages.

In dx 0.7.10, `bundled_app_name()` uses the Cargo binary name in PascalCase,
so `werdy-app` becomes `WerdyApp`. The default Android manifest gives both
application and activity `@string/app_name`, populated from that binary name.
Neither `[application].name` nor `[bundle]` changes this Android label.
`[application].android_manifest` selects our complete manifest instead. It
keeps dx's native activity, `main` library entry point, theme and network
settings. Reminder permissions and receivers still come from the existing
Android library manifest through Gradle merging. Future manifest settings in
`Dioxus.toml` must also be reflected in our generator because dx copies custom
manifests verbatim.

`[bundle].icon` names the PNG for bundlers that use it. Android in dx 0.7.10
writes stock `ic_launcher` resources without reading that list. Our manifest
instead selects `werdy_launcher` from the existing Android library: an adaptive
icon on API 26 and above, with vector foreground and token green background,
and PNG fallbacks on API 24 and 25. Unique resource names avoid collisions
with dx's stock resources. Check the merged manifest, launcher label and
masked icon when building Android. Recheck this workaround after upgrading dx.

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
