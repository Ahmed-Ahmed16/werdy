//! Installing werdy from the browser (#86): Chrome's install prompt, or the
//! Share steps on iPhone, where Safari has no prompt.

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

#[cfg(target_arch = "wasm32")]
#[dioxus::prelude::component]
pub fn InstallSection() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    let i18n = super::context::use_i18n();
    let mut browser = use_signal(BrowserInstall::default);
    let mut failed = use_signal(|| false);
    use_future(move || async move {
        loop {
            browser.set(check().await);
            if dioxus::document::eval(OFFERED).await.is_err() {
                break;
            }
        }
    });
    if failed() {
        // The prompt is spent once it fails, so only the browser menu is left.
        return rsx! {
            section { class: "settings-card", aria_label: i18n.t("install-title"),
                h2 { {i18n.t("install-title")} }
                p { role: "alert", {i18n.t("install-failed")} }
            }
        };
    }
    match install_hint(browser()) {
        InstallHint::Hidden => rsx! {},
        InstallHint::Prompt => rsx! {
            section { class: "settings-card", aria_label: i18n.t("install-title"),
                h2 { {i18n.t("install-title")} }
                p { {i18n.t("install-purpose")} }
                button { class: "secondary-button",
                    onclick: move |_| async move {
                        let shown = dioxus::document::eval(PROMPT).join::<bool>().await;
                        failed.set(!matches!(shown, Ok(true)));
                        browser.set(check().await);
                    },
                    {i18n.t("install-button")}
                }
            }
        },
        InstallHint::IosSteps => rsx! {
            section { class: "settings-card", aria_label: i18n.t("install-title"),
                h2 { {i18n.t("install-title")} }
                p { {i18n.t("install-purpose")} }
                p { {i18n.t("install-ios-steps")} }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_app_shows_no_hint() {
        let browser = BrowserInstall {
            standalone: true,
            ios: true,
            prompt: true,
        };
        assert_eq!(install_hint(browser), InstallHint::Hidden);
    }

    #[test]
    fn chrome_prompt_gets_a_button_and_iphone_gets_the_share_steps() {
        let chrome = BrowserInstall {
            prompt: true,
            ..BrowserInstall::default()
        };
        let iphone = BrowserInstall {
            ios: true,
            ..BrowserInstall::default()
        };
        assert_eq!(install_hint(chrome), InstallHint::Prompt);
        assert_eq!(install_hint(iphone), InstallHint::IosSteps);
    }

    #[test]
    fn browser_without_a_prompt_or_steps_shows_nothing() {
        assert_eq!(install_hint(BrowserInstall::default()), InstallHint::Hidden);
    }
}
