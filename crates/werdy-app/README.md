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
