# werdy

A daily deeds tracker that gets the numbers right.

Track your daily Islamic deeds and life work, mark each one done, half done or missed, and see clear weekly and monthly totals that only count what was actually due.

**Status:** a first web app shows today's sheet and saves marks in the browser. See [issue #1](https://github.com/Ahmed-Ahmed16/werdy/issues/1) for the plan.

## Run it

You need Rust, the wasm target and the Dioxus CLI 0.7:

```sh
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10 --locked
```

Then serve the web app locally and open the address it prints:

```sh
cd crates/werdy-app
dx serve --platform web
```

`dx build --platform web` writes a static build to `target/dx/werdy-app/debug/web/public`. Marks are saved in the browser's local storage on this device only.

Run the checks with `cargo test --workspace`.

## Install the web app

Live address: [ahmed-ahmed16.github.io/werdy/](https://ahmed-ahmed16.github.io/werdy/) after the first deployment from `main`. In the GitHub repo settings, select GitHub Actions as the Pages source. Open the site once while online; later visits can load the app shell offline. Marks remain on this device.

On Android, open the address in Chrome, then use the browser menu and choose **Install app** or **Add to Home screen**. On iPhone, open it in Safari, tap **Share**, then **Add to Home Screen**.

## Why

Existing apps mark a deed as done or not done, and count every deed on every day, even a Friday deed on a Tuesday. werdy fixes that:

* Three marks per deed: green (done), yellow (partly done), red (missed).
* Each deed has a schedule, and only counts on days it is due.
* Today shows as in progress, not missed.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option. Copyright (c) 2026 Ahmed Hossam.

Unless you state otherwise, any contribution you submit for inclusion is dual licensed as above, without any additional terms.
