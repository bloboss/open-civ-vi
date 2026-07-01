//! Session-token mint / validate / revoke.
//!
//! Bearer shape: `lobby_<base64url(48 bytes of OS randomness)>`. The
//! raw token is shown to the user once (HTTP cookie / Bearer header);
//! the DB only ever stores its SHA-256 hash, so a database compromise
//! cannot be used to mint logins.
//!
//! Phase 2.5 of `book/src/roadmap/accounts-and-login.md`. Reordered
//! ahead of Phase 2.3 (OIDC) so the session-cookie middleware on the
//! lobby has a concrete `RawToken` to mint into.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite};
use thiserror::Error;

use crate::PlayerId;

/// Default session lifetime: 30 days.
pub const DEFAULT_TTL: Duration = Duration::days(30);

/// Length of the random material in each token, in bytes.
const RAW_BYTES: usize = 48;

const TOKEN_PREFIX: &str = "lobby_";

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("storage backend: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("invalid input: {0}")]
    Invalid(&'static str),
}

/// A freshly-minted session token. The raw bearer is intentionally
/// not `Display` — callers should hand it to a cookie / header
/// helper rather than logging it.
#[derive(Debug, Clone)]
pub struct RawToken(String);

impl RawToken {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// Mint a fresh session row for `player_id` with the given TTL.
/// Returns the raw bearer the caller hands to the client; the DB row
/// stores only the SHA-256 hex of the token.
pub async fn mint_session(
    pool: &Pool<Sqlite>,
    player_id: PlayerId,
    ttl: Duration,
) -> Result<RawToken, SessionError> {
    let bytes: [u8; RAW_BYTES] = rand::random();
    let raw = format!("{TOKEN_PREFIX}{}", B64.encode(bytes));
    let token_hash = sha256_hex(raw.as_bytes());
    let player_id_text = format!("{:016X}", player_id.0);
    let now = Utc::now();
    let expires_at = now + ttl;
    sqlx::query(
        "INSERT INTO sessions (token_hash, player_id, created_at, expires_at, revoked_at) \
         VALUES (?1, ?2, ?3, ?4, NULL)",
    )
    .bind(&token_hash)
    .bind(&player_id_text)
    .bind(now.to_rfc3339())
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(RawToken(raw))
}

/// Validate a bearer token. Returns the owning `PlayerId` iff the
/// token exists, hasn't expired, and hasn't been revoked.
///
/// The SHA-256 hash is the lookup key, so this is a single indexed
/// SELECT — there's no need for a constant-time compare against the
/// full `sessions` table.
pub async fn validate_session(
    pool: &Pool<Sqlite>,
    raw: &str,
) -> Result<Option<PlayerId>, SessionError> {
    if !raw.starts_with(TOKEN_PREFIX) {
        return Ok(None);
    }
    let token_hash = sha256_hex(raw.as_bytes());
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT player_id, expires_at, revoked_at FROM sessions WHERE token_hash = ?1",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    let Some((player_id_text, expires_at, revoked_at)) = row else {
        return Ok(None);
    };
    if revoked_at.is_some() {
        return Ok(None);
    }
    let exp = DateTime::parse_from_rfc3339(&expires_at)
        .map_err(|_| SessionError::Invalid("expires_at"))?;
    if exp.with_timezone(&Utc) <= Utc::now() {
        return Ok(None);
    }
    let raw_u64 =
        u64::from_str_radix(&player_id_text, 16).map_err(|_| SessionError::Invalid("player_id"))?;
    Ok(Some(PlayerId::new(raw_u64)))
}

/// Revoke a single session. Idempotent — revoking an already-revoked
/// or non-existent token is a no-op.
pub async fn revoke_session(pool: &Pool<Sqlite>, raw: &str) -> Result<(), SessionError> {
    if !raw.starts_with(TOKEN_PREFIX) {
        return Ok(());
    }
    let token_hash = sha256_hex(raw.as_bytes());
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE sessions SET revoked_at = ?1 \
         WHERE token_hash = ?2 AND revoked_at IS NULL",
    )
    .bind(&now)
    .bind(&token_hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Revoke every active session for a given player — used by the
/// "sign out everywhere" surface.
pub async fn revoke_all_for_player(
    pool: &Pool<Sqlite>,
    player_id: PlayerId,
) -> Result<u64, SessionError> {
    let player_id_text = format!("{:016X}", player_id.0);
    let now = Utc::now().to_rfc3339();
    let res = sqlx::query(
        "UPDATE sessions SET revoked_at = ?1 \
         WHERE player_id = ?2 AND revoked_at IS NULL",
    )
    .bind(&now)
    .bind(&player_id_text)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut s = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

// ───────────────────────────── Tests ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    async fn fresh_pool_and_player() -> (Pool<Sqlite>, PlayerId) {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        // Insert a stub account so the FK passes.
        let player_id = PlayerId::new(0xCAFEF00DCAFEF00D);
        sqlx::query(
            "INSERT INTO accounts (player_id, preferred_name, pronouns, bio, \
                                   prefs_json, created_at, updated_at) \
             VALUES (?1, '', '', '', '{}', ?2, ?2)",
        )
        .bind(format!("{:016X}", player_id.0))
        .bind(Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
        (pool, player_id)
    }

    #[tokio::test]
    async fn mint_validate_round_trip() {
        let (pool, player_id) = fresh_pool_and_player().await;
        let raw = mint_session(&pool, player_id, DEFAULT_TTL).await.unwrap();
        assert!(raw.as_str().starts_with("lobby_"));
        let resolved = validate_session(&pool, raw.as_str()).await.unwrap();
        assert_eq!(resolved, Some(player_id));
    }

    #[tokio::test]
    async fn revoked_session_does_not_validate() {
        let (pool, player_id) = fresh_pool_and_player().await;
        let raw = mint_session(&pool, player_id, DEFAULT_TTL).await.unwrap();
        revoke_session(&pool, raw.as_str()).await.unwrap();
        let resolved = validate_session(&pool, raw.as_str()).await.unwrap();
        assert_eq!(resolved, None);
    }

    #[tokio::test]
    async fn expired_session_does_not_validate() {
        let (pool, player_id) = fresh_pool_and_player().await;
        let raw = mint_session(&pool, player_id, Duration::seconds(-1))
            .await
            .unwrap();
        let resolved = validate_session(&pool, raw.as_str()).await.unwrap();
        assert_eq!(resolved, None);
    }

    #[tokio::test]
    async fn unknown_token_does_not_validate() {
        let (pool, _) = fresh_pool_and_player().await;
        let resolved = validate_session(&pool, "lobby_does_not_exist_at_all")
            .await
            .unwrap();
        assert_eq!(resolved, None);
        // Wrong prefix — refuse without even consulting the DB.
        let resolved = validate_session(&pool, "bearer_other").await.unwrap();
        assert_eq!(resolved, None);
    }

    #[tokio::test]
    async fn revoke_all_kills_every_session() {
        let (pool, player_id) = fresh_pool_and_player().await;
        let a = mint_session(&pool, player_id, DEFAULT_TTL).await.unwrap();
        let b = mint_session(&pool, player_id, DEFAULT_TTL).await.unwrap();
        let killed = revoke_all_for_player(&pool, player_id).await.unwrap();
        assert_eq!(killed, 2);
        assert_eq!(validate_session(&pool, a.as_str()).await.unwrap(), None);
        assert_eq!(validate_session(&pool, b.as_str()).await.unwrap(), None);
    }

    // Note: ON DELETE CASCADE behaviour for sessions when an
    // account is deleted is enforced by the FK declaration in
    // 0001_initial.sql and exercised by the live store path; we
    // don't add a redundant runtime test here.
}
