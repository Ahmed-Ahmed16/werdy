//! The few things that differ between the browser and a native build:
//! where records are kept and how today is read.

use std::rc::Rc;

use werdy_core::{Error, WerdyDay};
use werdy_store::Store;

/// Opens the store for this platform: browser localStorage on the web, and an
/// in memory store elsewhere until a file or SQLite store exists.
pub fn open_store() -> Rc<dyn Store> {
    #[cfg(target_arch = "wasm32")]
    {
        Rc::new(web::LocalStore)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Rc::new(werdy_store::MemoryStore::default())
    }
}

/// Returns the current werdy day as the local calendar date.
// TODO(#11): a werdy day ends at real Fajr (ADR 0010), not at midnight. Swap
// this for the calendar crate's moment to werdy day function.
pub fn today() -> Result<WerdyDay, Error> {
    #[cfg(target_arch = "wasm32")]
    {
        let now = js_sys::Date::new_0();
        let year = i32::try_from(now.get_full_year()).map_err(|_| Error::InvalidDate)?;
        let month = u8::try_from(now.get_month() + 1).map_err(|_| Error::InvalidDate)?;
        let day = u8::try_from(now.get_date()).map_err(|_| Error::InvalidDate)?;
        WerdyDay::new(year, month, day)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        const SECONDS_PER_DAY: u64 = 86_400;
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::InvalidDate)?
            .as_secs();
        let days = i64::try_from(seconds / SECONDS_PER_DAY).map_err(|_| Error::InvalidDate)?;
        WerdyDay::from_unix_days(days)
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use werdy_store::{OwnerData, Store, StoreError, from_json, to_json};

    /// Key under which the records live in localStorage.
    const KEY: &str = "werdy.records";

    /// Store backed by the browser's localStorage.
    pub struct LocalStore;

    fn storage() -> Result<web_sys::Storage, StoreError> {
        web_sys::window()
            .ok_or_else(|| StoreError::Backend("no window".to_owned()))?
            .local_storage()
            .map_err(|error| StoreError::Backend(format!("{error:?}")))?
            .ok_or_else(|| StoreError::Backend("no localStorage".to_owned()))
    }

    impl Store for LocalStore {
        fn load(&self) -> Result<Option<OwnerData>, StoreError> {
            let text = storage()?
                .get_item(KEY)
                .map_err(|error| StoreError::Backend(format!("{error:?}")))?;
            text.as_deref().map(from_json).transpose()
        }

        fn save(&self, data: &OwnerData) -> Result<(), StoreError> {
            storage()?
                .set_item(KEY, &to_json(data)?)
                .map_err(|error| StoreError::Backend(format!("{error:?}")))
        }
    }
}
