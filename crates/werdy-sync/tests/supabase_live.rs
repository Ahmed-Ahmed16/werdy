#![cfg(all(feature = "supabase", not(target_arch = "wasm32")))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{SystemTime, UNIX_EPOCH};
use werdy_sync::{Backend, Change, Pending, SupabaseBackend};

/// Use a dedicated test account. This appends receipts that remain in its stream.
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires WERDY_SUPABASE_URL, WERDY_SUPABASE_KEY and WERDY_SUPABASE_TOKEN"]
async fn supabase_round_trip() {
    let url = std::env::var("WERDY_SUPABASE_URL").expect("set WERDY_SUPABASE_URL");
    let key = std::env::var("WERDY_SUPABASE_KEY").expect("set WERDY_SUPABASE_KEY");
    let token = std::env::var("WERDY_SUPABASE_TOKEN").expect("set WERDY_SUPABASE_TOKEN");
    let mut backend = SupabaseBackend::new(&url, &key, &token).unwrap();
    let install = format!(
        "rpc_test_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let pending = Pending {
        install: install.clone(),
        operation: 1,
        change: Change::ChosenSet {
            key: install,
            chosen: false,
        },
    };
    assert!(backend.server_time().await.unwrap() > 0);
    let pushed = backend.push(std::slice::from_ref(&pending)).await.unwrap();
    assert_eq!(pushed.changes.len(), 1);
    assert_eq!(pushed.changes[0].pending, pending);
    let stamp = pushed.changes[0].stamp;
    assert!(stamp.sequence > 0);
    let pulled = backend.pull(stamp.sequence - 1).await.unwrap();
    assert!(pulled.changes.contains(&pushed.changes[0]));
    let retried = backend.push(&[pending]).await.unwrap();
    assert_eq!(retried.changes, pushed.changes);
}
