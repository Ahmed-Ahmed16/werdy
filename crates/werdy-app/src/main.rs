//! The werdy app. Web first (ADR 0029); the same code later builds for Android.

#![forbid(unsafe_code)]

mod editor_data;
mod i18n;
mod location;
mod platform;
#[cfg(test)]
mod pwa_check;
mod starter;
mod state;
mod ui;

#[cfg(test)]
mod style_check;

fn main() {
    dioxus::launch(ui::App);
}
