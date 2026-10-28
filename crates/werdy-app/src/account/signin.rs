use super::{AccountStore, config};
#[cfg(any(test, target_os = "android", target_arch = "wasm32"))]
use sha2::{Digest, Sha256};
use std::rc::Rc;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_arch = "wasm32")]
mod web;

pub fn random_hex<const N: usize>() -> Result<String, ()> {
    let mut bytes = [0; N];
    getrandom::getrandom(&mut bytes).map_err(|_| ())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(any(test, target_os = "android", target_arch = "wasm32"))]
pub struct Nonce {
    pub raw: String,
    pub hash: String,
}
#[cfg(any(test, target_os = "android", target_arch = "wasm32"))]
impl Nonce {
    pub fn generate() -> Result<Self, ()> {
        let raw = random_hex::<32>()?;
        let hash = Sha256::digest(raw.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(Self { raw, hash })
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
#[cfg(any(test, target_os = "android", target_arch = "wasm32"))]
pub enum Reply {
    Pending,
    Ok {
        #[serde(rename = "idToken")]
        id_token: String,
    },
    Error,
}

struct LoginGuard(Rc<AccountStore>);
impl Drop for LoginGuard {
    fn drop(&mut self) {
        self.0.account.borrow_mut().signing_in = false;
    }
}

pub async fn begin(store: Rc<AccountStore>) {
    if !store.enabled || store.account.borrow().signing_in {
        return;
    }
    store.account.borrow_mut().signing_in = true;
    store.account.borrow_mut().error = None;
    let generation = store.account.borrow().generation;
    let _guard = LoginGuard(store.clone());
    let outcome = credentials().await;
    let outcome = match outcome {
        Ok((token, nonce)) => {
            match werdy_sync::SupabaseAuth::new(config::SUPABASE_URL, config::SUPABASE_KEY) {
                Ok(auth) => auth
                    .exchange(&token, &nonce, crate::platform::change_time() / 1000)
                    .await
                    .map_err(|_| "account-signin-error"),
                Err(_) => Err("account-signin-error"),
            }
        }
        Err(()) => Err("account-signin-error"),
    };
    if store.account.borrow().generation != generation {
        return;
    }
    match outcome {
        Ok(session) => {
            use werdy_store::Store;
            let saved = store
                .load()
                .and_then(|records| store.sign_in(session, &records.unwrap_or_default()));
            if saved.is_err() {
                store.account.borrow_mut().error = Some("account-storage-error");
            }
        }
        Err(key) => store.account.borrow_mut().error = Some(key),
    }
}

async fn credentials() -> Result<(String, String), ()> {
    #[cfg(target_os = "android")]
    {
        android::credentials().await
    }
    #[cfg(target_arch = "wasm32")]
    {
        web::credentials().await
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        Err(())
    }
}
