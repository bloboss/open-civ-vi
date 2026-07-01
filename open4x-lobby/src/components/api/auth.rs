//! Bindings for `/api/v1/auth/*`.

use serde::{Deserialize, Serialize};

use super::http::{ApiError, fetch_json};

#[derive(Debug, Clone, Serialize)]
pub struct EmailStartBody {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailStartResp {
    pub ok: bool,
    pub message: String,
}

/// `POST /api/v1/auth/email/start`. The browser sends the request
/// over the same origin as the SPA; the lobby returns 202 + a
/// `magic_link_sent` ack.
pub async fn email_start(email: String) -> Result<EmailStartResp, ApiError> {
    let body = EmailStartBody { email };
    fetch_json::<EmailStartResp, EmailStartBody>(
        "POST",
        "/api/v1/auth/email/start",
        Some(&body),
    )
    .await
}

// ───────────────────────────── pubkey auth ───────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PubkeyChallengeBody {
    pub pubkey: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PubkeyChallengeResp {
    pub nonce: String,
    pub expires_in: i64,
}

/// `POST /api/v1/auth/pubkey/challenge`. Returns a single-use nonce
/// (hex) to be signed with the matching Ed25519 private key.
pub async fn pubkey_challenge(pubkey: String) -> Result<PubkeyChallengeResp, ApiError> {
    let body = PubkeyChallengeBody { pubkey };
    fetch_json::<PubkeyChallengeResp, PubkeyChallengeBody>(
        "POST",
        "/api/v1/auth/pubkey/challenge",
        Some(&body),
    )
    .await
}

#[derive(Debug, Clone, Serialize)]
pub struct PubkeyVerifyBody {
    pub pubkey: String,
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PubkeyVerifyResp {
    pub ok: bool,
    pub player_id: String,
}

/// `POST /api/v1/auth/pubkey/verify`. On success the lobby sets the
/// `lobby_session` cookie (the browser stores it); we just need the
/// `ok`/`player_id` ack.
pub async fn pubkey_verify(
    pubkey: String,
    signature: String,
    email: Option<String>,
) -> Result<PubkeyVerifyResp, ApiError> {
    let body = PubkeyVerifyBody {
        pubkey,
        signature,
        email,
    };
    fetch_json::<PubkeyVerifyResp, PubkeyVerifyBody>(
        "POST",
        "/api/v1/auth/pubkey/verify",
        Some(&body),
    )
    .await
}

/// `POST /api/v1/auth/signout`.
pub async fn signout() -> Result<(), ApiError> {
    fetch_json::<serde_json::Value, ()>("POST", "/api/v1/auth/signout", None)
        .await
        .map(|_| ())
}
