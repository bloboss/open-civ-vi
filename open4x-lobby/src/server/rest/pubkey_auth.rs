//! Ed25519 pubkey challenge-response auth handlers.
//!
//! Lightweight, passwordless, secure-by-possession login: the client
//! holds an Ed25519 keypair, asks the server for a one-time nonce, signs
//! it, and posts the signature back. The server verifies the signature
//! against the claimed public key, then mints the same `lobby_session`
//! cookie the magic-link flow uses. The *same* keypair authenticates
//! against the in-game `open4x-server` bearer surface.
//!
//! - `POST /api/v1/auth/pubkey/challenge { pubkey }` — issue a nonce.
//! - `POST /api/v1/auth/pubkey/verify { pubkey, signature, email? }` —
//!   verify the signature, find-or-create the account keyed on the
//!   pubkey identity, optionally link `email` as a secondary identity,
//!   mint a session, set the cookie.
//!
//! Wire encoding: `pubkey` is 32-byte lowercase hex, `signature` is
//! 64-byte hex, `nonce` (in the challenge response) is 32-byte hex.

#![cfg(feature = "ssr")]

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use open4x_accounts::Identity;
use open4x_accounts::audit::{AuditEventKind, AuditStore, NewAuditEvent};
use open4x_accounts::session;
use open4x_accounts::store::AccountStore;

use super::email_auth::cookie_value;
use crate::server::AppState;
use crate::server::auth::SESSION_COOKIE_NAME;

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn err(status: StatusCode, error: &'static str, message: Option<String>) -> Response {
    (status, Json(ErrorBody { error, message })).into_response()
}

/// Decode a fixed-width lowercase-hex field into `[u8; N]`.
fn decode_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let bytes = hex::decode(s.trim()).ok()?;
    bytes.try_into().ok()
}

// ──────────────────────── /auth/pubkey/challenge ───────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChallengeBody {
    /// 32-byte Ed25519 public key, lowercase hex.
    pub pubkey: String,
}

#[derive(Debug, Serialize)]
pub struct ChallengeResp {
    /// 32-byte nonce, lowercase hex, to be signed with the matching key.
    pub nonce: String,
    /// Seconds the challenge stays valid.
    pub expires_in: i64,
}

pub async fn challenge(State(state): State<AppState>, Json(body): Json<ChallengeBody>) -> Response {
    let pubkey_hex = body.pubkey.trim().to_ascii_lowercase();
    // Validate it's a well-formed Ed25519 key before issuing — no point
    // minting a nonce for a key we could never verify against.
    let Some(key_bytes) = decode_hex::<32>(&pubkey_hex) else {
        return err(StatusCode::BAD_REQUEST, "invalid_pubkey", None);
    };
    if VerifyingKey::from_bytes(&key_bytes).is_err() {
        return err(StatusCode::BAD_REQUEST, "invalid_pubkey", None);
    }

    let nonce = state.pubkey_challenges.issue(&pubkey_hex, Utc::now());
    (
        StatusCode::OK,
        Json(ChallengeResp {
            nonce: hex::encode(nonce),
            expires_in: crate::server::pubkey_challenge::CHALLENGE_TTL_SECS,
        }),
    )
        .into_response()
}

// ───────────────────────── /auth/pubkey/verify ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct VerifyBody {
    /// 32-byte Ed25519 public key, lowercase hex.
    pub pubkey: String,
    /// 64-byte signature over the issued nonce, lowercase hex.
    pub signature: String,
    /// Optional email to link as a secondary identity on first sign-in.
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VerifyResp {
    pub ok: bool,
    /// Canonical hex display of the authenticated account's PlayerId.
    pub player_id: String,
}

pub async fn verify(State(state): State<AppState>, Json(body): Json<VerifyBody>) -> Response {
    let pubkey_hex = body.pubkey.trim().to_ascii_lowercase();

    let Some(key_bytes) = decode_hex::<32>(&pubkey_hex) else {
        return err(StatusCode::BAD_REQUEST, "invalid_pubkey", None);
    };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&key_bytes) else {
        return err(StatusCode::BAD_REQUEST, "invalid_pubkey", None);
    };
    let Some(sig_bytes) = decode_hex::<64>(&body.signature) else {
        return err(StatusCode::BAD_REQUEST, "invalid_signature", None);
    };
    let signature = Signature::from_bytes(&sig_bytes);

    // Consume the pending nonce (single use). Absent / expired → 401.
    let Some(nonce) = state.pubkey_challenges.consume(&pubkey_hex, Utc::now()) else {
        record_failed(&state, "no_challenge").await;
        return err(
            StatusCode::UNAUTHORIZED,
            "no_pending_challenge",
            Some("request a fresh challenge and retry".into()),
        );
    };

    if verifying_key.verify(&nonce, &signature).is_err() {
        record_failed(&state, "bad_signature").await;
        return err(StatusCode::UNAUTHORIZED, "bad_signature", None);
    }

    // Signature proves possession — find or create the account keyed on
    // this pubkey identity.
    let identity = Identity::PublicKey {
        ed25519_hex: pubkey_hex.clone(),
        label: String::new(),
    };
    let account = match state
        .store
        .find_or_create_account_for_identity(identity)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            return err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "account_failed",
                Some(e.to_string()),
            );
        }
    };

    // Optionally link an email as a secondary (unverified) identity on
    // first sign-in. Best-effort: a conflict (email already linked
    // elsewhere) or any store error must not fail an otherwise-valid
    // pubkey login. The user can verify the email later via magic-link.
    if let Some(raw_email) = body.email.as_deref() {
        let email = raw_email.trim().to_lowercase();
        if email.contains('@') && email.len() <= 254 {
            let already_linked = account
                .identities
                .iter()
                .any(|i| matches!(i, Identity::Email { address, .. } if address == &email));
            if !already_linked {
                let _ = state
                    .store
                    .link_identity(
                        account.player_id,
                        Identity::Email {
                            address: email,
                            verified: false,
                            primary: false,
                        },
                    )
                    .await;
            }
        }
    }

    let raw =
        match session::mint_session(&state.pool, account.player_id, session::DEFAULT_TTL).await {
            Ok(t) => t,
            Err(e) => {
                return err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "session_failed",
                    Some(e.to_string()),
                );
            }
        };

    let _ = state
        .audit
        .record(NewAuditEvent {
            kind: AuditEventKind::SignIn,
            player_id: Some(account.player_id),
            ip: None,
            detail: format!(
                "pubkey:{}",
                open4x_accounts::pubkey_fingerprint(&pubkey_hex)
            ),
        })
        .await;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie_value(SESSION_COOKIE_NAME, raw.as_str(), false)
            .parse()
            .expect("valid cookie value"),
    );
    (
        headers,
        Json(VerifyResp {
            ok: true,
            player_id: format!("{}", account.player_id),
        }),
    )
        .into_response()
}

async fn record_failed(state: &AppState, detail: &str) {
    let _ = state
        .audit
        .record(NewAuditEvent {
            kind: AuditEventKind::SignInFailed,
            player_id: None,
            ip: None,
            detail: format!("pubkey:{detail}"),
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_hex_roundtrips_and_rejects_bad_width() {
        let key = "ab".repeat(32);
        assert!(decode_hex::<32>(&key).is_some());
        // Wrong width.
        assert!(decode_hex::<32>("abcd").is_none());
        // Non-hex.
        assert!(decode_hex::<32>(&"zz".repeat(32)).is_none());
    }
}
