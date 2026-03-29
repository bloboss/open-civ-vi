//! REST API endpoints for programmatic game data access.
//!
//! All endpoints require `Authorization: Bearer <token>` header.
//! The `/api/game/view` endpoint returns the full `GameView`, enabling custom
//! renderers — any player can write their own frontend against this data.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

use crate::server::api_token;
use crate::server::projection::project_game_view;
use crate::server::reports;
use crate::server::state::AppState;
use crate::types::ids::{CityId, CivId, GameId};

// ── Auth extractor ──────────────────────────────────────────────────────────

fn extract_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

fn auth_or_401(
    state: &Arc<AppState>,
    headers: &HeaderMap,
) -> Result<(GameId, CivId), (StatusCode, &'static str)> {
    let token = extract_token(headers).ok_or((StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    api_token::resolve_token(state, token)
        .ok_or((StatusCode::UNAUTHORIZED, "invalid or expired token"))
}

fn get_view(
    state: &Arc<AppState>,
    game_id: GameId,
    civ_id: CivId,
) -> Result<crate::types::view::GameView, (StatusCode, &'static str)> {
    let room = state
        .games
        .get(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "game not found"))?;
    let libciv_civ_id = libciv::CivId::from_ulid(civ_id.as_ulid());
    Ok(project_game_view(&room.state, libciv_civ_id))
}

// ── Endpoints ───────────────────────────────────────────────────────────────

/// GET /api/game/view — full GameView (enables custom renderers).
pub async fn game_view(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;
    Ok(Json(view))
}

/// GET /api/game/cities — city report rows.
pub async fn cities(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;
    Ok(Json(reports::build_city_report(&view)))
}

/// GET /api/game/city/:id — detailed city report.
pub async fn city_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;

    let city_ulid: ulid::Ulid = id
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid city id"))?;
    let city_id = CityId::from_ulid(city_ulid);

    let city = view
        .cities
        .iter()
        .find(|c| c.id == city_id)
        .ok_or((StatusCode::NOT_FOUND, "city not found"))?;

    Ok(Json(city.clone()))
}

/// GET /api/game/resources — aggregated resource report.
pub async fn resources(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;
    Ok(Json(reports::build_resource_report(&view)))
}

/// GET /api/game/units — unit report.
pub async fn units(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;
    Ok(Json(reports::build_unit_report(&view)))
}

/// GET /api/game/map-stats — terrain, feature, resource counts.
pub async fn map_stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;
    Ok(Json(reports::build_map_stats(&view)))
}

/// GET /api/game/players — opponent data.
pub async fn players(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;
    Ok(Json(reports::build_player_reports(&view)))
}

/// GET /api/game/science — tech tree and progress.
pub async fn science(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;
    Ok(Json(reports::build_science_report(&view)))
}

/// GET /api/game/culture — civic tree and progress.
pub async fn culture(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let view = get_view(&state, game_id, civ_id)?;
    Ok(Json(reports::build_culture_report(&view)))
}

/// GET /api/game/turn — current turn status.
pub async fn turn_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let (game_id, _civ_id) = auth_or_401(&state, &headers)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "game not found"))?;
    let status = reports::build_turn_status(game_id, room.state.turn, room.status, &room.players);
    Ok(Json(status))
}
