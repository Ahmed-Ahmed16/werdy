#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::{Change, DeedKey};
use werdy_core::{Fraction, Mark, MarkEntry, WerdyDay};

fn backend() -> SupabaseBackend {
    SupabaseBackend::new(
        "https://example.invalid",
        "sb_publishable_test",
        "test_token",
    )
    .unwrap()
}

#[test]
fn sql_wire_fixture_matches_rust_request_layout() {
    let sql = include_str!("../../../../supabase/tests/sync.sql");
    let fixture = sql.split("$wire$").nth(1).unwrap();
    let batch: Vec<Pending> = serde_json::from_str(fixture).unwrap();
    let request = backend()
        .request("werdy_sync_push", &PushRequest { batch: &batch })
        .unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
    assert_eq!(
        body["batch"],
        serde_json::from_str::<serde_json::Value>(fixture).unwrap()
    );
}

#[test]
fn rpc_requests_use_correct_routes_arguments_and_refreshed_credentials() {
    let mut backend = backend();
    let pending = Pending {
        install: "phone".to_owned(),
        operation: 7,
        change: Change::ChosenSet {
            key: "prayers".to_owned(),
            chosen: true,
        },
    };
    let request = backend
        .request("werdy_sync_push", &PushRequest { batch: &[pending] })
        .unwrap();
    assert_eq!(
        request.url().as_str(),
        "https://example.invalid/rest/v1/rpc/werdy_sync_push"
    );
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(request.headers()["apikey"], "sb_publishable_test");
    assert_eq!(request.headers()[AUTHORIZATION], "Bearer test_token");
    assert!(request.headers()[AUTHORIZATION].is_sensitive());
    assert!(request.headers()["apikey"].is_sensitive());
    let body: serde_json::Value =
        serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"batch":[{
            "install":"phone", "operation":7, "change":{"ChosenSet":{"key":"prayers","chosen":true}}
        }]})
    );
    backend.set_access_token("refreshed_token").unwrap();
    assert!(backend.set_access_token("bad\ntoken").is_err());
    let request = backend
        .request("werdy_sync_pull", &PullRequest { after_cursor: 42 })
        .unwrap();
    assert_eq!(request.url().path(), "/rest/v1/rpc/werdy_sync_pull");
    assert_eq!(request.headers()[AUTHORIZATION], "Bearer refreshed_token");
    assert_eq!(
        request.body().unwrap().as_bytes().unwrap(),
        br#"{"after_cursor":42}"#
    );
    let request = backend
        .request("werdy_sync_server_time", &TimeRequest {})
        .unwrap();
    assert_eq!(request.body().unwrap().as_bytes().unwrap(), b"{}");
}

#[test]
fn full_width_fraction_serializes_without_value_conversion_or_rounding() {
    let day = WerdyDay::new(2026, 10, 28).unwrap();
    let pending = Pending {
        install: "phone".to_owned(),
        operation: u64::MAX,
        change: Change::Mark {
            id: DeedKey {
                install: "phone".to_owned(),
                sequence: 1,
            },
            day,
            entry: Some(MarkEntry::new(
                Mark::Partial(Fraction::from_ratio(u128::MAX - 1, u128::MAX).unwrap()),
                day,
            )),
        },
    };
    let request = backend()
        .request(
            "werdy_sync_push",
            &PushRequest {
                batch: std::slice::from_ref(&pending),
            },
        )
        .unwrap();
    #[derive(serde::Deserialize)]
    struct Sent {
        batch: Vec<Pending>,
    }
    let sent: Sent = serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
    assert_eq!(sent.batch, vec![pending]);
}

#[test]
fn canned_rpc_batch_preserves_receipts_tombstones_and_server_stamps() {
    let batch: Batch = parse_response(br#"{
        "cursor":2,"changes":[
          {"pending":{"install":"phone","operation":1,"change":{"Note":{"day":20754,"note":{"text":"ill","credit":"half","entered_on":20754}}}},
           "stamp":{"received_at":1793145600000,"sequence":1}},
          {"pending":{"install":"phone","operation":2,"change":{"Note":{"day":20754,"note":null}}},
           "stamp":{"received_at":1793145600000,"sequence":2}}
        ]}"#).unwrap();
    assert_eq!(batch.cursor, 2);
    assert_eq!(batch.changes[0].stamp.received_at, 1_793_145_600_000);
    assert_eq!(batch.changes[1].stamp.sequence, 2);
    assert!(matches!(
        batch.changes[1].pending.change,
        Change::Note { note: None, .. }
    ));
    assert_eq!(
        parse_response::<u64>(b"1793145600000").unwrap(),
        1_793_145_600_000
    );
}

#[test]
fn malformed_responses_and_http_failures_never_echo_response_secrets() {
    for body in [
        br#"{"cursor":"secret_token","changes":[]}"#.as_slice(),
        br#"{"cursor":1,"changes":[{"pending":{}}]}"#,
        br#"{"cursor":0,"changes":[],"extra":"secret_token"}"#,
        b"not JSON secret_token",
        br#"[{"cursor":0,"changes":[]}]"#,
    ] {
        let error = parse_response::<Batch>(body).unwrap_err();
        assert!(!error.to_string().contains("secret_token"));
    }
    for status in [
        StatusCode::UNAUTHORIZED,
        StatusCode::FORBIDDEN,
        StatusCode::BAD_REQUEST,
        StatusCode::FOUND,
        StatusCode::INTERNAL_SERVER_ERROR,
    ] {
        let error = check_status(status).unwrap_err();
        assert!(error.to_string().contains(status.as_str()));
    }
}

#[test]
fn unsafe_urls_and_invalid_credentials_are_rejected_without_echo() {
    for url in [
        "http://example.invalid",
        "https://secret_token@example.invalid",
        "https://example.invalid/?token=secret_token",
        "https://example.invalid/rest",
        "https://example.invalid/#secret_token",
        "not a URL",
    ] {
        let error = SupabaseBackend::new(url, "sb_publishable_test", "secret_token")
            .err()
            .unwrap();
        assert!(!error.to_string().contains("secret_token"));
    }
    for (key, token) in [
        ("service_role_secret_token", "token"),
        ("sb_publishable_test", ""),
        ("sb_publishable_test", "secret_token\n"),
    ] {
        let error = SupabaseBackend::new("https://example.invalid", key, token)
            .err()
            .unwrap();
        assert!(!error.to_string().contains("secret_token"));
    }
    assert!(SupabaseBackend::new("http://127.0.0.1:54321", "sb_publishable_test", "token").is_ok());
}
