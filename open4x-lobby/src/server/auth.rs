//! Session-cookie middleware + `RequireSession` extractor.
//!
//! Wire format: `Set-Cookie: lobby_session=<RawToken>; HttpOnly;
//! SameSite=Lax; Path=/; Secure` (Secure dropped in dev when the
//! request was http://). `RawToken` is the unhashed bearer minted
//! by `open4x_accounts::session::mint_session`; the DB only ever
//! stores its SHA-256 hex hash.
//!
//! The middleware always runs (cheap when there's no cookie).
//! Auth-required handlers pull the resolved `PlayerId` via the
//! `RequireSession` extractor; the middleware itself doesn't gate
//! anything — public routes (the SPA, /health, the auth pair)
//! continue to work.

#![cfg(feature = "ssr")]

use axum::Json;
use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::{HeaderMap, Request, StatusCode, header::COOKIE, request::Parts};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use open4x_accounts::PlayerId;
use open4x_accounts::session;
use serde::Serialize;

use super::AppState;

pub const SESSION_COOKIE_NAME: &str = "lobby_session";

/// Wrapper around the raw bearer token pulled out of a cookie. Stored
/// in request extensions when the middleware finds + validates one.
#[derive(Debug, Clone)]
pub struct AuthCookie(pub String);

/// Middleware: parse the lobby_session cookie, validate it, and
/// attach the resulting `PlayerId` to request extensions. Always
/// runs; auth gating happens via the extractor below.
pub async fn session_layer(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    if let Some(token) = extract_cookie(req.headers(), SESSION_COOKIE_NAME) {
        if let Ok(Some(player_id)) = session::validate_session(&state.pool, &token).await {
            req.extensions_mut().insert(player_id);
            req.extensions_mut().insert(AuthCookie(token));
        }
    }
    next.run(req).await
}

/// Auth-required extractor. `Result` lets handlers choose whether to
/// 401 or fall back to a guest path.
pub struct RequireSession(pub PlayerId);

impl<S> FromRequestParts<S> for RequireSession
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<PlayerId>()
            .copied()
            .map(RequireSession)
            .ok_or(AuthRejection)
    }
}

#[derive(Debug, Serialize)]
pub struct AuthErrorBody {
    pub error: &'static str,
}

pub struct AuthRejection;

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(AuthErrorBody {
                error: "no_session",
            }),
        )
            .into_response()
    }
}

// ───────────────────────────── Cookie parser ──────────────────────────────────

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(COOKIE)?.to_str().ok()?;
    for entry in raw.split(';') {
        let entry = entry.trim();
        if let Some(rest) = entry.strip_prefix(name) {
            if let Some(value) = rest.strip_prefix('=') {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with_cookie(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(COOKIE, HeaderValue::from_str(value).unwrap());
        h
    }

    #[test]
    fn extracts_named_cookie() {
        let h = headers_with_cookie("foo=bar; lobby_session=abc.def; baz=qux");
        assert_eq!(
            extract_cookie(&h, SESSION_COOKIE_NAME),
            Some("abc.def".to_string())
        );
    }

    #[test]
    fn missing_cookie_returns_none() {
        let h = headers_with_cookie("foo=bar");
        assert_eq!(extract_cookie(&h, SESSION_COOKIE_NAME), None);
    }

    #[test]
    fn empty_cookie_value_is_returned_as_empty() {
        let h = headers_with_cookie("lobby_session=");
        assert_eq!(extract_cookie(&h, SESSION_COOKIE_NAME), Some(String::new()));
    }
}
