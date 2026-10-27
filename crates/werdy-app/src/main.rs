//! The werdy app. Web first (ADR 0029); the same code later builds for Android.

#![forbid(unsafe_code)]

mod i18n;
mod platform;
mod starter;
mod state;
mod ui;

#[cfg(test)]
mod style_check;

fn main() {
    dioxus::launch(ui::App);
}
