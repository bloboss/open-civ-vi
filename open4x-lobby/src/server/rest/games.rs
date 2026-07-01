//! `/api/v1/games` — list / create / read / soft-delete game rows.
//!
//! Phase 4.2 of `book/src/roadmap/accounts-and-login.md`. The
//! orchestrator (Phase 4.3) is still pending: `POST /api/v1/games`
//! creates a record but leaves `server_url` + `server_token`
//! empty so the lobby can wire the wizard end-to-end against this
//! surface today, and the orchestrator fills them in later.
//! `POST /games/{id}/resume` returns 503 until that's done.

#![cfg(feature = "ssr")]

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use open4x_accounts::audit::{AuditEventKind, AuditStore, NewAuditEvent};
use open4x_accounts::games::{GameRecord, GameStatus, GameStore, GameStoreError, NewGame};
use serde::{Deserialize, Serialize};

use crate::server::AppState;
use crate::server::auth::RequireSession;
use crate::server::orchestrator::{self, NewGameRequest};
use crate::server::process::DeployMode;

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

// ───────────────────────────── Wire shape ────────────────────────────────────

/// What the browser sees. Mirrors `GameRecord` but never exposes
/// `server_token` (the lobby-only secret used at Resume time).
#[derive(Debug, Serialize)]
pub struct GameView {
    pub game_id: String,
    pub owner_player_id: String,
    pub name: String,
    pub leader: String,
    pub civ_id: String,
    pub difficulty: String,
    pub players_human: u32,
    pub players_ai: u32,
    pub map_type: String,
    pub map_size: String,
    pub seed: String,
    pub turn: u32,
    pub era: String,
    pub score: i32,
    pub status: GameStatus,
    pub server_url: String,
    pub last_played_at: Option<String>,
    pub created_at: String,
    pub notes: String,
}

impl From<GameRecord> for GameView {
    fn from(g: GameRecord) -> Self {
        Self {
            game_id: g.game_id,
            owner_player_id: g.owner_player_id.display(),
            name: g.name,
            leader: g.leader,
            civ_id: g.civ_id,
            difficulty: g.difficulty,
            players_human: g.players_human,
            players_ai: g.players_ai,
            map_type: g.map_type,
            map_size: g.map_size,
            seed: g.seed,
            turn: g.turn,
            era: g.era,
            score: g.score,
            status: g.status,
            server_url: g.server_url,
            last_played_at: g.last_played_at,
            created_at: g.created_at,
            notes: g.notes,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GamesListResp {
    pub games: Vec<GameView>,
}

// ───────────────────────────── GET /api/v1/games ──────────────────────────────

pub async fn list(
    State(state): State<AppState>,
    RequireSession(player_id): RequireSession,
) -> Response {
    match state.games.list_for_player(player_id).await {
        Ok(rows) => Json(GamesListResp {
            games: rows.into_iter().map(GameView::from).collect(),
        })
        .into_response(),
        Err(e) => store_error_response(e),
    }
}

// ───────────────────────────── POST /api/v1/games ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateGameBody {
    pub name: String,
    pub leader: String,
    pub civ_id: String,
    pub difficulty: String,
    pub players_human: u32,
    pub players_ai: u32,
    pub map_type: String,
    pub map_size: String,
    pub seed: String,
}

pub async fn create(
    State(state): State<AppState>,
    RequireSession(player_id): RequireSession,
    Json(body): Json<CreateGameBody>,
) -> Response {
    if body.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "name_required",
                message: None,
            }),
        )
            .into_response();
    }

    // Best-effort orchestration: in shared mode we POST to the
    // single configured `open4x-server`; in per-game mode we spawn
    // a fresh server per row. Failure here doesn't fail the lobby
    // write — the game row still lands with empty server_url /
    // server_token, and Resume returns 503 until a follow-up
    // RetryBootstrap path exists.
    let map_dims = map_size_to_dims(&body.map_size);
    let req = NewGameRequest {
        display_name: Some(body.name.clone()),
        width: Some(map_dims.0),
        height: Some(map_dims.1),
        seed: parse_seed(&body.seed),
        num_ai: Some(body.players_ai),
        turn_limit: None,
    };
    let (server_url, server_token) = match state.deploy_mode {
        DeployMode::PerGame => match state.process_orch.as_ref() {
            Some(orch) => match orch.bootstrap_per_game(&req).await {
                Ok(boot) => (boot.server_url, boot.server_token),
                Err(e) => {
                    eprintln!("[orchestrator] per-game bootstrap failed: {e}");
                    (String::new(), String::new())
                }
            },
            None => (String::new(), String::new()),
        },
        DeployMode::Shared => {
            if state.game_server_url.is_empty() {
                (String::new(), String::new())
            } else {
                match orchestrator::bootstrap_game(&state.game_server_url, &req).await {
                    Ok(boot) => (boot.server_url, boot.server_token),
                    Err(e) => {
                        eprintln!("[orchestrator] bootstrap failed: {e}");
                        (String::new(), String::new())
                    }
                }
            }
        }
    };

    let new = NewGame {
        owner_player_id: player_id,
        name: body.name,
        leader: body.leader,
        civ_id: body.civ_id,
        difficulty: body.difficulty,
        players_human: body.players_human,
        players_ai: body.players_ai,
        map_type: body.map_type,
        map_size: body.map_size,
        seed: body.seed,
        server_url,
        server_token,
    };
    match state.games.create_game(new).await {
        Ok(g) => {
            let _ = state
                .audit
                .record(NewAuditEvent {
                    kind: AuditEventKind::NewGameCreated,
                    player_id: Some(player_id),
                    ip: None,
                    detail: g.game_id.clone(),
                })
                .await;
            (StatusCode::CREATED, Json(GameView::from(g))).into_response()
        }
        Err(e) => store_error_response(e),
    }
}

/// Map the wire's coarse `map_size` enum into width/height tile counts
/// matching the JSX wizard's `map size` Popup ("duel 44×26 · tiny
/// 60×38 · small 74×46 · std 84×54 · large 96×60 · huge 106×66").
fn map_size_to_dims(size: &str) -> (u32, u32) {
    match size {
        "duel" => (44, 26),
        "tiny" => (60, 38),
        "small" => (74, 46),
        "large" => (96, 60),
        "huge" => (106, 66),
        _ => (84, 54), // std default
    }
}

/// Parse the wizard's seed string (e.g. `"0xCAFE·B33F·1A77"`) into a
/// u64 by ignoring non-hex characters. Returns `None` if no hex
/// digits remain — open4x-server will pick its own seed.
fn parse_seed(s: &str) -> Option<u64> {
    let hex: String = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(16) // 64-bit ceiling
        .collect();
    if hex.is_empty() {
        None
    } else {
        u64::from_str_radix(&hex, 16).ok()
    }
}

// ───────────────────────────── GET /api/v1/games/{id} ─────────────────────────

pub async fn get_one(
    State(state): State<AppState>,
    RequireSession(player_id): RequireSession,
    Path(game_id): Path<String>,
) -> Response {
    match state.games.get_game(&game_id).await {
        Ok(Some(g)) if g.owner_player_id == player_id => Json(GameView::from(g)).into_response(),
        Ok(Some(_)) => (
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "not_a_member",
                message: None,
            }),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody {
                error: "game_not_found",
                message: None,
            }),
        )
            .into_response(),
        Err(e) => store_error_response(e),
    }
}

// ───────────────────────────── DELETE /api/v1/games/{id} ──────────────────────

pub async fn delete_one(
    State(state): State<AppState>,
    RequireSession(player_id): RequireSession,
    Path(game_id): Path<String>,
) -> Response {
    match state.games.soft_delete(&game_id, player_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => store_error_response(e),
    }
}

// ───────────────────────────── POST /games/{id}/notes ────────────────────────

#[derive(Debug, Deserialize)]
pub struct NotesBody {
    pub notes: String,
}

pub async fn set_notes(
    State(state): State<AppState>,
    RequireSession(player_id): RequireSession,
    Path(game_id): Path<String>,
    Json(body): Json<NotesBody>,
) -> Response {
    // Owner-only.
    let owns = match state.games.get_game(&game_id).await {
        Ok(Some(g)) => g.owner_player_id == player_id,
        Ok(None) => return store_error_response(GameStoreError::NotFound),
        Err(e) => return store_error_response(e),
    };
    if !owns {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                error: "not_a_member",
                message: None,
            }),
        )
            .into_response();
    }
    if body.notes.len() > 16 * 1024 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "notes_too_long",
                message: Some("notes capped at 16 KiB".into()),
            }),
        )
            .into_response();
    }
    match state.games.set_notes(&game_id, &body.notes).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => store_error_response(e),
    }
}

// ───────────────────────── GET /games/{id}/thumbnail ─────────────────────────

/// Server-side proxy that asks `<server_url>/api/v1/world/snapshot`
/// for the current game's tile array and reduces it to a minimap
/// grid the SPA can render. Owner-gated. The bearer hop uses the
/// games row's stored `server_token` so the browser never has to
/// see it.
#[derive(Debug, Serialize)]
pub struct ThumbnailResp {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<ThumbCell>,
}

#[derive(Debug, Serialize)]
pub struct ThumbCell {
    pub q: i32,
    pub r: i32,
    pub terrain: String,
    /// True when the tile is currently in fog-of-war for the
    /// viewing civ. The SPA dims these cells.
    #[serde(default)]
    pub fog: bool,
    /// Three-state ownership relative to the requesting player:
    /// `Some(true)`  — owned by my civ
    /// `Some(false)` — owned by *another* civ
    /// `None`        — unowned (neutral)
    pub owned_by_me: Option<bool>,
    /// True when a city centre sits on this tile.
    #[serde(default)]
    pub city: bool,
    /// True when the city on this tile is its civ's capital.
    #[serde(default)]
    pub capital: bool,
}

pub async fn thumbnail(
    State(state): State<AppState>,
    RequireSession(player_id): RequireSession,
    Path(game_id): Path<String>,
) -> Response {
    let game = match state.games.get_game(&game_id).await {
        Ok(Some(g)) if g.owner_player_id == player_id => g,
        Ok(Some(_)) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorBody {
                    error: "not_a_member",
                    message: None,
                }),
            )
                .into_response();
        }
        Ok(None) => return store_error_response(GameStoreError::NotFound),
        Err(e) => return store_error_response(e),
    };
    if game.server_url.is_empty() || game.server_token.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "orchestrator_not_ready",
                message: Some("game has no backing server yet".into()),
            }),
        )
            .into_response();
    }
    // For per-game mode: server_url stored on the row may be the
    // public template-rendered form (https://g-4501.example.com)
    // which the lobby itself can't always reach. The orchestrator
    // tracks the loopback URL separately; for v1 we just hit the
    // stored URL and document the constraint.
    let url = format!(
        "{}/api/v1/world/snapshot",
        game.server_url.trim_end_matches('/')
    );
    let client = reqwest::Client::new();
    let resp = match client
        .get(&url)
        .bearer_auth(&game.server_token)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[thumbnail] transport error for {url}: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorBody {
                    error: "upstream_unreachable",
                    message: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::BAD_GATEWAY,
            Json(ErrorBody {
                error: "upstream_status",
                message: Some(format!("{status}: {body}")),
            }),
        )
            .into_response();
    }
    let snap: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorBody {
                    error: "upstream_decode",
                    message: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };
    let world = snap.get("world").cloned().unwrap_or_default();
    let width = world.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let height = world.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    // viewer's civ id from the lobby's games row — used to label
    // each tile as "mine / foreign / neutral".
    let my_civ = game.civ_id.clone();
    let cells: Vec<ThumbCell> = snap
        .get("tiles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| {
            let q = t.get("q").and_then(|v| v.as_i64())? as i32;
            let r = t.get("r").and_then(|v| v.as_i64())? as i32;
            let terrain = t
                .get("terrain")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let fog = t.get("fog").and_then(|v| v.as_bool()).unwrap_or(false);
            let owner = t.get("owner").and_then(|v| v.as_str()).map(str::to_string);
            let owned_by_me = owner.map(|o| o == my_civ);
            let city_obj = t.get("city");
            let city = city_obj.is_some_and(|c| !c.is_null());
            let capital = city_obj
                .and_then(|c| c.get("capital"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Some(ThumbCell {
                q,
                r,
                terrain,
                fog,
                owned_by_me,
                city,
                capital,
            })
        })
        .collect();
    Json(ThumbnailResp {
        width,
        height,
        cells,
    })
    .into_response()
}

// ───────────────────────────── POST /games/{id}/resume ────────────────────────

pub async fn resume(
    State(state): State<AppState>,
    RequireSession(player_id): RequireSession,
    Path(game_id): Path<String>,
) -> Response {
    let game = match state.games.get_game(&game_id).await {
        Ok(Some(g)) if g.owner_player_id == player_id => g,
        Ok(Some(_)) => {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorBody {
                    error: "not_a_member",
                    message: None,
                }),
            )
                .into_response();
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    error: "game_not_found",
                    message: None,
                }),
            )
                .into_response();
        }
        Err(e) => return store_error_response(e),
    };

    if game.server_url.is_empty() || game.server_token.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "orchestrator_not_ready",
                message: Some(
                    "POST /games/{id}/resume needs the Phase 4.3 orchestrator wired to populate server_url and server_token."
                        .into(),
                ),
            }),
        )
            .into_response();
    }
    let _ = state.games.touch_last_played(&game.game_id).await;
    // For now: return the URL + a one-shot token for the client to
    // attach. Phase 4.3 may turn this into a 302 with a session-bridge
    // cookie instead.
    Json(ResumeResp {
        url: game.server_url,
        token: game.server_token,
    })
    .into_response()
}

#[derive(Debug, Serialize)]
struct ResumeResp {
    url: String,
    token: String,
}

// ───────────────────────────── error mapping ──────────────────────────────────

fn store_error_response(e: GameStoreError) -> Response {
    let (status, code) = match &e {
        GameStoreError::NotFound => (StatusCode::NOT_FOUND, "game_not_found"),
        GameStoreError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
        GameStoreError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR, "store_error"),
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
