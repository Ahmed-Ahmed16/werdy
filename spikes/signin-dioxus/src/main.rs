use dioxus::prelude::*;

#[cfg(target_os = "android")]
mod signin;

#[cfg(target_arch = "wasm32")]
mod web_signin;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let status = use_signal(|| "Ready.".to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let busy = use_signal(|| false);
    #[cfg(not(target_arch = "wasm32"))]
    let button_label = if cfg!(target_os = "android") {
        "Sign in with Google"
    } else {
        "Android only in this gate"
    };
    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    let credentials = use_signal(|| None::<signin_dioxus::Credentials>);
    #[cfg(target_arch = "wasm32")]
    let mut started = use_signal(|| false);

    #[cfg(target_arch = "wasm32")]
    let controls = rsx! {
        div {
            id: "gsi-button",
            onmounted: move |_| {
                if !started() {
                    started.set(true);
                    web_signin::begin(status, credentials);
                }
            },
        }
    };

    #[cfg(not(target_arch = "wasm32"))]
    let controls = rsx! {
        button {
            disabled: busy(),
            onclick: move |_| {
                #[cfg(target_os = "android")]
                signin::begin(status, busy, credentials);
                #[cfg(not(target_os = "android"))]
                {
                    let mut status = status;
                    status.set("Android only in this gate".to_string());
                }
            },
            "{button_label}"
        }
    };

    rsx! {
        h1 { "werdy sign in spike" }
        {controls}
        p { "{status}" }
    }
}
