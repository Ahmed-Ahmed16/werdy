//! The few things that differ between the browser and a native build:
//! where records are kept and how today is read.

use std::rc::Rc;

use werdy_calendar::{Date, LocalTime, Method, Place, fajr};
use werdy_core::{Error, WerdyDay};
use werdy_store::Store;
use werdy_store::{Location, PrayerMethod, Settings};

#[cfg(any(test, target_arch = "wasm32"))]
const RECORDS_KEY: &str = "werdy.records";
#[cfg(any(test, target_arch = "wasm32"))]
const DEMO_RECORDS_KEY: &str = "werdy.demo.records";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataMode {
    Normal,
    Demo,
}

#[cfg(any(test, target_arch = "wasm32"))]
impl DataMode {
    fn key(self) -> &'static str {
        match self {
            Self::Normal => RECORDS_KEY,
            Self::Demo => DEMO_RECORDS_KEY,
        }
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
fn mode_from_query(query: &str) -> DataMode {
    if query
        .trim_start_matches('?')
        .split('&')
        .any(|part| part.split('=').next() == Some("demo"))
    {
        DataMode::Demo
    } else {
        DataMode::Normal
    }
}

/// Reads the query once when the root component starts.
pub fn data_mode() -> DataMode {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.location().search().ok())
            .map_or(DataMode::Normal, |query| mode_from_query(&query))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        DataMode::Normal
    }
}

/// Current app path without the demo query.
pub fn app_address() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.location().pathname().ok())
            .unwrap_or_else(|| "/".to_owned())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "/".to_owned()
    }
}

/// Opens the store for this platform: browser localStorage on the web, and an
/// in memory store elsewhere until a file or SQLite store exists.
pub fn open_store(mode: DataMode) -> Rc<dyn Store> {
    #[cfg(target_arch = "wasm32")]
    {
        Rc::new(web::LocalStore { key: mode.key() })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = mode;
        Rc::new(werdy_store::MemoryStore::default())
    }
}

/// Returns local calendar date when no location is saved.
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

fn method(method: PrayerMethod) -> Method {
    match method {
        PrayerMethod::EgyptianGeneralAuthority => Method::EgyptianGeneralAuthority,
        PrayerMethod::MuslimWorldLeague => Method::MuslimWorldLeague,
        PrayerMethod::UmmAlQura => Method::UmmAlQura,
    }
}

fn calendar_parts(
    day: WerdyDay,
    offset: i32,
    settings: &Settings,
) -> Result<Option<(Date, Place)>, Error> {
    let Some(location) = &settings.location else {
        return Ok(None);
    };
    let (year, month, date) = day.date_parts();
    let year = u16::try_from(year).map_err(|_| Error::InvalidDate)?;
    let calendar_date = Date::new(year, month, date).map_err(|_| Error::InvalidDate)?;
    let (latitude, longitude) = location.coordinates();
    let place = Place::new(latitude, longitude, offset).map_err(|_| Error::InvalidDate)?;
    Ok(Some((calendar_date, place)))
}

#[cfg(target_arch = "wasm32")]
fn browser_midnight() -> js_sys::Date {
    let now = js_sys::Date::new_0();
    js_sys::Date::new_with_year_month_day(
        now.get_full_year(),
        now.get_month() as i32,
        now.get_date() as i32,
    )
}

#[cfg(target_arch = "wasm32")]
fn browser_offset_at_fajr(midnight: &js_sys::Date, minutes: u16) -> i32 {
    let instant = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(
        midnight.get_time() + f64::from(minutes) * 60_000.0,
    ));
    -(instant.get_timezone_offset() as i32)
}

fn fajr_with_offset_lookup(
    day: WerdyDay,
    settings: &Settings,
    midnight_offset: i32,
    offset_at_fajr: impl Fn(u16) -> i32,
) -> Result<Option<u16>, Error> {
    let Some((date, place)) = calendar_parts(day, midnight_offset, settings)? else {
        return Ok(None);
    };
    let first = match fajr(date, place, method(settings.prayer_method)) {
        Ok(time) => time.minutes_since_midnight(),
        Err(werdy_calendar::Error::FajrDoesNotOccur) => return Ok(None),
        Err(_) => return Err(Error::InvalidDate),
    };
    let actual_offset = offset_at_fajr(first);
    if actual_offset == midnight_offset {
        return Ok(Some(first));
    }
    let place = Place::new(place.latitude(), place.longitude(), actual_offset)
        .map_err(|_| Error::InvalidDate)?;
    fajr(date, place, method(settings.prayer_method))
        .map(|time| Some(time.minutes_since_midnight()))
        .map_err(|_| Error::InvalidDate)
}

fn day_for_with_offset_lookup(
    day: WerdyDay,
    hour: u8,
    minute: u8,
    midnight_offset: i32,
    settings: &Settings,
    offset_at_fajr: impl Fn(u16) -> i32,
) -> Result<WerdyDay, Error> {
    LocalTime::new(hour, minute).map_err(|_| Error::InvalidDate)?;
    let Some(fajr_minutes) =
        fajr_with_offset_lookup(day, settings, midnight_offset, offset_at_fajr)?
    else {
        return Ok(day);
    };
    if u16::from(hour) * 60 + u16::from(minute) < fajr_minutes {
        day.previous()
    } else {
        Ok(day)
    }
}

/// Pure date boundary. Browser supplies local time and offset for this date.
#[cfg(any(test, not(target_arch = "wasm32")))]
pub fn day_for(
    day: WerdyDay,
    hour: u8,
    minute: u8,
    offset: i32,
    settings: &Settings,
) -> Result<WerdyDay, Error> {
    day_for_with_offset_lookup(day, hour, minute, offset, settings, |_| offset)
}

/// Browser local offset is read for the calendar date, including daylight saving.
pub fn today_with_settings(settings: &Settings) -> Result<WerdyDay, Error> {
    let day = today()?;
    #[cfg(target_arch = "wasm32")]
    {
        let now = js_sys::Date::new_0();
        let date = browser_midnight();
        let offset = -(date.get_timezone_offset() as i32);
        day_for_with_offset_lookup(
            day,
            now.get_hours() as u8,
            now.get_minutes() as u8,
            offset,
            settings,
            |minutes| browser_offset_at_fajr(&date, minutes),
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        day_for(day, 12, 0, 0, settings)
    }
}

/// Today's Fajr in local clock minutes, for the selected place.
pub fn fajr_today(settings: &Settings) -> Result<Option<u16>, Error> {
    let day = today()?;
    #[cfg(target_arch = "wasm32")]
    {
        let date = browser_midnight();
        let offset = -(date.get_timezone_offset() as i32);
        fajr_with_offset_lookup(day, settings, offset, |minutes| {
            browser_offset_at_fajr(&date, minutes)
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let offset = 0;
        fajr_with_offset_lookup(day, settings, offset, |_| offset)
    }
}

#[cfg(target_arch = "wasm32")]
const LOCATION_TIMEOUT_MS: u32 = 15_000;

/// Browser GPS is requested only after a tap.
pub fn request_location(
    on_success: impl FnOnce(Location) + 'static,
    on_error: impl FnOnce() + 'static,
) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let Some(window) = web_sys::window() else {
            on_error();
            return;
        };
        let Ok(geo) = window.navigator().geolocation() else {
            on_error();
            return;
        };
        let error_callback = std::rc::Rc::new(std::cell::RefCell::new(Some(on_error)));
        // Browser timeout excludes the permission prompt, so bound the full wait too.
        let timer_error = error_callback.clone();
        let timer = wasm_bindgen::closure::Closure::once(move || {
            if let Some(callback) = timer_error.borrow_mut().take() {
                callback();
            }
        });
        if window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                timer.as_ref().unchecked_ref(),
                LOCATION_TIMEOUT_MS as i32,
            )
            .is_err()
            && let Some(callback) = error_callback.borrow_mut().take()
        {
            callback();
            return;
        }
        timer.forget();
        let invalid_coordinates = error_callback.clone();
        let completed_error = error_callback.clone();
        let success =
            wasm_bindgen::closure::Closure::once_into_js(move |position: web_sys::Position| {
                let coordinates = position.coords();
                match Location::new(None, None, coordinates.latitude(), coordinates.longitude()) {
                    Some(location) => {
                        let _ = completed_error.borrow_mut().take();
                        on_success(location);
                    }
                    None => {
                        if let Some(callback) = invalid_coordinates.borrow_mut().take() {
                            callback();
                        }
                    }
                }
            });
        let geolocation_error = error_callback.clone();
        let failure =
            wasm_bindgen::closure::Closure::once_into_js(move |_: web_sys::PositionError| {
                if let Some(callback) = geolocation_error.borrow_mut().take() {
                    callback();
                }
            });
        let options = web_sys::PositionOptions::new();
        options.set_timeout(LOCATION_TIMEOUT_MS);
        if geo
            .get_current_position_with_error_callback_and_options(
                success.unchecked_ref(),
                Some(failure.unchecked_ref()),
                &options,
            )
            .is_err()
            && let Some(callback) = error_callback.borrow_mut().take()
        {
            callback();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = on_success;
        on_error();
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

/// Distinguishes a reset from stale browser tabs that still hold old demo marks.
pub fn demo_generation() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        format!("{}-{}", change_time(), js_sys::Math::random())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT: AtomicU64 = AtomicU64::new(0);
        format!("{}-{}", change_time(), NEXT.fetch_add(1, Ordering::Relaxed))
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

/// Restores the top after a new screen has rendered its content.
pub fn scroll_to_top() {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        use wasm_bindgen::JsCast;

        let scrolling_window = window.clone();
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
            scrolling_window.scroll_to_with_x_and_y(0.0, 0.0);
        });
        if window
            .request_animation_frame(callback.unchecked_ref())
            .is_err()
        {
            window.scroll_to_with_x_and_y(0.0, 0.0);
        }
    }
}

/// Opens the browser's modal dialog and puts focus on its note field.
pub fn open_note_dialog() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(dialog) = note_dialog() {
            match dialog.show_modal() {
                Ok(()) => focus_element("note-text"),
                Err(error) => web_sys::console::error_1(&error),
            }
        }
    }
}

/// Closes the modal so the Note button can receive focus again.
pub fn close_note_dialog() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(dialog) = note_dialog() {
            dialog.close();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn note_dialog() -> Option<web_sys::HtmlDialogElement> {
    use wasm_bindgen::JsCast;

    web_sys::window()?
        .document()?
        .get_element_by_id("note-dialog")?
        .dyn_into::<web_sys::HtmlDialogElement>()
        .ok()
}

/// Focuses a page control after modal state changes.
pub fn focus_element(id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        if let Some(element) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id(id))
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = element.focus();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = id;
}

#[cfg(target_arch = "wasm32")]
pub use web::WatchHandle;

#[cfg(not(target_arch = "wasm32"))]
pub struct WatchHandle;

/// Matches the width where the week card becomes visible.
#[cfg(any(target_arch = "wasm32", test))]
fn week_card_visible_at(width: f64) -> bool {
    width >= 840.0
}

/// Reads current width for the week card.
pub fn wide_layout() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.inner_width().ok())
            .and_then(|width| width.as_f64())
            .is_some_and(week_card_visible_at)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// Watches for a new day, saved changes, and viewport resize.
pub fn watch_changes(
    mode: DataMode,
    on_clock: impl FnMut() + 'static,
    on_storage: impl FnMut() + 'static,
    on_resize: impl FnMut() + 'static,
) -> WatchHandle {
    #[cfg(target_arch = "wasm32")]
    {
        web::watch_changes(mode.key(), on_clock, on_storage, on_resize)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (mode, on_clock, on_storage, on_resize);
        WatchHandle
    }
}

#[cfg(test)]
mod tests {
    use werdy_calendar::{Date, Method, Place, fajr};
    use werdy_core::WerdyDay;
    use werdy_store::{Location, PrayerMethod, Settings};

    #[test]
    fn demo_query_selects_only_demo_records_key() {
        assert_eq!(super::mode_from_query("?demo").key(), "werdy.demo.records");
        assert_eq!(
            super::mode_from_query("?demo=1").key(),
            "werdy.demo.records"
        );
        assert_eq!(
            super::mode_from_query("?other=1&demo=").key(),
            "werdy.demo.records"
        );
        assert_eq!(
            super::mode_from_query("?demography=1").key(),
            "werdy.records"
        );
        assert_eq!(super::mode_from_query("").key(), "werdy.records");
    }

    #[test]
    fn week_card_starts_at_expanded_breakpoint() {
        assert!(!super::week_card_visible_at(839.0));
        assert!(super::week_card_visible_at(840.0));
    }

    #[test]
    fn saudi_setting_uses_umm_al_qura_fajr() -> Result<(), Box<dyn std::error::Error>> {
        let settings = Settings {
            location: Location::new(
                Some("riyadh".to_owned()),
                Some("SA".to_owned()),
                24.71,
                46.67,
            ),
            prayer_method: PrayerMethod::UmmAlQura,
            ..Settings::default()
        };
        let time =
            super::fajr_with_offset_lookup(WerdyDay::new(2026, 1, 1)?, &settings, 180, |_| 180)?
                .ok_or("no Fajr")?;
        assert!(time.abs_diff(5 * 60 + 14) <= 2);
        Ok(())
    }

    #[test]
    fn spring_forward_uses_offset_at_fajr() -> Result<(), Box<dyn std::error::Error>> {
        let day = WerdyDay::new(2026, 3, 29)?;
        let settings = Settings {
            location: Location::new(Some("paris".to_owned()), Some("FR".to_owned()), 48.86, 2.35),
            ..Settings::default()
        };
        let old = super::fajr_with_offset_lookup(day, &settings, 60, |_| 60)?.ok_or("no Fajr")?;
        let shifted =
            super::fajr_with_offset_lookup(day, &settings, 60, |_| 120)?.ok_or("no Fajr")?;
        assert_eq!(shifted, old + 60);
        assert_eq!(
            super::day_for_with_offset_lookup(
                day,
                (shifted / 60) as u8,
                (shifted % 60) as u8,
                60,
                &settings,
                |_| 120
            )?,
            day
        );
        Ok(())
    }

    #[test]
    fn autumn_back_uses_offset_at_fajr() -> Result<(), Box<dyn std::error::Error>> {
        let day = WerdyDay::new(2026, 10, 25)?;
        let settings = Settings {
            location: Location::new(Some("paris".to_owned()), Some("FR".to_owned()), 48.86, 2.35),
            ..Settings::default()
        };
        let old = super::fajr_with_offset_lookup(day, &settings, 120, |_| 120)?.ok_or("no Fajr")?;
        let shifted =
            super::fajr_with_offset_lookup(day, &settings, 120, |_| 60)?.ok_or("no Fajr")?;
        assert_eq!(old, shifted + 60);
        assert_eq!(
            super::day_for_with_offset_lookup(
                day,
                (shifted / 60) as u8,
                (shifted % 60) as u8,
                120,
                &settings,
                |_| 60
            )?,
            day
        );
        Ok(())
    }

    #[test]
    fn cairo_day_rolls_at_computed_fajr() -> Result<(), Box<dyn std::error::Error>> {
        let day = WerdyDay::new(2026, 10, 27)?;
        let time = fajr(
            Date::new(2026, 10, 27).map_err(|error| format!("{error:?}"))?,
            Place::new(30.04, 31.24, 120).map_err(|error| format!("{error:?}"))?,
            Method::EgyptianGeneralAuthority,
        )
        .map_err(|error| format!("{error:?}"))?
        .minutes_since_midnight();
        let settings = Settings {
            location: Location::new(
                Some("cairo".to_owned()),
                Some("EG".to_owned()),
                30.04,
                31.24,
            ),
            prayer_method: PrayerMethod::EgyptianGeneralAuthority,
            ..Settings::default()
        };
        assert_eq!(
            super::day_for(
                day,
                ((time - 1) / 60) as u8,
                ((time - 1) % 60) as u8,
                120,
                &settings
            )?,
            day.previous()?
        );
        assert_eq!(
            super::day_for(day, (time / 60) as u8, (time % 60) as u8, 120, &settings)?,
            day
        );
        Ok(())
    }

    #[test]
    fn no_location_keeps_calendar_midnight() -> Result<(), Box<dyn std::error::Error>> {
        let day = WerdyDay::new(2026, 10, 27)?;
        assert_eq!(super::day_for(day, 1, 0, 120, &Settings::default())?, day);
        Ok(())
    }

    #[test]
    fn place_without_fajr_uses_midnight_boundary() -> Result<(), Box<dyn std::error::Error>> {
        let day = WerdyDay::new(2026, 6, 21)?;
        let settings = Settings {
            location: Location::new(
                Some("london".to_owned()),
                Some("GB".to_owned()),
                51.51,
                -0.13,
            ),
            ..Settings::default()
        };
        assert_eq!(super::day_for(day, 1, 0, 60, &settings)?, day);
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::cell::RefCell;
    use std::rc::Rc;

    use wasm_bindgen::{JsCast, closure::Closure};
    use werdy_store::{OwnerData, Store, StoreError, from_json, to_json};

    pub struct WatchHandle {
        window: Option<web_sys::Window>,
        document: Option<web_sys::Document>,
        clock: Closure<dyn FnMut(web_sys::Event)>,
        storage: Closure<dyn FnMut(web_sys::StorageEvent)>,
        resize: Closure<dyn FnMut(web_sys::Event)>,
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
                let _ = window.remove_event_listener_with_callback(
                    "resize",
                    self.resize.as_ref().unchecked_ref(),
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
        key: &'static str,
        on_clock: impl FnMut() + 'static,
        mut on_storage: impl FnMut() + 'static,
        mut on_resize: impl FnMut() + 'static,
    ) -> WatchHandle {
        let on_clock = Rc::new(RefCell::new(on_clock));
        let clock_event = on_clock.clone();
        let clock = Closure::wrap(
            Box::new(move |_: web_sys::Event| (clock_event.borrow_mut())()) as Box<dyn FnMut(_)>,
        );
        let storage = Closure::wrap(Box::new(move |event: web_sys::StorageEvent| {
            if event.key().as_deref() == Some(key) {
                on_storage();
            }
        }) as Box<dyn FnMut(_)>);
        let resize =
            Closure::wrap(Box::new(move |_: web_sys::Event| on_resize()) as Box<dyn FnMut(_)>);
        let timer = Closure::wrap(Box::new(move || (on_clock.borrow_mut())()) as Box<dyn FnMut()>);
        let window = web_sys::window();
        let document = window.as_ref().and_then(web_sys::Window::document);
        let interval = window.as_ref().and_then(|window| {
            let _ =
                window.add_event_listener_with_callback("focus", clock.as_ref().unchecked_ref());
            let _ = window
                .add_event_listener_with_callback("storage", storage.as_ref().unchecked_ref());
            let _ =
                window.add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref());
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
            resize,
            _timer: timer,
            interval,
        }
    }

    /// Store backed by the browser's localStorage.
    pub struct LocalStore {
        pub key: &'static str,
    }

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
                .get_item(self.key)
                .map_err(|error| StoreError::Backend(format!("{error:?}")))?;
            text.as_deref().map(from_json).transpose()
        }

        fn save(&self, data: &OwnerData) -> Result<(), StoreError> {
            storage()?
                .set_item(self.key, &to_json(data)?)
                .map_err(|error| StoreError::Backend(format!("{error:?}")))
        }
    }
}
