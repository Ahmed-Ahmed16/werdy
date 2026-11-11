use crate::{Accepted, Batch, SupabaseBackend, SyncError, invalid, normalize_email};
use serde::{Deserialize, Serialize};

/// A bearer secret, intentionally without Debug or serialization for logging.
#[derive(Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShareCode {
    pub code: String,
    pub expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharePerson {
    pub id: String,
    pub email: Option<String>,
    pub created_at: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchPage {
    pub changes: Vec<Accepted>,
    pub cursor: u64,
    pub has_more: bool,
}

impl WatchPage {
    pub fn batch(self) -> Batch {
        Batch {
            changes: self.changes,
            cursor: self.cursor,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchDirection {
    WatchMe,
    WatchThem,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchRequest {
    pub id: String,
    pub email: Option<String>,
    pub direction: WatchDirection,
    pub created_at: u64,
}

#[derive(Serialize)]
struct SendWatch<'a> {
    email: &'a str,
    direction: WatchDirection,
}

/// Mirrors the waiting sent request limit in werdy_request_send on the server.
pub const WATCH_REQUEST_LIMIT: u8 = 5;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SentWatchRequest {
    pub id: String,
    pub email: String,
    pub direction: WatchDirection,
    pub created_at: u64,
    pub expires_at: u64,
}
#[derive(Serialize)]
struct RequestId<'a> {
    request_id: &'a str,
}

#[derive(Serialize)]
struct Redeem<'a> {
    code: &'a str,
}
#[derive(Serialize)]
struct Revoke<'a> {
    watcher_id: &'a str,
}
#[derive(Serialize)]
struct Leave<'a> {
    owner_id: &'a str,
}
#[derive(Serialize)]
struct Pull<'a> {
    owner_id: &'a str,
    after_cursor: u64,
    page_limit: u16,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Outcome {
    ok: bool,
}

fn outcome(result: Outcome) -> Result<(), SyncError> {
    if result.ok {
        Ok(())
    } else {
        Err(invalid("share request refused"))
    }
}

impl SupabaseBackend {
    pub async fn send_watch_request(
        &self,
        email: &str,
        direction: WatchDirection,
    ) -> Result<(), SyncError> {
        let email = normalize_email(email)?;
        outcome(
            self.rpc(
                "werdy_request_send",
                &SendWatch {
                    email: &email,
                    direction,
                },
            )
            .await?,
        )
    }
    pub async fn incoming_watch_requests(&self) -> Result<Vec<WatchRequest>, SyncError> {
        self.rpc("werdy_request_incoming", &super::TimeRequest {})
            .await
    }
    pub async fn outgoing_watch_requests(&self) -> Result<Vec<SentWatchRequest>, SyncError> {
        self.rpc("werdy_request_outgoing", &super::TimeRequest {})
            .await
    }
    pub async fn cancel_watch_request(&self, id: &str) -> Result<(), SyncError> {
        outcome(
            self.rpc("werdy_request_cancel", &RequestId { request_id: id })
                .await?,
        )
    }
    pub async fn accept_watch_request(&self, id: &str) -> Result<(), SyncError> {
        outcome(
            self.rpc("werdy_request_accept", &RequestId { request_id: id })
                .await?,
        )
    }
    pub async fn decline_watch_request(&self, id: &str) -> Result<(), SyncError> {
        outcome(
            self.rpc("werdy_request_decline", &RequestId { request_id: id })
                .await?,
        )
    }

    pub async fn create_share_code(&self) -> Result<ShareCode, SyncError> {
        let reply: ShareCode = self
            .rpc("werdy_share_create", &super::TimeRequest {})
            .await?;
        if reply.code.len() != 10
            || !reply
                .code
                .bytes()
                .all(|b| b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789".contains(&b))
        {
            return Err(invalid("invalid share reply"));
        }
        Ok(reply)
    }
    pub async fn redeem_share_code(&self, code: &str) -> Result<(), SyncError> {
        if code.len() > 32 {
            return Err(invalid("share request refused"));
        }
        outcome(self.rpc("werdy_share_redeem", &Redeem { code }).await?)
    }
    pub async fn list_watchers(&self) -> Result<Vec<SharePerson>, SyncError> {
        self.rpc("werdy_share_watchers", &super::TimeRequest {})
            .await
    }
    pub async fn list_watched(&self) -> Result<Vec<SharePerson>, SyncError> {
        self.rpc("werdy_share_watched", &super::TimeRequest {})
            .await
    }
    pub async fn revoke_watcher(&self, watcher_id: &str) -> Result<(), SyncError> {
        outcome(
            self.rpc("werdy_share_revoke", &Revoke { watcher_id })
                .await?,
        )
    }
    pub async fn leave_share(&self, owner_id: &str) -> Result<(), SyncError> {
        outcome(self.rpc("werdy_share_leave", &Leave { owner_id }).await?)
    }
    pub async fn watch_pull(
        &self,
        owner_id: &str,
        after_cursor: u64,
        page_limit: u16,
    ) -> Result<WatchPage, SyncError> {
        if !(1..=500).contains(&page_limit) {
            return Err(invalid("invalid watch limit"));
        }
        let page: WatchPage = self
            .rpc(
                "werdy_watch_pull",
                &Pull {
                    owner_id,
                    after_cursor,
                    page_limit,
                },
            )
            .await?;
        if page.changes.len() > usize::from(page_limit) || page.has_more && page.changes.is_empty()
        {
            return Err(invalid("invalid watch page"));
        }
        Ok(page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sent_requests_require_email_and_expiry_and_reject_extra_fields() {
        let reply = serde_json::json!([{
            "id":"request", "email":"receiver@example.test", "direction":"watch_them",
            "created_at":1, "expires_at":2592000001_u64
        }]);
        let requests: Vec<SentWatchRequest> =
            super::super::parse_response(&serde_json::to_vec(&reply).unwrap()).unwrap();
        assert_eq!(requests[0].email, "receiver@example.test");
        assert_eq!(requests[0].direction, WatchDirection::WatchThem);
        assert_eq!(requests[0].expires_at - requests[0].created_at, 2592000000);
        for field in ["email", "expires_at"] {
            let mut invalid = reply.clone();
            invalid[0].as_object_mut().unwrap().remove(field);
            assert!(
                super::super::parse_response::<Vec<SentWatchRequest>>(
                    &serde_json::to_vec(&invalid).unwrap()
                )
                .is_err()
            );
        }
        let mut extra = reply;
        extra[0]["declined"] = true.into();
        assert!(
            super::super::parse_response::<Vec<SentWatchRequest>>(
                &serde_json::to_vec(&extra).unwrap()
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn invalid_watch_email_is_refused_before_network() {
        let backend =
            SupabaseBackend::new("https://example.invalid", "sb_publishable_test", "jwt").unwrap();
        assert!(matches!(
            backend
                .send_watch_request("bad", WatchDirection::WatchMe)
                .await,
            Err(SyncError::Invalid(_))
        ));
    }

    #[test]
    fn watch_request_wire_contract_is_strict() {
        let backend =
            SupabaseBackend::new("https://example.test", "sb_publishable_test", "jwt").unwrap();
        for (name, body, expected) in [
            (
                "werdy_request_send",
                serde_json::to_value(SendWatch {
                    email: "a@b",
                    direction: WatchDirection::WatchMe,
                })
                .unwrap(),
                serde_json::json!({"email":"a@b","direction":"watch_me"}),
            ),
            (
                "werdy_request_incoming",
                serde_json::json!({}),
                serde_json::json!({}),
            ),
            (
                "werdy_request_outgoing",
                serde_json::to_value(super::super::TimeRequest {}).unwrap(),
                serde_json::json!({}),
            ),
            (
                "werdy_request_cancel",
                serde_json::to_value(RequestId { request_id: "id" }).unwrap(),
                serde_json::json!({"request_id":"id"}),
            ),
            (
                "werdy_request_accept",
                serde_json::to_value(RequestId { request_id: "id" }).unwrap(),
                serde_json::json!({"request_id":"id"}),
            ),
            (
                "werdy_request_decline",
                serde_json::to_value(RequestId { request_id: "id" }).unwrap(),
                serde_json::json!({"request_id":"id"}),
            ),
        ] {
            let request = backend.request(name, &body).unwrap();
            assert!(request.url().path().ends_with(name));
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(
                    request.body().unwrap().as_bytes().unwrap()
                )
                .unwrap(),
                expected
            );
        }
        let requests: Vec<WatchRequest> = super::super::parse_response(
            br#"[{"id":"id","email":null,"direction":"watch_me","created_at":1}]"#,
        )
        .unwrap();
        assert_eq!(requests[0].direction, WatchDirection::WatchMe);
        assert_eq!(
            serde_json::to_value(WatchDirection::WatchThem).unwrap(),
            "watch_them"
        );
        assert!(
            super::super::parse_response::<WatchRequest>(
                br#"{"id":"id","email":null,"direction":"watch_me","created_at":1,"extra":true}"#
            )
            .is_err()
        );
        assert!(super::super::parse_response::<Outcome>(br#"{"ok":true,"extra":true}"#).is_err());
        assert!(outcome(super::super::parse_response(br#"{"ok":false}"#).unwrap()).is_err());
    }

    #[test]
    fn sharing_wire_requests_and_canned_replies_do_not_leak_secrets() {
        let backend =
            SupabaseBackend::new("https://example.test", "sb_publishable_test", "jwt").unwrap();
        for (name, body, expected) in [
            (
                "werdy_share_create",
                serde_json::json!({}),
                serde_json::json!({}),
            ),
            (
                "werdy_share_redeem",
                serde_json::to_value(Redeem { code: "SECRET" }).unwrap(),
                serde_json::json!({"code":"SECRET"}),
            ),
            (
                "werdy_share_watchers",
                serde_json::json!({}),
                serde_json::json!({}),
            ),
            (
                "werdy_share_watched",
                serde_json::json!({}),
                serde_json::json!({}),
            ),
            (
                "werdy_share_revoke",
                serde_json::to_value(Revoke { watcher_id: "id" }).unwrap(),
                serde_json::json!({"watcher_id":"id"}),
            ),
            (
                "werdy_share_leave",
                serde_json::to_value(Leave { owner_id: "id" }).unwrap(),
                serde_json::json!({"owner_id":"id"}),
            ),
            (
                "werdy_watch_pull",
                serde_json::to_value(Pull {
                    owner_id: "id",
                    after_cursor: 3,
                    page_limit: 50,
                })
                .unwrap(),
                serde_json::json!({"owner_id":"id","after_cursor":3,"page_limit":50}),
            ),
        ] {
            let request = backend.request(name, &body).unwrap();
            assert!(request.url().path().ends_with(name));
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(
                    request.body().unwrap().as_bytes().unwrap()
                )
                .unwrap(),
                expected
            );
        }
        let code: ShareCode =
            super::super::parse_response(br#"{"code":"ABCDEFGHJK","expires_at":1000}"#).unwrap();
        assert_eq!(code.expires_at, 1000);
        let people: Vec<SharePerson> = super::super::parse_response(
            br#"[{"id":"id","email":"owner@example.test","created_at":1}]"#,
        )
        .unwrap();
        assert_eq!(people[0].email.as_deref(), Some("owner@example.test"));
        let page: WatchPage =
            super::super::parse_response(br#"{"changes":[],"cursor":0,"has_more":false}"#).unwrap();
        assert_eq!(page.batch().cursor, 0);
        assert!(outcome(Outcome { ok: true }).is_ok());
        assert!(outcome(Outcome { ok: false }).is_err());
        let error =
            super::super::parse_response::<ShareCode>(br#"{"code":"SECRET","token":"JWT"}"#)
                .err()
                .unwrap()
                .to_string();
        assert!(!error.contains("SECRET") && !error.contains("JWT"));
    }
}
