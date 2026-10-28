use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::dioxus_core::Task;
use dioxus::prelude::spawn;
use werdy_core::{Error, WerdyDay};

pub(super) struct Clock {
    pub day: WerdyDay,
    pub hour: u8,
    pub minute: u8,
    midnight_seconds: i64,
}

impl Clock {
    pub fn midnight_offset(&self) -> i32 {
        offset_at(self.midnight_seconds)
    }

    pub fn offset_at_fajr(&self, minutes: u16) -> i32 {
        offset_at(self.midnight_seconds + i64::from(minutes) * 60)
    }
}

pub(super) fn clock() -> Result<Clock, Error> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::InvalidDate)?
        .as_secs();
    let seconds = i64::try_from(seconds).map_err(|_| Error::InvalidDate)?;
    let offset = offset_at(seconds);
    let (day, hour, minute) = super::local_clock_at(seconds, offset)?;
    let local_midnight = (seconds + i64::from(offset) * 60).div_euclid(86_400) * 86_400;
    let guess = local_midnight - i64::from(offset) * 60;
    let midnight_seconds = local_midnight - i64::from(offset_at(guess)) * 60;
    Ok(Clock {
        day,
        hour,
        minute,
        midnight_seconds,
    })
}

/// Falls back to UTC if the VM or timezone lookup fails.
fn offset_at(seconds: i64) -> i32 {
    let Some(millis) = seconds.checked_mul(1_000) else {
        return 0;
    };
    manganis::android::with_activity(|env, _| match timezone_offset(env, millis) {
        Ok(offset) => Some(offset),
        Err(_) => {
            let _ = env.exception_clear();
            None
        }
    })
    .unwrap_or(0)
}

fn timezone_offset(
    env: &mut manganis::jni::JNIEnv<'_>,
    millis: i64,
) -> manganis::jni::errors::Result<i32> {
    let zone = env
        .call_static_method(
            "java/util/TimeZone",
            "getDefault",
            "()Ljava/util/TimeZone;",
            &[],
        )?
        .l()?;
    let zone = env.auto_local(zone);
    env.call_method(
        &zone,
        "getOffset",
        "(J)I",
        &[manganis::jni::objects::JValue::Long(millis)],
    )?
    .i()
    .map(|offset| offset / 60_000)
}

thread_local! {
    static WIDE_LAYOUT: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn wide_layout() -> bool {
    WIDE_LAYOUT.get()
}

#[derive(serde::Deserialize)]
#[serde(tag = "event", content = "width")]
enum BrowserEvent {
    Resize(f64),
    Clock,
}

pub struct WatchHandle {
    clock: Task,
    browser: Task,
    listener: dioxus::document::Eval,
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        let _ = self.listener.send(());
        self.clock.cancel();
        self.browser.cancel();
    }
}

const WATCH_SCRIPT: &str = r#"
    const resize = () => dioxus.send({event: 'Resize', width: window.innerWidth});
    const clock = () => dioxus.send({event: 'Clock'});
    window.addEventListener('resize', resize);
    window.addEventListener('focus', clock);
    document.addEventListener('visibilitychange', clock);
    try {
        resize();
        await dioxus.recv();
    } finally {
        window.removeEventListener('resize', resize);
        window.removeEventListener('focus', clock);
        document.removeEventListener('visibilitychange', clock);
    }
"#;

pub(super) fn watch_changes(
    on_clock: impl FnMut() + 'static,
    on_resize: impl FnMut() + 'static,
) -> WatchHandle {
    let on_clock = Rc::new(RefCell::new(on_clock));
    let timer_clock = on_clock.clone();
    let clock = spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            (timer_clock.borrow_mut())();
        }
    });
    let listener = dioxus::document::eval(WATCH_SCRIPT);
    let browser = spawn(watch_browser(listener, on_clock, on_resize));
    WatchHandle {
        clock,
        browser,
        listener,
    }
}

async fn watch_browser(
    mut listener: dioxus::document::Eval,
    on_clock: Rc<RefCell<impl FnMut()>>,
    mut on_resize: impl FnMut(),
) {
    while let Ok(event) = listener.recv::<BrowserEvent>().await {
        match event {
            BrowserEvent::Resize(width) => {
                WIDE_LAYOUT.set(super::week_card_visible_at(width));
                on_resize();
            }
            BrowserEvent::Clock => (on_clock.borrow_mut())(),
        }
    }
}
