use dioxus::prelude::*;
use manganis::jni::JNIEnv;
use manganis::jni::objects::{GlobalRef, JClass, JObject, JValue};
use signin_dioxus::{Credentials, Reply, config::WEB_CLIENT_ID, nonce, reply, supabase, token};
use std::time::{Duration, Instant};

const PLUGIN_CLASS: &str = "com.ahmedhossam.werdy.signin.GoogleSignin";

#[manganis::ffi("src/android")]
unsafe extern "Kotlin" {
    pub type GoogleSignin;
    pub fn start(this: &GoogleSignin, web_client_id: String, nonce: String) -> String;
    pub fn poll(this: &GoogleSignin) -> String;
    pub fn cancel(this: &GoogleSignin) -> String;
}

pub fn begin(
    mut status: Signal<String>,
    mut busy: Signal<bool>,
    mut credentials: Signal<Option<Credentials>>,
) {
    if busy() {
        return;
    }
    busy.set(true);
    credentials.set(None);
    status.set("Waiting for Google account sheet...".to_string());
    spawn(async move {
        match sign_in().await {
            Ok(saved) => {
                status.set(format!("{} | Asking Supabase...", success_message(&saved)));
                match supabase::exchange(&saved).await {
                    Ok(session) => status.set(format!(
                        "Supabase user id: {} | email: {} | access token length: {}",
                        session.user_id,
                        session.email.as_deref().unwrap_or("___"),
                        session.access_token.len()
                    )),
                    Err(message) => status.set(format!("Supabase failed: {message}")),
                }
                credentials.set(Some(saved));
            }
            Err(message) => status.set(message),
        }
        busy.set(false);
    });
}

fn success_message(saved: &Credentials) -> String {
    match token::display_claims(&saved.id_token) {
        Ok(claims) => format!(
            "Display only, token not verified. Email: {} | sub: {} | token length: {}",
            claims.email.as_deref().unwrap_or("___"),
            claims.sub,
            saved.id_token.len()
        ),
        Err(message) => format!("Token received ({} bytes). {message}", saved.id_token.len()),
    }
}

async fn sign_in() -> Result<Credentials, String> {
    let nonce = nonce::generate()?;
    let plugin = new_plugin()?;
    let initial = start(&plugin, WEB_CLIENT_ID.to_string(), nonce.hash)?;
    let outcome = wait_for_token(&plugin, initial).await;
    let cleanup = cancel(&plugin);
    let id_token = outcome?;
    cleanup?;
    Ok(Credentials {
        id_token,
        raw_nonce: nonce.raw,
    })
}

/// Workaround for Dioxus issue #5629. The generated `GoogleSignin::new()` uses JNI
/// `FindClass`, which on a native thread sees only system classes, so the app crashes
/// with `ClassNotFoundException`. Here the class comes from the activity's own class loader.
fn new_plugin() -> Result<GoogleSignin, String> {
    manganis::android::with_activity(|env, activity| {
        let built = construct(env, activity);
        if built.is_err() {
            // A failed JNI call leaves a pending Java exception; clear it so it cannot crash the app later.
            let _ = env.exception_clear();
        }
        Some(
            built
                .map(GoogleSignin::from_global_ref)
                .map_err(|error| format!("Could not create GoogleSignin: {error}")),
        )
    })
    .unwrap_or_else(|| Err("No Android activity available".to_string()))
}

fn construct(
    env: &mut JNIEnv<'_>,
    activity: &JObject<'_>,
) -> manganis::jni::errors::Result<GlobalRef> {
    let loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
        .l()?;
    let name = env.new_string(PLUGIN_CLASS)?;
    let class = env
        .call_method(
            &loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&name)],
        )?
        .l()?;
    let instance = env.new_object(
        JClass::from(class),
        "(Landroid/app/Activity;)V",
        &[JValue::Object(activity)],
    )?;
    env.new_global_ref(instance)
}

async fn wait_for_token(plugin: &GoogleSignin, mut json: String) -> Result<String, String> {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        match reply(&json)? {
            Reply::Ok { id_token } => return Ok(id_token),
            Reply::Error { kind, message } => return Err(format!("{kind}: {message}")),
            Reply::Pending => {}
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("Timeout: Google sign in exceeded 120 seconds".to_string());
        }
        tokio::time::sleep(remaining.min(Duration::from_millis(300))).await;
        json = poll(plugin)?;
    }
}
