//! Append-only audit log — Phase 6 of
//! `book/src/roadmap/accounts-and-login.md`.
//!
//! `AuditStore::record` writes one row per security-relevant event
//! (sign-in, sign-out, identity link, account delete, magic-link
//! mint, magic-link verify-failed). Pure append; no UPDATE / DELETE
//! paths from runtime code. The Phase 6 `lobby db dump` subcommand
//! is the consumer.


use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use ulid::Ulid;

use crate::PlayerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventKind {
    /// Magic-link minted (no claim of who used it yet).
    MagicLinkMint,
    /// Token verified, session minted.
    SignIn,
    /// Verify failed (bad token, reused, expired). `detail` carries
    /// the error variant name for incident triage.
    SignInFailed,
    SignOut,
    IdentityLinked,
    IdentityUnlinked,
    AccountDeleted,
    NewGameCreated,
    /// `POST /auth/email/start` failed BEFORE a mint could happen
    /// (mint_failed, mail_failed, etc). `detail` carries a short
    /// category tag (`mint_failed`, `mail_failed`, …) so ops can
    /// split server-side breakage from token-validation failures
    /// (which keep using `SignInFailed`).
    AuthEmailStartFailed,
}

impl AuditEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AuditEventKind::MagicLinkMint => "magic_link_mint",
            AuditEventKind::SignIn => "sign_in",
            AuditEventKind::SignInFailed => "sign_in_failed",
            AuditEventKind::SignOut => "sign_out",
            AuditEventKind::IdentityLinked => "identity_linked",
            AuditEventKind::IdentityUnlinked => "identity_unlinked",
            AuditEventKind::AccountDeleted => "account_deleted",
            AuditEventKind::NewGameCreated => "new_game_created",
            AuditEventKind::AuthEmailStartFailed => "auth_email_start_failed",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub id: String,
    pub ts: String,
    pub kind: AuditEventKind,
    pub player_id: Option<PlayerId>,
    pub ip: Option<String>,
    pub detail: String,
}

/// Input shape for [`AuditStore::record`]. Fields mirror the row
/// schema except for the auto-generated id + ts.
#[derive(Debug, Clone)]
pub struct NewAuditEvent {
    pub kind: AuditEventKind,
    pub player_id: Option<PlayerId>,
    pub ip: Option<String>,
    pub detail: String,
}

#[async_trait]
pub trait AuditStore: Send + Sync {
    async fn record(&self, event: NewAuditEvent) -> Result<(), sqlx::Error>;

    /// Bounded scan, newest first. Used by the Phase 6 CLI dump.
    async fn list_recent(&self, limit: u32) -> Result<Vec<AuditEvent>, sqlx::Error>;

    /// Count rows matching `(kind, detail)` whose `ts` is at or after
    /// `since_rfc3339`. Backs the rate-limiter — `magic_link_mint`
    /// events store the email in `detail`, so this is the per-email
    /// throttle key.
    async fn recent_count_by_kind_and_detail(
        &self,
        kind: AuditEventKind,
        detail: &str,
        since_rfc3339: &str,
    ) -> Result<u64, sqlx::Error>;

    /// Count rows matching `(kind, ip)` whose `ts` is at or after
    /// `since_rfc3339`. Backs the per-IP throttle — magic-link mints
    /// from a single client IP across many emails.
    async fn recent_count_by_kind_and_ip(
        &self,
        kind: AuditEventKind,
        ip: &str,
        since_rfc3339: &str,
    ) -> Result<u64, sqlx::Error>;
}

pub struct SqliteAuditStore {
    pool: Pool<Sqlite>,
}

impl SqliteAuditStore {
    pub fn from_pool(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

fn player_id_text(id: &PlayerId) -> String {
    format!("{:016X}", id.0)
}

#[async_trait]
impl AuditStore for SqliteAuditStore {
    async fn record(&self, event: NewAuditEvent) -> Result<(), sqlx::Error> {
        let id = Ulid::new().to_string();
        let ts = Utc::now().to_rfc3339();
        let pid = event.player_id.as_ref().map(player_id_text);
        sqlx::query(
            "INSERT INTO audit_events (id, ts, kind, player_id, ip, detail) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(&ts)
        .bind(event.kind.as_str())
        .bind(&pid)
        .bind(&event.ip)
        .bind(&event.detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn recent_count_by_kind_and_detail(
        &self,
        kind: AuditEventKind,
        detail: &str,
        since_rfc3339: &str,
    ) -> Result<u64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_events \
             WHERE kind = ?1 AND detail = ?2 AND ts >= ?3",
        )
        .bind(kind.as_str())
        .bind(detail)
        .bind(since_rfc3339)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0.max(0) as u64)
    }

    async fn recent_count_by_kind_and_ip(
        &self,
        kind: AuditEventKind,
        ip: &str,
        since_rfc3339: &str,
    ) -> Result<u64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_events \
             WHERE kind = ?1 AND ip = ?2 AND ts >= ?3",
        )
        .bind(kind.as_str())
        .bind(ip)
        .bind(since_rfc3339)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0.max(0) as u64)
    }

    async fn list_recent(&self, limit: u32) -> Result<Vec<AuditEvent>, sqlx::Error> {
        let rows: Vec<AuditRow> = sqlx::query_as::<_, AuditRow>(
            "SELECT id, ts, kind, player_id, ip, detail \
             FROM audit_events ORDER BY ts DESC LIMIT ?1",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| AuditEvent {
                id: r.id,
                ts: r.ts,
                kind: parse_kind(&r.kind),
                player_id: r.player_id.and_then(|p| {
                    u64::from_str_radix(&p, 16).ok().map(PlayerId::new)
                }),
                ip: r.ip,
                detail: r.detail,
            })
            .collect())
    }
}

fn parse_kind(s: &str) -> AuditEventKind {
    match s {
        "magic_link_mint" => AuditEventKind::MagicLinkMint,
        "sign_in" => AuditEventKind::SignIn,
        "sign_in_failed" => AuditEventKind::SignInFailed,
        "sign_out" => AuditEventKind::SignOut,
        "identity_linked" => AuditEventKind::IdentityLinked,
        "identity_unlinked" => AuditEventKind::IdentityUnlinked,
        "account_deleted" => AuditEventKind::AccountDeleted,
        "new_game_created" => AuditEventKind::NewGameCreated,
        "auth_email_start_failed" => AuditEventKind::AuthEmailStartFailed,
        // Tolerate unknown kinds — schema can grow without breaking
        // historical reads.
        _ => AuditEventKind::SignInFailed,
    }
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    ts: String,
    kind: String,
    player_id: Option<String>,
    ip: Option<String>,
    detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn fresh_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn record_and_list_round_trip() {
        let store = SqliteAuditStore::from_pool(fresh_pool().await);
        store
            .record(NewAuditEvent {
                kind: AuditEventKind::MagicLinkMint,
                player_id: None,
                ip: Some("127.0.0.1".into()),
                detail: "alice@example.com".into(),
            })
            .await
            .unwrap();
        store
            .record(NewAuditEvent {
                kind: AuditEventKind::SignIn,
                player_id: Some(PlayerId::new(0xDEADBEEFu64)),
                ip: Some("127.0.0.1".into()),
                detail: String::new(),
            })
            .await
            .unwrap();
        let rows = store.list_recent(10).await.unwrap();
        assert_eq!(rows.len(), 2);
        // Newest first — SignIn was recorded second.
        assert_eq!(rows[0].kind, AuditEventKind::SignIn);
        assert_eq!(rows[0].player_id, Some(PlayerId::new(0xDEADBEEFu64)));
        assert_eq!(rows[1].kind, AuditEventKind::MagicLinkMint);
        assert_eq!(rows[1].ip.as_deref(), Some("127.0.0.1"));
    }

    #[tokio::test]
    async fn recent_count_filters_by_kind_detail_window() {
        let store = SqliteAuditStore::from_pool(fresh_pool().await);
        let earlier = "1970-01-01T00:00:00+00:00";
        // Three mints for alice, two for bob.
        for _ in 0..3 {
            store
                .record(NewAuditEvent {
                    kind: AuditEventKind::MagicLinkMint,
                    player_id: None,
                    ip: None,
                    detail: "alice@example.com".into(),
                })
                .await
                .unwrap();
        }
        for _ in 0..2 {
            store
                .record(NewAuditEvent {
                    kind: AuditEventKind::MagicLinkMint,
                    player_id: None,
                    ip: None,
                    detail: "bob@example.com".into(),
                })
                .await
                .unwrap();
        }
        // One sign_in for alice — different kind, must not count.
        store
            .record(NewAuditEvent {
                kind: AuditEventKind::SignIn,
                player_id: None,
                ip: None,
                detail: "alice@example.com".into(),
            })
            .await
            .unwrap();
        let alice_mints = store
            .recent_count_by_kind_and_detail(
                AuditEventKind::MagicLinkMint,
                "alice@example.com",
                earlier,
            )
            .await
            .unwrap();
        assert_eq!(alice_mints, 3);
        let bob_mints = store
            .recent_count_by_kind_and_detail(
                AuditEventKind::MagicLinkMint,
                "bob@example.com",
                earlier,
            )
            .await
            .unwrap();
        assert_eq!(bob_mints, 2);
        // Future window -> 0.
        let future = "9999-01-01T00:00:00+00:00";
        let zero = store
            .recent_count_by_kind_and_detail(
                AuditEventKind::MagicLinkMint,
                "alice@example.com",
                future,
            )
            .await
            .unwrap();
        assert_eq!(zero, 0);
    }

    #[tokio::test]
    async fn list_recent_respects_limit() {
        let store = SqliteAuditStore::from_pool(fresh_pool().await);
        for _ in 0..5 {
            store
                .record(NewAuditEvent {
                    kind: AuditEventKind::SignOut,
                    player_id: None,
                    ip: None,
                    detail: String::new(),
                })
                .await
                .unwrap();
        }
        let rows = store.list_recent(3).await.unwrap();
        assert_eq!(rows.len(), 3);
    }
}
