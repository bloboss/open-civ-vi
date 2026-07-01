//! `/api/v1/friends` — list / request / accept / unfriend.
//!
//! v1 takes the *other* PlayerId as a 16-char hex (or canonical
//! dot-grouped form) directly. Identity-search (resolve email /
//! handle / OpenID → PlayerId) is a follow-up route.

#![cfg(feature = "ssr")]

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use open4x_accounts::PlayerId;
use open4x_accounts::friends::{FriendStatus, FriendsError, FriendsStore};
use serde::{Deserialize, Serialize};

use crate::server::AppState;
use crate::server::auth::RequireSession;

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

// ───────────────────────────── wire shape ─────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FriendView {
    /// Other party's PlayerId in canonical dot-grouped hex.
    pub player_id: String,
    pub status: FriendStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct FriendsListResp {
    pub friends: Vec<FriendView>,
}

// ───────────────────────────── helpers ────────────────────────────────────────

/// Accept the canonical `0xAAAA·BBBB·CCCC·DDDD` display form *or*
/// a bare 16-char hex string (no separators, no `0x` prefix).
fn parse_player_id(s: &str) -> Option<PlayerId> {
    let trimmed = s.trim();
    let body = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let stripped: String = body.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if stripped.len() != 16 {
        return None;
    }
    Some(PlayerId::new(u64::from_str_radix(&stripped, 16).ok()?))
}

fn friends_error_response(e: FriendsError) -> Response {
    let (status, code) = match &e {
        FriendsError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
        FriendsError::AlreadyExists => (StatusCode::CONFLICT, "already_exists"),
        FriendsError::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_input"),
        FriendsError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR, "store_error"),
    };
    (
        status,
        Json(ErrorBody {
            error: code,
            message: Some(e.to_string()),
        }),
    )
        .into_response()
}

// ───────────────────────────── GET /friends ───────────────────────────────────

pub async fn list(
    State(state): State<AppState>,
    RequireSession(player_id): RequireSession,
) -> Response {
    match state.friends.list_for(player_id).await {
        Ok(rows) => Json(FriendsListResp {
            friends: rows
                .into_iter()
                .map(|r| FriendView {
                    player_id: r.other_player_id.display(),
                    status: r.status,
                    created_at: r.created_at,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => friends_error_response(e),
    }
}

// ───────────────────────────── POST /friends/request ─────────────────────────

#[derive(Debug, Deserialize)]
pub struct RequestBody {
    pub player_id: String,
}

pub async fn request(
    State(state): State<AppState>,
    RequireSession(me): RequireSession,
    Json(body): Json<RequestBody>,
) -> Response {
    let Some(target) = parse_player_id(&body.player_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "invalid_player_id",
                message: Some("expected 16 hex digits or canonical 0xAAAA·… form".into()),
            }),
        )
            .into_response();
    };
    match state.friends.request(me, target).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"ok": true, "player_id": target.display()})),
        )
            .into_response(),
        Err(e) => friends_error_response(e),
    }
}

// ───────────────────────────── POST /friends/{id}/accept ─────────────────────

pub async fn accept(
    State(state): State<AppState>,
    RequireSession(me): RequireSession,
    Path(other_id): Path<String>,
) -> Response {
    let Some(other) = parse_player_id(&other_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "invalid_player_id",
                message: None,
            }),
        )
            .into_response();
    };
    match state.friends.accept(me, other).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => friends_error_response(e),
    }
}

// ───────────────────────────── POST /friends/search ──────────────────────────

#[derive(Debug, Deserialize)]
pub struct SearchBody {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct SearchHitView {
    pub player_id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResp {
    pub matches: Vec<SearchHitView>,
}

pub async fn search(
    State(state): State<AppState>,
    RequireSession(_me): RequireSession,
    Json(body): Json<SearchBody>,
) -> Response {
    match state.store.search_for_friend(&body.query).await {
        Ok(hits) => Json(SearchResp {
            matches: hits
                .into_iter()
                .map(|h| SearchHitView {
                    player_id: h.player_id.display(),
                    kind: h.kind,
                    label: h.label,
                })
                .collect(),
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: "store_error",
                message: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}

// ───────────────────────────── DELETE /friends/{id} ──────────────────────────

pub async fn unfriend(
    State(state): State<AppState>,
    RequireSession(me): RequireSession,
    Path(other_id): Path<String>,
) -> Response {
    let Some(other) = parse_player_id(&other_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "invalid_player_id",
                message: None,
            }),
        )
            .into_response();
    };
    match state.friends.unfriend(me, other).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => friends_error_response(e),
    }
}
