//! JNI alarm delivery runs on one worker, so rapid saves cannot overtake each other.
use crate::{i18n::Texts, reminders};
use manganis::jni::{
    JNIEnv,
    objects::{GlobalRef, JClass, JObject, JValue},
};
use std::sync::{OnceLock, mpsc};
use werdy_core::WerdyDay;
use werdy_store::OwnerData;

// The exported method name must match Kotlin's requestPermission signature.
#[allow(non_snake_case)]
mod bridge {
    #[manganis::ffi("src/android")]
    unsafe extern "Kotlin" {
        pub type Reminders;
        pub fn schedule(this: &Reminders, json: String) -> String;
        pub fn permission(this: &Reminders) -> String;
        pub fn requestPermission(this: &Reminders) -> String;
    }
}

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

// Dioxus #5629: native threads must use the activity's class loader, not FindClass.
fn new_plugin() -> Result<bridge::Reminders, String> {
    manganis::android::with_activity(|env, activity| {
        let built = env.with_local_frame(8, |env| construct(env, activity));
        if built.is_err() {
            let _ = env.exception_clear();
        }
        Some(
            built
                .map(bridge::Reminders::from_global_ref)
                .map_err(|error| format!("Could not create reminders bridge: {error}")),
        )
    })
    .unwrap_or_else(|| Err("No Android activity available".into()))
}

fn construct(
    env: &mut JNIEnv<'_>,
    activity: &JObject<'_>,
) -> manganis::jni::errors::Result<GlobalRef> {
    let loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
        .l()?;
    let name = env.new_string("com.ahmedhossam.werdy.reminders.Reminders")?;
    let class = env
        .call_method(
            &loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&name)],
        )?
        .l()?;
    let instance = env.new_object(
        JClass::from(class),
        "(Landroid/app/Activity;)V",
        &[JValue::Object(activity)],
    )?;
    env.new_global_ref(instance)
}
