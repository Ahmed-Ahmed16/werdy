use dioxus::prelude::*;
use signin_dioxus::{Credentials, config::WEB_CLIENT_ID, nonce, supabase, token};

pub fn begin(mut status: Signal<String>, mut credentials: Signal<Option<Credentials>>) {
    status.set("Choose a Google account.".to_string());
    spawn(async move {
        match google_credentials().await {
            Ok(saved) => {
                status.set(format!("{} | Asking Supabase...", display_message(&saved)));
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
            Err(message) => status.set(format!("Google failed: {message}")),
        }
    });
}

fn display_message(saved: &Credentials) -> String {
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

async fn google_credentials() -> Result<Credentials, String> {
    let nonce = nonce::generate()?;
    let client_id = serde_json::to_string(WEB_CLIENT_ID)
        .map_err(|_| "Could not encode Google client ID".to_string())?;
    let nonce_hash = serde_json::to_string(&nonce.hash)
        .map_err(|_| "Could not encode Google nonce".to_string())?;
    let script =
        format!("const clientId = {client_id};\nconst nonce = {nonce_hash};\n{GOOGLE_SCRIPT}");
    let mut eval = dioxus::document::eval(&script);
    let id_token = eval
        .recv::<String>()
        .await
        .map_err(|_| "Could not receive Google credential".to_string())?;
    if let Some(message) = id_token.strip_prefix("GIS error: ") {
        return Err(message.to_string());
    }
    Ok(Credentials {
        id_token,
        raw_nonce: nonce.raw,
    })
}

const GOOGLE_SCRIPT: &str = r#"
try {
    if (!globalThis.google?.accounts?.id) {
        globalThis.werdyGisReady ??= new Promise((resolve, reject) => {
            const script = document.createElement("script");
            script.src = "https://accounts.google.com/gsi/client";
            script.async = true;
            script.onload = resolve;
            script.onerror = () => reject(new Error("Google script failed to load"));
            document.head.appendChild(script);
        });
        await globalThis.werdyGisReady;
    }
    const container = document.getElementById("gsi-button");
    if (!container) throw new Error("Google button is missing");
    // Keep the eval channel alive until GIS calls back.
    await new Promise((resolve) => {
        google.accounts.id.initialize({
            client_id: clientId,
            nonce,
            callback: (reply) => {
                if (typeof reply?.credential === "string" && reply.credential.length > 0) {
                    dioxus.send(reply.credential);
                } else {
                    dioxus.send("GIS error: Google returned no credential");
                }
                resolve();
            }
        });
        google.accounts.id.renderButton(container, { theme: "outline", size: "large" });
    });
} catch {
    dioxus.send("GIS error: Could not load or initialize Google sign in");
}
"#;
