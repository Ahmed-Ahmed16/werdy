use super::context::use_i18n;
use crate::account::{
    runtime::{AccountHandle, AccountView},
    signin,
};
use crate::state::AppState;
use dioxus::prelude::*;
use fluent_bundle::FluentValue;

#[component]
pub fn AccountSection() -> Element {
    let i18n = use_i18n();
    let handle = use_context::<AccountHandle>();
    let status = use_context::<Signal<AccountView>>();
    let state = use_context::<Signal<AppState>>();
    let mut local_error = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    let sharing = try_use_context::<Signal<super::sharing::Sharing>>();
    if !handle.0.enabled {
        return rsx! {};
    }
    let account = status.read();
    let last_sync = account
        .last_sync
        .and_then(crate::account::runtime::display_time);
    let email = account
        .email
        .clone()
        .unwrap_or_else(|| i18n.t("account-email-unknown"));
    let signout_store = handle.0.clone();
    let delete_store = handle.0.clone();
    let (deleting, delete_error) = sharing
        .map(|sharing| (sharing.read().deleting, sharing.read().delete_error))
        .unwrap_or_default();
    rsx! {
        section { class: "settings-card", aria_label: i18n.t("account-title"),
            h2 { {i18n.t("account-title")} }
            if account.signed_in {
                p { dir: "auto", "{email}" }
                if let Some(error) = account.error {
                    p { role: "status", {i18n.t(error)} }
                } else if let Some(time) = last_sync {
                    p { role: "status", {i18n.t_with("account-last-sync",&[("time",FluentValue::from(time))])} }
                } else { p { role: "status", {i18n.t("account-waiting")} } }
                button { class: "secondary-button", onclick: move |_| {
                    let records = state.read().data.clone();
                    let signed_out = signout_store.sign_out(&records).is_ok();
                    local_error.set(!signed_out);
                    // Drop the old account's codes and watchers now, not at the next poll.
                    if let (true, Some(mut sharing)) = (signed_out, sharing) {
                        sharing.set(super::sharing::Sharing::default());
                    }
                },{i18n.t("account-signout")} }
                if let Some(sharing) = sharing {
                    if confirm_delete() {
                        div { class: "confirm-box", role: "alertdialog", aria_label: i18n.t("account-delete-confirm"),
                            p { {i18n.t("account-delete-confirm")} }
                            button { class: "danger-button", disabled: deleting, onclick: move |_| {
                                confirm_delete.set(false);
                                super::sharing::delete_account(sharing, state, delete_store.clone());
                            }, {i18n.t("account-delete-yes")} }
                            button { class: "secondary-button", onclick: move |_| confirm_delete.set(false), {i18n.t("editor-cancel")} }
                        }
                    } else {
                        button { class: "danger-button", disabled: deleting, onclick: move |_| confirm_delete.set(true), {i18n.t("account-delete")} }
                    }
                    if deleting { p { role: "status", {i18n.t("account-deleting")} } }
                    if let Some(error) = delete_error { p { role: "alert", {i18n.t(error)} } }
                }
            } else {
                p { {i18n.t("account-purpose")} }
                SignInButton {}
                if account.busy { p { role: "status", {i18n.t("account-signing-in")} } }
                if let Some(error) = account.error { p { role: "status", {i18n.t(error)} } }
            }
            if *local_error.read() { p { role: "alert", {i18n.t("account-storage-error")} } }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn GoogleButton() -> Element {
    let handle = use_context::<AccountHandle>();
    rsx! { div { id: "werdy-google-signin", onmounted: move |_| {
        let store = handle.0.clone(); spawn(async move { signin::begin(store).await; });
    } } }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn SignInButton() -> Element {
    let i18n = use_i18n();
    let status = use_context::<Signal<AccountView>>();
    let mut attempt = use_signal(|| 0_u64);
    rsx! {
        GoogleButton {key:"{attempt}"}
        if status.read().error.is_some() && !status.read().busy {
            button {class:"secondary-button",onclick:move |_| {
                let next=attempt().wrapping_add(1); attempt.set(next);
            },{i18n.t("account-signin")} }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
fn SignInButton() -> Element {
    let i18n = use_i18n();
    let handle = use_context::<AccountHandle>();
    let status = use_context::<Signal<AccountView>>();
    rsx! { button {class:"secondary-button",disabled:status.read().busy,onclick:move |_| {
        let store=handle.0.clone(); spawn(async move {signin::begin(store).await;});
    },{i18n.t("account-signin")} } }
}
