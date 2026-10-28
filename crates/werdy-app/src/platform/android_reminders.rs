//! JNI alarm delivery runs on one worker, so rapid saves cannot overtake each other.
use crate::{i18n::Texts, reminders};
use std::sync::{OnceLock, mpsc};
use werdy_core::WerdyDay;
use werdy_store::OwnerData;

use super::android_bridge as bridge;

pub(super) fn enqueue(owner: &OwnerData) {
    static WORKER: OnceLock<Option<mpsc::Sender<OwnerData>>> = OnceLock::new();
    if let Some(sender) = WORKER.get_or_init(start_worker)
        && sender.send(owner.clone()).is_err()
    {
        dioxus::prelude::warn!("Reminder worker stopped");
    }
}

fn start_worker() -> Option<mpsc::Sender<OwnerData>> {
    let (sender, receiver) = mpsc::channel();
    match std::thread::Builder::new()
        .name("werdy-reminders".into())
        .spawn(move || {
            let texts = Texts::load();
            while let Ok(mut owner) = receiver.recv() {
                while let Ok(latest) = receiver.try_recv() {
                    owner = latest;
                }
                if let Err(error) = replace(&owner, &texts) {
                    dioxus::prelude::warn!("Reminder scheduling failed: {error}");
                }
            }
        }) {
        Ok(_) => Some(sender),
        Err(error) => {
            dioxus::prelude::warn!("Could not start reminder worker: {error}");
            None
        }
    }
}

fn epoch_at(date: WerdyDay, minute: u16) -> Result<i64, String> {
    reminders::epoch_with_lookup(date, minute, super::android::offset_at)
}

fn replace(owner: &OwnerData, texts: &Texts) -> Result<(), String> {
    let clock = super::android::clock().map_err(|error| error.to_string())?;
    let now = werdy_reminders::LocalMoment::new(clock.day, clock.hour, clock.minute)
        .map_err(|error| error.to_string())?;
    // Scheduling uses elapsed minutes from midnight, including a transition later that day.
    let mut midnights = std::collections::BTreeMap::new();
    let mut date = clock.day.previous().map_err(|error| error.to_string())?;
    for _ in 0..3 {
        midnights.insert(date, epoch_at(date, 0)? / 1000);
        date = date.next().map_err(|error| error.to_string())?;
    }
    let plan = werdy_reminders::schedule(owner, now, 2, |date, minute| {
        midnights.get(&date).map_or(i32::MAX, |midnight| {
            super::android::offset_at(*midnight + i64::from(minute) * 60)
        })
    })
    .map_err(|error| error.to_string())?;
    let payload = reminders::payload(owner, &plan, texts, epoch_at)?;
    let json = serde_json::to_string(&payload).map_err(|error| error.to_string())?;
    reminders::ok_reply(&jni_reply(bridge::schedule(&new_plugin()?, json))?)
}

pub(super) fn permission() -> Result<bool, String> {
    reminders::permission_reply(&jni_reply(bridge::permission(&new_plugin()?))?)
}

pub(super) fn request_permission() -> Result<(), String> {
    reminders::ok_reply(&jni_reply(bridge::requestPermission(&new_plugin()?))?)
}

fn jni_reply(reply: Result<String, String>) -> Result<String, String> {
    reply.inspect_err(|_| {
        manganis::android::with_activity(|env, _| {
            let _ = env.exception_clear();
            Some(())
        });
    })
}

fn new_plugin() -> Result<bridge::Reminders, String> {
    bridge::new_plugin("com.ahmedhossam.werdy.reminders.Reminders")
        .map(bridge::Reminders::from_global_ref)
}
