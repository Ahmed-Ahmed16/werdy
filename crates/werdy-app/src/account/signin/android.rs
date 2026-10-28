use super::{Nonce, Reply};
use crate::platform::android_bridge as bridge;
use std::time::{Duration, Instant};

struct Request(bridge::GoogleSignin);
impl Drop for Request {
    fn drop(&mut self) {
        let _ = bridge::cancel(&self.0);
        bridge::clear_exception();
    }
}

pub async fn credentials() -> Result<(String, String), ()> {
    let nonce = Nonce::generate()?;
    let reference =
        bridge::new_plugin("com.ahmedhossam.werdy.signin.GoogleSignin").map_err(|_| ())?;
    let request = Request(bridge::GoogleSignin::from_global_ref(reference));
    let mut reply = bridge::start(
        &request.0,
        super::config::WEB_CLIENT_ID.to_owned(),
        nonce.hash,
    )
    .map_err(|_| {
        bridge::clear_exception();
    })?;
    let started = Instant::now();
    loop {
        match serde_json::from_str::<Reply>(&reply).map_err(|_| ())? {
            Reply::Ok { id_token } if !id_token.is_empty() => return Ok((id_token, nonce.raw)),
            Reply::Pending => {}
            _ => return Err(()),
        }
        if started.elapsed() >= Duration::from_secs(120) {
            return Err(());
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        reply = bridge::poll(&request.0).map_err(|_| {
            bridge::clear_exception();
        })?;
    }
}
