#[cfg(any(target_os = "android", target_arch = "wasm32"))]
pub const WEB_CLIENT_ID: &str =
    "667797962167-5ic1h7tc5i8gc5f7actv7rqb03ldcebf.apps.googleusercontent.com";
pub const SUPABASE_URL: &str = "https://ysrvszrhkwgsqrrvvdhj.supabase.co";
/// Public by design. RLS and the owner's JWT control access.
pub const SUPABASE_KEY: &str = "sb_publishable_4-gZvoma1DWo2GljgV2T0w_q6D1NivV";
