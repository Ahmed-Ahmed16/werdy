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

/// Milliseconds from the local clock, used to order mark changes in browser tabs.
pub fn change_time() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now().max(0.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| {
                u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
            })
    }
}

/// Local hour and minute for the temporary time slot boundaries.
pub fn local_time() -> (u8, u8) {
    #[cfg(target_arch = "wasm32")]
    {
        let now = js_sys::Date::new_0();
        (now.get_hours() as u8, now.get_minutes() as u8)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        (0, 0)
    }
}

/// Scrolls a visible time slot into view in the browser.
pub fn scroll_slot(slot: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(&format!("slot-{slot}")))
    {
        element.scroll_into_view();
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = slot;
}

#[cfg(target_arch = "wasm32")]
pub use web::WatchHandle;

#[cfg(not(target_arch = "wasm32"))]
pub struct WatchHandle;

/// Watches for a new day and for changes saved by another browser tab.
pub fn watch_changes(
    on_clock: impl FnMut() + 'static,
    on_storage: impl FnMut() + 'static,
) -> WatchHandle {
    #[cfg(target_arch = "wasm32")]
    {
        web::watch_changes(on_clock, on_storage)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (on_clock, on_storage);
        WatchHandle
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::cell::RefCell;
    use std::rc::Rc;

    use wasm_bindgen::{JsCast, closure::Closure};
    use werdy_store::{OwnerData, Store, StoreError, from_json, to_json};

    /// Key under which the records live in localStorage.
    const KEY: &str = "werdy.records";

    pub struct WatchHandle {
        window: Option<web_sys::Window>,
        document: Option<web_sys::Document>,
        clock: Closure<dyn FnMut(web_sys::Event)>,
        storage: Closure<dyn FnMut(web_sys::StorageEvent)>,
        _timer: Closure<dyn FnMut()>,
        interval: Option<i32>,
    }

    impl Drop for WatchHandle {
        fn drop(&mut self) {
            if let Some(window) = &self.window {
                let _ = window.remove_event_listener_with_callback(
                    "focus",
                    self.clock.as_ref().unchecked_ref(),
                );
                let _ = window.remove_event_listener_with_callback(
                    "storage",
                    self.storage.as_ref().unchecked_ref(),
                );
                if let Some(interval) = self.interval {
                    window.clear_interval_with_handle(interval);
                }
            }
            if let Some(document) = &self.document {
                let _ = document.remove_event_listener_with_callback(
                    "visibilitychange",
                    self.clock.as_ref().unchecked_ref(),
                );
            }
        }
    }

    pub fn watch_changes(
        on_clock: impl FnMut() + 'static,
        mut on_storage: impl FnMut() + 'static,
    ) -> WatchHandle {
        let on_clock = Rc::new(RefCell::new(on_clock));
        let clock_event = on_clock.clone();
        let clock = Closure::wrap(
            Box::new(move |_: web_sys::Event| (clock_event.borrow_mut())()) as Box<dyn FnMut(_)>,
        );
        let storage = Closure::wrap(Box::new(move |event: web_sys::StorageEvent| {
            if event.key().as_deref() == Some(KEY) {
                on_storage();
            }
        }) as Box<dyn FnMut(_)>);
        let timer = Closure::wrap(Box::new(move || (on_clock.borrow_mut())()) as Box<dyn FnMut()>);
        let window = web_sys::window();
        let document = window.as_ref().and_then(web_sys::Window::document);
        let interval = window.as_ref().and_then(|window| {
            let _ =
                window.add_event_listener_with_callback("focus", clock.as_ref().unchecked_ref());
            let _ = window
                .add_event_listener_with_callback("storage", storage.as_ref().unchecked_ref());
            window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    timer.as_ref().unchecked_ref(),
                    60_000,
                )
                .ok()
        });
        if let Some(document) = &document {
            let _ = document.add_event_listener_with_callback(
                "visibilitychange",
                clock.as_ref().unchecked_ref(),
            );
        }
        WatchHandle {
            window,
            document,
            clock,
            storage,
            _timer: timer,
            interval,
        }
    }

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
