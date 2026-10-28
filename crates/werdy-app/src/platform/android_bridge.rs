//! One metadata entry for the one Gradle library. Two FFI blocks would create
//! separate artifact names for the same path in manganis 0.7.10.
use manganis::jni::{
    JNIEnv,
    objects::{GlobalRef, JClass, JObject, JValue},
};

#[allow(non_snake_case)]
#[manganis::ffi("src/android")]
unsafe extern "Kotlin" {
    pub type Reminders;
    pub fn schedule(this: &Reminders, json: String) -> String;
    pub fn permission(this: &Reminders) -> String;
    pub fn requestPermission(this: &Reminders) -> String;
    pub type GoogleSignin;
    pub fn start(this: &GoogleSignin, web_client_id: String, nonce: String) -> String;
    pub fn poll(this: &GoogleSignin) -> String;
    pub fn cancel(this: &GoogleSignin) -> String;
}

pub fn clear_exception() {
    manganis::android::with_activity(|env, _| {
        let _ = env.exception_clear();
        Some(())
    });
}

pub fn new_plugin(class: &str) -> Result<GlobalRef, String> {
    manganis::android::with_activity(|env, activity| {
        let built = env.with_local_frame(8, |env| construct(env, activity, class));
        if built.is_err() {
            let _ = env.exception_clear();
        }
        Some(built.map_err(|_| "Could not create Android bridge".into()))
    })
    .unwrap_or_else(|| Err("No Android activity available".into()))
}

// Dioxus #5629: FindClass on native threads cannot see app classes.
fn construct(
    env: &mut JNIEnv<'_>,
    activity: &JObject<'_>,
    class: &str,
) -> manganis::jni::errors::Result<GlobalRef> {
    let loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
        .l()?;
    let name = env.new_string(class)?;
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
