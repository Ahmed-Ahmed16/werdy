use super::{Nonce, Reply};
use dioxus::document::{Eval, eval};

struct Cleanup(Eval);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

pub async fn credentials() -> Result<(String, String), ()> {
    let nonce = Nonce::generate()?;
    let client = serde_json::to_string(super::config::WEB_CLIENT_ID).map_err(|_| ())?;
    let hash = serde_json::to_string(&nonce.hash).map_err(|_| ())?;
    let script = format!(
        "const clientId={client}; const nonce={hash};\n{}",
        include_str!("google.js")
    );
    let mut channel = eval(&script);
    let _cleanup = Cleanup(channel);
    match channel.recv::<Reply>().await.map_err(|_| ())? {
        Reply::Ok { id_token } if !id_token.is_empty() => Ok((id_token, nonce.raw)),
        _ => Err(()),
    }
}
