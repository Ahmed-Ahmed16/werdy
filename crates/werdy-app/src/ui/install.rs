//! Installing werdy from the browser (#86): Chrome's install prompt, or the
//! Share steps on iPhone, where Safari has no prompt.

#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

/// What the page learns from the browser.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
pub struct BrowserInstall {
    /// Already running as an installed app.
    pub standalone: bool,
    pub ios: bool,
    /// Chrome offered an install prompt that the page kept for later.
    pub prompt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallHint {
    Hidden,
    Prompt,
    IosSteps,
}

pub fn install_hint(browser: BrowserInstall) -> InstallHint {
    if browser.standalone {
        InstallHint::Hidden
    } else if browser.prompt {
        InstallHint::Prompt
    } else if browser.ios {
        InstallHint::IosSteps
    } else {
        InstallHint::Hidden
    }
}

/// `index.html` keeps Chrome's `beforeinstallprompt` event in
/// `window.werdyInstall`, since it can fire before the app loads.
#[cfg(target_arch = "wasm32")]
const CHECK: &str = r#"
const ua = navigator.userAgent;
return {
  standalone: matchMedia("(display-mode: standalone)").matches || navigator.standalone === true,
  ios: /iPhone|iPad|iPod/.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1),
  prompt: !!window.werdyInstall,
};
"#;

#[cfg(target_arch = "wasm32")]
const PROMPT: &str = r#"
const event = window.werdyInstall;
window.werdyInstall = null;
if (!event) return true;
try { await event.prompt(); await event.userChoice; return true; } catch (_) { return false; }
"#;

/// Resolves when index.html keeps a prompt that Chrome offered after mount.
#[cfg(target_arch = "wasm32")]
const OFFERED: &str = r#"
await new Promise((done) => window.addEventListener("werdyinstall", done, { once: true }));
return true;
"#;

#[cfg(target_arch = "wasm32")]
async fn check() -> BrowserInstall {
    dioxus::document::eval(CHECK)
        .join::<BrowserInstall>()
        .await
        .unwrap_or_default()
}

/// Whether there is anything to show: an install path, or the note that the
/// prompt failed, which stays even after the prompt is spent.
pub fn has_something_to_show(hint: InstallHint, failed: bool) -> bool {
    failed || hint != InstallHint::Hidden
}

/// Install state shared by the Settings card and the sheet banner.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq)]
pub struct Install {
    browser: Signal<BrowserInstall>,
    failed: Signal<bool>,
}

#[cfg(target_arch = "wasm32")]
impl Install {
    fn hint(&self) -> InstallHint {
        install_hint((self.browser)())
    }

    fn visible(&self) -> bool {
        has_something_to_show(self.hint(), (self.failed)())
    }

    async fn prompt(mut self) {
        let shown = dioxus::document::eval(PROMPT).join::<bool>().await;
        self.failed.set(!matches!(shown, Ok(true)));
        self.browser.set(check().await);
    }
}

#[cfg(target_arch = "wasm32")]
fn use_install() -> Install {
    let mut browser = use_signal(BrowserInstall::default);
    let failed = use_signal(|| false);
    use_future(move || async move {
        loop {
            browser.set(check().await);
            if dioxus::document::eval(OFFERED).await.is_err() {
                break;
            }
        }
    });
    Install { browser, failed }
}

/// Purpose line plus the Install button, the iPhone steps, or the failure note.
#[cfg(target_arch = "wasm32")]
#[component]
fn InstallBody(install: Install) -> Element {
    let i18n = super::context::use_i18n();
    if (install.failed)() {
        // The prompt is spent once it fails, so only the browser menu is left.
        return rsx! { p { role: "alert", {i18n.t("install-failed")} } };
    }
    rsx! {
        p { {i18n.t("install-purpose")} }
        match install.hint() {
            InstallHint::Prompt => rsx! {
                button { class: "secondary-button",
                    onclick: move |_| install.prompt(),
                    {i18n.t("install-button")}
                }
            },
            InstallHint::IosSteps => rsx! { p { {i18n.t("install-ios-steps")} } },
            InstallHint::Hidden => rsx! {},
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn InstallSection() -> Element {
    let i18n = super::context::use_i18n();
    let install = use_install();
    if !install.visible() {
        return rsx! {};
    }
    rsx! {
        section { class: "settings-card", aria_label: i18n.t("install-title"),
            h2 { {i18n.t("install-title")} }
            InstallBody { install }
        }
    }
}

/// Closing the banner is remembered in this browser only. Storage can be
/// blocked (private mode); then the banner simply shows again next visit.
#[cfg(target_arch = "wasm32")]
const BANNER_CLOSED_KEY: &str = "werdy-install-banner-closed";

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// Install banner on the daily sheet, so people find it without opening Settings.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn InstallBanner() -> Element {
    let i18n = super::context::use_i18n();
    let install = use_install();
    let mut closed = use_signal(|| {
        local_storage()
            .is_some_and(|storage| matches!(storage.get_item(BANNER_CLOSED_KEY), Ok(Some(_))))
    });
    if closed() || !install.visible() {
        return rsx! {};
    }
    rsx! {
        section { class: "install-banner", aria_label: i18n.t("install-title"),
            div { class: "install-banner-text",
                strong { {i18n.t("install-title")} }
                InstallBody { install }
            }
            button { class: "install-banner-close", aria_label: i18n.t("install-close"),
                onclick: move |_| {
                    if let Some(storage) = local_storage() {
                        // Best effort: a blocked store only means it shows again later.
                        let _ = storage.set_item(BANNER_CLOSED_KEY, "1");
                    }
                    closed.set(true);
                },
                "\u{00D7}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_browser_gets_the_install_path_it_supports() {
        let browser = |standalone, ios, prompt| BrowserInstall {
            standalone,
            ios,
            prompt,
        };
        let cases = [
            (
                "installed app",
                browser(true, true, true),
                InstallHint::Hidden,
            ),
            (
                "Chrome prompt",
                browser(false, false, true),
                InstallHint::Prompt,
            ),
            (
                "iPhone Safari",
                browser(false, true, false),
                InstallHint::IosSteps,
            ),
            (
                "other browser",
                browser(false, false, false),
                InstallHint::Hidden,
            ),
        ];
        for (name, input, expected) in cases {
            assert_eq!(install_hint(input), expected, "{name}");
        }
    }

    #[test]
    fn failed_prompt_note_stays_after_the_prompt_is_spent() {
        assert!(has_something_to_show(InstallHint::Hidden, true));
        assert!(has_something_to_show(InstallHint::IosSteps, false));
        assert!(!has_something_to_show(InstallHint::Hidden, false));
    }
}
