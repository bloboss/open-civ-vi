//! Bearer-token extractor shared by every `/api/v1/*` handler.
//!
//! This is a thin wrapper over [`crate::server::api_token::resolve_token`]
//! that produces the `(StatusCode, Json<ApiErrorBody>)` tuple Axum expects.

use std::sync::Arc;

use axum::Json;
use axum::http::{HeaderMap, StatusCode};

use crate::server::api_token;
use crate::server::state::AppState;
use open4x_protocol::v1::ids::{CivId, GameId};
use open4x_protocol::v1::web::ApiErrorBody;

pub type ApiError = (StatusCode, Json<ApiErrorBody>);

fn err(status: StatusCode, code: &str, message: Option<&str>) -> ApiError {
    (
        status,
        Json(ApiErrorBody {
            error: code.to_string(),
            message: message.map(|s| s.to_string()),
        }),
    )
}

fn extract_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Resolve `Authorization: Bearer <token>` to its `(GameId, CivId)` pair.
pub fn auth_or_401(
    state: &Arc<AppState>,
    headers: &HeaderMap,
) -> Result<(GameId, CivId), ApiError> {
    let token = extract_token(headers).ok_or_else(|| {
        err(
            StatusCode::UNAUTHORIZED,
            "missing_or_invalid_token",
            Some("missing bearer token"),
        )
    })?;
    api_token::resolve_token(state, token).ok_or_else(|| {
        err(
            StatusCode::UNAUTHORIZED,
            "missing_or_invalid_token",
            Some("invalid or expired token"),
        )
    })
}

/// Build a generic 404 error body.
pub fn not_found(what: &str) -> ApiError {
    err(StatusCode::NOT_FOUND, "not_found", Some(what))
}

/// Build a 400 error body.
pub fn bad_request(code: &str, message: &str) -> ApiError {
    err(StatusCode::BAD_REQUEST, code, Some(message))
}
