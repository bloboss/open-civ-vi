//! Magic-link tokens for the email auth flow.
//!
//! Format: `{base64url(payload)}.{base64url(hmac)}` where `payload`
//! is the UTF-8 string `"{email}|{expires_at_unix}|{nonce}"`.
//! `hmac` is `HMAC-SHA256(payload)` keyed on the per-deployment
//! `MagicLinkSigner` secret (see [`MagicLinkSigner::from_env_or_path`]).
//!
//! Verification:
//! 1. Split into `payload.signature`.
//! 2. Recompute HMAC; constant-time compare with the supplied
//!    signature.
//! 3. Parse and check `expires_at_unix > now`.
//! 4. Atomically consume the nonce in the
//!    `magic_link_nonces` table — single-use enforcement.
//!
//! Phase 2.2 of `book/src/roadmap/accounts-and-login.md`. The
//! mint side does NOT touch persistence (so it can be called before
//! the DB is reachable, e.g. from a CLI tool); only `verify` consumes
//! the nonce.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::{Pool, Sqlite};
use subtle::ConstantTimeEq;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

/// Default magic-link lifetime: 15 minutes.
pub const DEFAULT_TTL: Duration = Duration::minutes(15);

/// Length of the random nonce embedded in each magic link, in bytes.
const NONCE_BYTES: usize = 16;

#[derive(Debug, Error)]
pub enum MagicLinkError {
    #[error("malformed magic-link token")]
    Malformed,
    #[error("magic-link signature did not verify")]
    BadSignature,
    #[error("magic link has expired")]
    Expired,
    #[error("magic link has already been used")]
    Reused,
    #[error("magic-link store: {0}")]
    Store(#[from] sqlx::Error),
    #[error("magic-link key: {0}")]
    Key(#[from] std::io::Error),
}

/// Holds the per-deployment HMAC key. Constructable from a fixed-size
/// 32-byte secret, or via [`from_env_or_path`] which prefers
/// `OPEN4X_LOBBY_HMAC_KEY` (hex) and falls back to a generate-on-disk
/// flow for self-host single-binary deploys.
#[derive(Clone)]
pub struct MagicLinkSigner {
    key: [u8; 32],
}

impl MagicLinkSigner {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Resolve a signer in this order:
    /// 1. `OPEN4X_LOBBY_HMAC_KEY` env var (hex-encoded, 64 chars).
    /// 2. A 32-byte file at `path`. If the file exists, read it.
    /// 3. Generate a fresh 32-byte secret, persist to `path` with
    ///    0600 permissions, and use it.
    pub fn from_env_or_path(path: impl AsRef<Path>) -> Result<Self, MagicLinkError> {
        if let Ok(hex) = std::env::var("OPEN4X_LOBBY_HMAC_KEY") {
            let bytes = hex_decode_32(&hex).ok_or_else(|| {
                MagicLinkError::Key(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "OPEN4X_LOBBY_HMAC_KEY must be 64 hex chars (32 bytes)",
                ))
            })?;
            return Ok(Self::new(bytes));
        }
        let path: PathBuf = path.as_ref().to_path_buf();
        match std::fs::read(&path) {
            Ok(bytes) => {
                let mut key = [0u8; 32];
                if bytes.len() != 32 {
                    return Err(MagicLinkError::Key(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "key file is not 32 bytes",
                    )));
                }
                key.copy_from_slice(&bytes);
                Ok(Self::new(key))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Mint a fresh key and persist it.
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                let key: [u8; 32] = rand::random();
                std::fs::write(&path, key)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
                }
                Ok(Self::new(key))
            }
            Err(e) => Err(MagicLinkError::Key(e)),
        }
    }

    /// Mint a magic-link token for `email` valid for `ttl`. The caller
    /// is responsible for storing the nonce row in the DB BEFORE
    /// emailing — use [`Self::mint_and_record`] for the typical case.
    pub fn mint(&self, email: &str, ttl: Duration) -> MintedToken {
        let now = Utc::now();
        let expires_at = now + ttl;
        let nonce = random_nonce();
        let payload = encode_payload(email, expires_at, &nonce);
        let signature = self.sign(payload.as_bytes());
        let token = format!(
            "{}.{}",
            B64.encode(payload.as_bytes()),
            B64.encode(signature),
        );
        MintedToken {
            token,
            nonce,
            expires_at,
            email: email.to_string(),
        }
    }

    /// Convenience wrapper: mint a token and persist its nonce in
    /// `magic_link_nonces` so [`verify`] can later consume it.
    pub async fn mint_and_record(
        &self,
        pool: &Pool<Sqlite>,
        email: &str,
        ttl: Duration,
    ) -> Result<MintedToken, MagicLinkError> {
        let minted = self.mint(email, ttl);
        sqlx::query(
            "INSERT INTO magic_link_nonces (nonce, email, expires_at) \
             VALUES (?1, ?2, ?3)",
        )
        .bind(&minted.nonce)
        .bind(&minted.email)
        .bind(minted.expires_at.to_rfc3339())
        .execute(pool)
        .await?;
        Ok(minted)
    }

    /// Verify a token end-to-end and consume its single-use nonce.
    /// Returns the email address embedded in the token on success.
    pub async fn verify(&self, pool: &Pool<Sqlite>, token: &str) -> Result<String, MagicLinkError> {
        // Split + decode envelope.
        let (payload_b64, sig_b64) = token.split_once('.').ok_or(MagicLinkError::Malformed)?;
        let payload = B64
            .decode(payload_b64)
            .map_err(|_| MagicLinkError::Malformed)?;
        let supplied_sig = B64.decode(sig_b64).map_err(|_| MagicLinkError::Malformed)?;

        // Recompute HMAC and constant-time compare.
        let expected = self.sign(&payload);
        if expected.ct_eq(&supplied_sig).unwrap_u8() == 0 {
            return Err(MagicLinkError::BadSignature);
        }

        // Parse payload.
        let payload_str = std::str::from_utf8(&payload).map_err(|_| MagicLinkError::Malformed)?;
        let (email, expires_at, nonce) = decode_payload(payload_str)?;

        // Expiry check.
        if Utc::now() >= expires_at {
            return Err(MagicLinkError::Expired);
        }

        // Atomic single-use consumption: UPDATE … WHERE consumed_at IS NULL
        // returns 0 rows on the second call.
        let now = Utc::now().to_rfc3339();
        let res = sqlx::query(
            "UPDATE magic_link_nonces SET consumed_at = ?1 \
             WHERE nonce = ?2 AND consumed_at IS NULL",
        )
        .bind(&now)
        .bind(&nonce)
        .execute(pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(MagicLinkError::Reused);
        }

        Ok(email)
    }

    fn sign(&self, payload: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(payload);
        mac.finalize().into_bytes().to_vec()
    }
}

/// Output of [`MagicLinkSigner::mint`] — the token to email plus
/// metadata for the caller to log / persist.
#[derive(Debug, Clone)]
pub struct MintedToken {
    /// The full `payload.signature` string to embed in the magic link.
    pub token: String,
    /// The random nonce embedded in the token. Must be inserted into
    /// `magic_link_nonces` before the user clicks the link.
    pub nonce: String,
    /// Absolute expiry timestamp.
    pub expires_at: DateTime<Utc>,
    /// The email address the token was minted for (for logging /
    /// `magic_link_nonces.email`).
    pub email: String,
}

// ───────────────────────────── Helpers ────────────────────────────────────────

fn random_nonce() -> String {
    let bytes: [u8; NONCE_BYTES] = rand::random();
    B64.encode(bytes)
}

fn encode_payload(email: &str, expires_at: DateTime<Utc>, nonce: &str) -> String {
    format!("{email}|{}|{nonce}", expires_at.timestamp())
}

fn decode_payload(s: &str) -> Result<(String, DateTime<Utc>, String), MagicLinkError> {
    let mut parts = s.splitn(3, '|');
    let email = parts.next().ok_or(MagicLinkError::Malformed)?.to_string();
    let exp_str = parts.next().ok_or(MagicLinkError::Malformed)?;
    let nonce = parts.next().ok_or(MagicLinkError::Malformed)?.to_string();
    let exp_unix: i64 = exp_str.parse().map_err(|_| MagicLinkError::Malformed)?;
    let expires_at =
        DateTime::<Utc>::from_timestamp(exp_unix, 0).ok_or(MagicLinkError::Malformed)?;
    Ok((email, expires_at, nonce))
}

fn hex_decode_32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

// ───────────────────────────── Tests ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool_with_migrations() -> Pool<Sqlite> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    fn signer() -> MagicLinkSigner {
        MagicLinkSigner::new([7u8; 32])
    }

    #[tokio::test]
    async fn mint_and_verify_round_trip() {
        let pool = pool_with_migrations().await;
        let s = signer();
        let minted = s
            .mint_and_record(&pool, "alice@example.com", DEFAULT_TTL)
            .await
            .unwrap();
        let email = s.verify(&pool, &minted.token).await.unwrap();
        assert_eq!(email, "alice@example.com");
    }

    #[tokio::test]
    async fn second_use_is_rejected_as_reused() {
        let pool = pool_with_migrations().await;
        let s = signer();
        let minted = s
            .mint_and_record(&pool, "alice@example.com", DEFAULT_TTL)
            .await
            .unwrap();
        assert!(s.verify(&pool, &minted.token).await.is_ok());
        let err = s.verify(&pool, &minted.token).await.unwrap_err();
        assert!(matches!(err, MagicLinkError::Reused), "got {err:?}");
    }

    #[tokio::test]
    async fn expired_token_is_rejected() {
        let pool = pool_with_migrations().await;
        let s = signer();
        // Mint with a -1 second TTL → already expired.
        let minted = s
            .mint_and_record(&pool, "alice@example.com", Duration::seconds(-1))
            .await
            .unwrap();
        let err = s.verify(&pool, &minted.token).await.unwrap_err();
        assert!(matches!(err, MagicLinkError::Expired), "got {err:?}");
    }

    #[tokio::test]
    async fn tampered_signature_is_rejected() {
        let pool = pool_with_migrations().await;
        let s = signer();
        let minted = s
            .mint_and_record(&pool, "alice@example.com", DEFAULT_TTL)
            .await
            .unwrap();
        // Flip the last byte of the signature segment.
        let mut t = minted.token;
        let last = t.pop().unwrap();
        let flipped = if last == 'A' { 'B' } else { 'A' };
        t.push(flipped);
        let err = s.verify(&pool, &t).await.unwrap_err();
        assert!(
            matches!(
                err,
                MagicLinkError::BadSignature | MagicLinkError::Malformed
            ),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn unknown_nonce_is_rejected_as_reused() {
        // Token is well-formed and signed by the right key, but its
        // nonce was never inserted into magic_link_nonces. UPDATE
        // affects 0 rows → Reused, the same shape we use for
        // double-spend.
        let pool = pool_with_migrations().await;
        let s = signer();
        let minted = s.mint("alice@example.com", DEFAULT_TTL);
        let err = s.verify(&pool, &minted.token).await.unwrap_err();
        assert!(matches!(err, MagicLinkError::Reused), "got {err:?}");
    }

    #[test]
    fn signer_loads_from_env() {
        let key_hex = "00".repeat(32);
        // Edition-2024 set_var/remove_var are unsafe; tests are
        // single-threaded for this module so the data race risk is
        // moot.
        unsafe {
            std::env::set_var("OPEN4X_LOBBY_HMAC_KEY", &key_hex);
        }
        let s = MagicLinkSigner::from_env_or_path("/tmp/never_read").unwrap();
        assert_eq!(s.key, [0u8; 32]);
        unsafe {
            std::env::remove_var("OPEN4X_LOBBY_HMAC_KEY");
        }
    }
}
