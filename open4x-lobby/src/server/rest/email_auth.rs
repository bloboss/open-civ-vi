//! Email magic-link auth handlers.
//!
//! - `POST /api/v1/auth/email/start { email }` — mint a magic link,
//!   record the nonce, hand the link to the configured `Mailer`,
//!   return 202.
//! - `GET /api/v1/auth/email/verify?token=…` — verify the token,
//!   find-or-create the account, mint a session, set the
//!   `lobby_session` cookie, redirect to `/`.
//! - `POST /api/v1/auth/signout` — revoke the current session and
//!   clear the cookie.

#![cfg(feature = "ssr")]

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;

use crate::server::client_ip::client_ip;
use chrono::{Duration, Utc};
use open4x_accounts::audit::{AuditEventKind, AuditStore, NewAuditEvent};
use open4x_accounts::magic_link::{DEFAULT_TTL, MagicLinkError};
use open4x_accounts::session;
use open4x_accounts::store::AccountStore;
use open4x_accounts::Identity;
use serde::{Deserialize, Serialize};

use crate::server::auth::{AuthCookie, SESSION_COOKIE_NAME};
use crate::server::AppState;

// ───────────────────────────── /auth/email/start ──────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StartBody {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct StartResp {
    pub ok: bool,
    pub message: &'static str,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Maximum magic-link mints per email in the last
/// [`MAGIC_LINK_THROTTLE_WINDOW_SECS`] seconds before /email/start
/// returns 429. Keep generous for normal use; tighten if abuse
/// telemetry warrants it.
const MAGIC_LINK_THROTTLE_LIMIT: u64 = 5;
/// Per-IP cap for the same window — higher than per-email because
/// a household / NAT can legitimately request links for a few
/// different addresses.
const MAGIC_LINK_THROTTLE_LIMIT_PER_IP: u64 = 20;
const MAGIC_LINK_THROTTLE_WINDOW_SECS: i64 = 5 * 60;

pub async fn start(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<StartBody>,
) -> Response {
    let email = body.email.trim().to_lowercase();
    if !email.contains('@') || email.len() > 254 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "invalid_email",
                message: None,
            }),
        )
            .into_response();
    }

    // Throttles read the audit log directly — no second in-memory
    // limiter, and the caps survive restarts.
    let since = (Utc::now() - Duration::seconds(MAGIC_LINK_THROTTLE_WINDOW_SECS))
        .to_rfc3339();
    let ip_str = client_ip(addr.ip(), &headers, &state.trusted_proxies);

    if let Ok(count) = state
        .audit
        .recent_count_by_kind_and_detail(AuditEventKind::MagicLinkMint, &email, &since)
        .await
    {
        if count >= MAGIC_LINK_THROTTLE_LIMIT {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(axum::http::header::RETRY_AFTER, "60")],
                Json(ErrorBody {
                    error: "rate_limited",
                    message: Some(format!(
                        "{MAGIC_LINK_THROTTLE_LIMIT} magic-links / {} min for this email — try again later",
                        MAGIC_LINK_THROTTLE_WINDOW_SECS / 60
                    )),
                }),
            )
                .into_response();
        }
    }

    if let Ok(count) = state
        .audit
        .recent_count_by_kind_and_ip(AuditEventKind::MagicLinkMint, &ip_str, &since)
        .await
    {
        if count >= MAGIC_LINK_THROTTLE_LIMIT_PER_IP {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(axum::http::header::RETRY_AFTER, "60")],
                Json(ErrorBody {
                    error: "rate_limited",
                    message: Some(format!(
                        "{MAGIC_LINK_THROTTLE_LIMIT_PER_IP} magic-links / {} min from this IP — try again later",
                        MAGIC_LINK_THROTTLE_WINDOW_SECS / 60
                    )),
                }),
            )
                .into_response();
        }
    }

    let minted = match state
        .signer
        .mint_and_record(&state.pool, &email, DEFAULT_TTL)
        .await
    {
        Ok(m) => m,
        Err(e) => {
            let _ = state
                .audit
                .record(NewAuditEvent {
                    kind: AuditEventKind::AuthEmailStartFailed,
                    player_id: None,
                    ip: Some(ip_str.clone()),
                    detail: format!("mint_failed:{e}"),
                })
                .await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "mint_failed",
                    message: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };

    let link = build_verify_link(&state.public_base_url, &minted.token);
    if let Err(e) = state.mailer.send_magic_link(&email, &link).await {
        let _ = state
            .audit
            .record(NewAuditEvent {
                kind: AuditEventKind::AuthEmailStartFailed,
                player_id: None,
                ip: Some(ip_str.clone()),
                detail: format!("mail_failed:{e}"),
            })
            .await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: "mail_failed",
                message: Some(e.to_string()),
            }),
        )
            .into_response();
    }

    let _ = state
        .audit
        .record(NewAuditEvent {
            kind: AuditEventKind::MagicLinkMint,
            player_id: None,
            ip: Some(ip_str.clone()),
            detail: email.clone(),
        })
        .await;

    (
        StatusCode::ACCEPTED,
        Json(StartResp {
            ok: true,
            message: "magic_link_sent",
        }),
    )
        .into_response()
}

fn build_verify_link(base_url: &str, token: &str) -> String {
    if base_url.is_empty() {
        format!("/api/v1/auth/email/verify?token={token}")
    } else {
        format!(
            "{}/api/v1/auth/email/verify?token={token}",
            base_url.trim_end_matches('/'),
        )
    }
}

// ───────────────────────────── /auth/email/verify ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct VerifyQuery {
    pub token: String,
}

pub async fn verify(
    State(state): State<AppState>,
    Query(q): Query<VerifyQuery>,
) -> Response {
    let email = match state.signer.verify(&state.pool, &q.token).await {
        Ok(e) => e,
        Err(err @ (MagicLinkError::Reused
        | MagicLinkError::Expired
        | MagicLinkError::BadSignature
        | MagicLinkError::Malformed)) => {
            let _ = state
                .audit
                .record(NewAuditEvent {
                    kind: AuditEventKind::SignInFailed,
                    player_id: None,
                    ip: None,
                    detail: format!("{err:?}"),
                })
                .await;
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorBody {
                    error: "invalid_magic_link",
                    message: None,
                }),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "verify_failed",
                    message: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };

    let identity = Identity::Email {
        address: email.clone(),
        verified: true,
        primary: true,
    };
    let account = match state
        .store
        .find_or_create_account_for_identity(identity)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "account_failed",
                    message: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };

    // Phase 5 — every successful magic-link consumption flips the
    // matching email identity's `verified` column to true. The
    // "Verify email" CTA in Profile mints a fresh link for an
    // already-linked email; the click path here marks it verified.
    // Best-effort: a store error here doesn't fail sign-in.
    let _ = state.store.mark_email_verified(&email).await;

    let raw = match session::mint_session(&state.pool, account.player_id, session::DEFAULT_TTL).await {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "session_failed",
                    message: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };

    let _ = state
        .audit
        .record(NewAuditEvent {
            kind: AuditEventKind::SignIn,
            player_id: Some(account.player_id),
            ip: None,
            detail: email.clone(),
        })
        .await;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie_value(SESSION_COOKIE_NAME, raw.as_str(), false)
            .parse()
            .expect("valid cookie value"),
    );
    (headers, Redirect::to("/")).into_response()
}

// ───────────────────────────── /auth/signout ──────────────────────────────────

pub async fn signout(
    State(state): State<AppState>,
    cookie: Option<axum::extract::Extension<AuthCookie>>,
    player_id: Option<axum::extract::Extension<open4x_accounts::PlayerId>>,
) -> Response {
    if let Some(axum::extract::Extension(AuthCookie(raw))) = cookie {
        let _ = session::revoke_session(&state.pool, &raw).await;
    }
    let _ = state
        .audit
        .record(NewAuditEvent {
            kind: AuditEventKind::SignOut,
            player_id: player_id.map(|axum::extract::Extension(p)| p),
            ip: None,
            detail: String::new(),
        })
        .await;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie_value(SESSION_COOKIE_NAME, "", true)
            .parse()
            .expect("valid cookie value"),
    );
    (headers, StatusCode::NO_CONTENT).into_response()
}

/// Build a `Set-Cookie` value. `clear=true` zeroes the value and
/// uses Max-Age=0 so the browser deletes the cookie. We don't add
/// `Secure` so dev (http://localhost) keeps working; production
/// should run behind TLS where the browser will negotiate it
/// regardless.
pub(crate) fn cookie_value(name: &str, value: &str, clear: bool) -> String {
    if clear {
        format!("{name}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0")
    } else {
        // Default 30-day session matches session::DEFAULT_TTL.
        format!("{name}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_verify_link_with_no_base_is_relative() {
        assert_eq!(
            build_verify_link("", "abc.def"),
            "/api/v1/auth/email/verify?token=abc.def"
        );
    }

    #[test]
    fn build_verify_link_uses_base_when_set() {
        assert_eq!(
            build_verify_link("https://lobby.example/", "abc.def"),
            "https://lobby.example/api/v1/auth/email/verify?token=abc.def"
        );
    }

    #[test]
    fn cookie_value_set_and_clear() {
        let set = cookie_value("lobby_session", "raw", false);
        assert!(set.contains("lobby_session=raw"));
        assert!(set.contains("HttpOnly"));
        assert!(set.contains("SameSite=Lax"));
        assert!(set.contains("Max-Age=2592000"));
        let clr = cookie_value("lobby_session", "", true);
        assert!(clr.contains("Max-Age=0"));
    }
}
