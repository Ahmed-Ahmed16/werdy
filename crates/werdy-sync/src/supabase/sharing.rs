use crate::{Accepted, Batch, SupabaseBackend, SyncError, invalid};
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
