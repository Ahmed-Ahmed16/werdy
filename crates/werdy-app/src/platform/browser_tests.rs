#![allow(clippy::expect_used)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use werdy_core::WerdyDay;
use werdy_store::{MemoryStore, OwnerData, PrayerMethod, Settings, Theme};

use super::{DataMode, open_store, request_location_with_timeout};
use crate::state::AppState;

wasm_bindgen_test_configure!(run_in_browser);

// Replace only the browser boundary, and restore it before yielding to other tests.
#[wasm_bindgen(inline_js = r#"
export class GeolocationStub {
    constructor() {
        this.geo = navigator.geolocation;
        this.original = Object.getOwnPropertyDescriptor(this.geo, 'getCurrentPosition');
        this.called = false;
        Object.defineProperty(this.geo, 'getCurrentPosition', {
            configurable: true,
            value: (success) => { this.called = true; this.success = success; }
        });
    }
    restore() {
        if (this.original) {
            Object.defineProperty(this.geo, 'getCurrentPosition', this.original);
        } else {
            delete this.geo.getCurrentPosition;
        }
    }
    answer() {
        this.success({ coords: { latitude: 51.51, longitude: -0.13 } });
    }
}
export function wait(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
"#)]
extern "C" {
    type GeolocationStub;
    #[wasm_bindgen(constructor)]
    fn new() -> GeolocationStub;
    #[wasm_bindgen(method)]
    fn restore(this: &GeolocationStub);
    #[wasm_bindgen(method)]
    fn answer(this: &GeolocationStub);
    #[wasm_bindgen(method, getter)]
    fn called(this: &GeolocationStub) -> bool;
    fn wait(ms: u32) -> js_sys::Promise;
}

#[wasm_bindgen_test]
async fn unanswered_geolocation_reports_error_after_timeout() {
    let request = Rc::new(RefCell::new(crate::state::LocationRequest::default()));
    let id = request.borrow_mut().start().expect("request starts");
    let errors = Rc::new(Cell::new(0));
    let failures = errors.clone();
    let pending = request.clone();
    let successes = Rc::new(Cell::new(0));
    let answers = successes.clone();
    let stub = GeolocationStub::new();
    request_location_with_timeout(
        5,
        move |_| answers.set(answers.get() + 1),
        move || {
            failures.set(failures.get() + 1);
            pending.borrow_mut().fail(id);
        },
    );
    stub.restore();
    assert!(stub.called());
    assert_eq!(errors.get(), 0);
    assert!(request.borrow().is_finding());

    JsFuture::from(wait(50))
        .await
        .expect("browser timer resolves");
    assert_eq!(errors.get(), 1);
    assert_eq!(successes.get(), 0);
    assert!(!request.borrow().is_finding());
    assert_eq!(request.borrow().error_key(), Some("location-error"));
}

#[wasm_bindgen_test]
async fn late_geolocation_preserves_chosen_city() {
    let state = Rc::new(RefCell::new(AppState::start(
        &MemoryStore::default(),
        WerdyDay::new(2026, 10, 28).expect("valid date"),
        Vec::new(),
    )));
    let id = state
        .borrow_mut()
        .location_request
        .start()
        .expect("request starts");
    let pending = state.clone();
    let accepted = Rc::new(Cell::new(None));
    let answer = accepted.clone();
    let stub = GeolocationStub::new();
    request_location_with_timeout(
        5,
        move |place| {
            answer.set(Some(
                pending
                    .borrow_mut()
                    .accept_location(id, place)
                    .expect("valid GPS"),
            ));
        },
        || {},
    );
    stub.restore();
    assert!(stub.called());
    let city = crate::location::cities()
        .expect("bundled cities")
        .into_iter()
        .find(|city| city.key == "cairo")
        .and_then(|city| city.location())
        .expect("Cairo exists");
    state.borrow_mut().location_request.cancel();
    state
        .borrow_mut()
        .set_location(city.clone())
        .expect("valid city");

    JsFuture::from(wait(1))
        .await
        .expect("browser timer resolves");
    stub.answer();
    assert_eq!(accepted.get(), Some(false));
    assert_eq!(state.borrow().data.settings.location, Some(city));
    assert_eq!(
        state.borrow().data.settings.prayer_method,
        PrayerMethod::EgyptianGeneralAuthority
    );
    assert_eq!(state.borrow().location_request.error_key(), None);
}

#[wasm_bindgen_test]
fn browser_store_round_trip() {
    let storage = web_sys::window()
        .expect("browser window")
        .local_storage()
        .expect("storage available")
        .expect("localStorage");
    for mode in [DataMode::Normal, DataMode::Demo] {
        let key = mode.key();
        let previous = storage.get_item(key).expect("read previous records");
        storage.remove_item(key).expect("empty test storage");
        let store = open_store(mode);
        let empty = store.load();
        let records = OwnerData {
            first_run_complete: true,
            settings: Settings {
                theme: Theme::Dark,
                ..Settings::default()
            },
            ..OwnerData::default()
        };
        let saved = store.save(&records);
        let raw = storage.get_item(key).expect("read saved JSON");
        let loaded = open_store(mode).load();
        match previous {
            Some(json) => storage
                .set_item(key, &json)
                .expect("restore previous records"),
            None => storage.remove_item(key).expect("remove test records"),
        }
        assert_eq!(empty, Ok(None));
        assert_eq!(saved, Ok(()));
        assert!(raw.is_some());
        assert_eq!(loaded, Ok(Some(records)));
    }
}
