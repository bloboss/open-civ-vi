//! REST handler functions for `/api/v1/*`.
//!
//! Phase 1 wires the HUD MVP: `health`, `player_state`, `world_snapshot`,
//! `world_tile`, and `end_turn`. Subsequent phases extend the surface from
//! `book/src/roadmap/web-ui.md` §4.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::server::api_token::{ApiTokenRecord, generate_token};
use crate::server::projection::project_game_view;
use crate::server::rest::auth::{ApiError, auth_or_401};
use crate::server::state::{AppState, GameRoom, GameRoomConfig, PlayerRecord};
use crate::server::web_projection;
use open4x_protocol::v1::ids::{CivId, GameId};
use open4x_protocol::v1::messages::{CreateGameRequest, GameStatus};
use open4x_protocol::v1::view::GameView;
use open4x_protocol::v1::web::{MutationResponse, TurnStatusBlock};

#[derive(Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    pub ok: bool,
    pub api: &'static str,
}

/// `GET /api/v1/health` — unauthenticated liveness check.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        api: "v1",
    })
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Project a fresh `GameView` for the authenticated player and pull the
/// configured `turn_limit` from the room.
fn view_and_turn_limit(
    state: &Arc<AppState>,
    game_id: GameId,
    civ_id: CivId,
) -> Result<(GameView, Option<u32>), ApiError> {
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    let libciv_civ_id = libciv::CivId::from_ulid(civ_id.as_ulid());
    let view = project_game_view(&room.state, libciv_civ_id);
    Ok((view, room.config.turn_limit))
}

fn turn_status_block(state: &Arc<AppState>, game_id: GameId) -> TurnStatusBlock {
    let turn = state.games.get(&game_id).map(|r| r.state.turn).unwrap_or(0);
    TurnStatusBlock { turn, ended: false }
}

// ── POST /games/new — bootstrap a single-player game over REST ───────────────

#[derive(Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NewGameRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub num_ai: Option<u32>,
    #[serde(default)]
    pub turn_limit: Option<u32>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NewGameResponse {
    pub game_id: GameId,
    pub civ_id: CivId,
    pub token: String,
    pub turn: u32,
}

/// `POST /api/v1/games/new` — create a fresh single-player game and mint a
/// bearer token for it. Unauthenticated; intended for the single-player REST
/// loop (multiplayer keeps using the WS auth handshake).
pub async fn new_game(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NewGameRequest>,
) -> Result<impl IntoResponse, ApiError> {
    use rand::Rng;

    let display_name = req.display_name.clone().unwrap_or_else(|| "Player".into());

    // Generate an anonymous pubkey for this single-player session.
    let mut rng = rand::rng();
    let pubkey: [u8; 32] = rng.random();

    // Register a PlayerRecord so build_server_session can look it up.
    state.players.insert(
        pubkey,
        PlayerRecord {
            pubkey,
            display_name: display_name.clone(),
            selected_template: state.templates[0].id,
            games_played: 0,
        },
    );

    let game_id = GameId::from_ulid(ulid::Ulid::new());

    let create_req = CreateGameRequest {
        name: format!("{display_name}'s game"),
        width: req.width.unwrap_or(40),
        height: req.height.unwrap_or(24),
        seed: req.seed.unwrap_or(42),
        num_ai: req.num_ai.unwrap_or(1),
        max_players: 1,
        turn_limit: req.turn_limit.or(Some(500)),
    };

    let session =
        crate::server::session::build_server_session(&create_req, &pubkey, &state, game_id);
    let civ_id = session.players.first().map(|s| s.civ_id).ok_or_else(|| {
        crate::server::rest::auth::bad_request("no_player_slot", "session has no player")
    })?;

    let (tx, _rx) = broadcast::channel(64);
    let room = GameRoom {
        game_id,
        name: create_req.name.clone(),
        state: session.state,
        rules: libciv::DefaultRulesEngine,
        players: session.players,
        ai_agents: session.ai_agents,
        status: GameStatus::InProgress,
        config: GameRoomConfig {
            max_players: 1,
            turn_limit: create_req.turn_limit,
        },
        tx,
        notifications: Default::default(),
    };

    let initial_turn = room.state.turn;
    state.games.insert(game_id, room);

    // Mint and store the bearer token.
    let token = generate_token();
    state.api_tokens.insert(
        token.clone(),
        ApiTokenRecord {
            token: token.clone(),
            pubkey,
            game_id,
            civ_id: CivId::from_ulid(civ_id.as_ulid()),
        },
    );

    Ok((
        StatusCode::CREATED,
        Json(NewGameResponse {
            game_id,
            civ_id: CivId::from_ulid(civ_id.as_ulid()),
            token,
            turn: initial_turn,
        }),
    ))
}

// ── /player-state ────────────────────────────────────────────────────────────

pub async fn player_state(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, turn_limit) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_player_state(&view, turn_limit)))
}

// ── /world/snapshot ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorldSnapshotQuery {
    pub q: Option<i32>,
    pub r: Option<i32>,
    pub radius: Option<u32>,
}

pub async fn world_snapshot(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WorldSnapshotQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let q = params.q.unwrap_or(0);
    let r = params.r.unwrap_or(0);
    // Default radius 0 = "all explored". The plan caps the radius at 32.
    let radius = params.radius.unwrap_or(0).min(32);
    Ok(Json(web_projection::build_world_snapshot(
        &view, q, r, radius,
    )))
}

// ── /world/tile/{q}/{r} ──────────────────────────────────────────────────────

pub async fn world_tile(
    State(state): State<Arc<AppState>>,
    Path((q, r)): Path<(i32, i32)>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let snapshot = web_projection::build_world_snapshot(&view, q, r, 1);
    snapshot
        .tiles
        .into_iter()
        .find(|t| t.q == q && t.r == r)
        .map(Json)
        .ok_or_else(|| crate::server::rest::auth::not_found("tile not in player view"))
}

// ── POST /turn/end ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EndTurnView {
    pub turn: u32,
}

pub async fn end_turn(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<axum::response::Response, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ_id = libciv::CivId::from_ulid(civ_id.as_ulid());

    // Block end-turn if any turn-queue item is required (per plan §4.3).
    // Returns 400 with the structured body { error, items } per the spec.
    let view = view_only(&state, game_id, civ_id)?;
    let queue = {
        let room = state
            .games
            .get(&game_id)
            .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
        web_projection::build_turn_queue_from_room(&view, &room, civ_id)
    };
    let required: Vec<_> = queue.items.into_iter().filter(|i| i.required).collect();
    if !required.is_empty() {
        let body = serde_json::json!({
            "error": "unresolved_required_actions",
            "items": required,
        });
        return Ok((StatusCode::BAD_REQUEST, Json(body)).into_response());
    }

    let mut room = state
        .games
        .get_mut(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;

    // Mark this player's slot submitted.
    if let Some(slot) = room.players.iter_mut().find(|s| s.civ_id == libciv_civ_id) {
        slot.submitted_turn = true;
    }

    // For Phase 1 single-player, advance immediately. Multiplayer gating
    // (room.all_submitted()) lives on the WS path and is out of scope here.
    room.resolve_turn();

    let new_turn = room.state.turn;
    drop(room);

    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view: EndTurnView { turn: new_turn },
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    )
        .into_response())
}

// ── /cities ──────────────────────────────────────────────────────────────────

pub async fn cities(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_cities(&view)))
}

pub async fn city_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let cities = web_projection::build_cities(&view);
    cities
        .cities
        .into_iter()
        .find(|c| c.id == id)
        .map(Json)
        .ok_or_else(|| crate::server::rest::auth::not_found("city not found"))
}

pub async fn city_tiles(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    web_projection::build_city_tiles(&view, &id)
        .map(Json)
        .ok_or_else(|| crate::server::rest::auth::not_found("city not found"))
}

// ── /units ───────────────────────────────────────────────────────────────────

pub async fn units(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    Ok(Json(web_projection::build_units_from_room(&view, &room)))
}

pub async fn unit_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    let units = web_projection::build_units_from_room(&view, &room);
    units
        .units
        .into_iter()
        .find(|u| u.id == id)
        .map(Json)
        .ok_or_else(|| crate::server::rest::auth::not_found("unit not found"))
}

// ── /armies (stub for Phase 4) ───────────────────────────────────────────────

pub async fn armies(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_armies(&view)))
}

// ── /combat/preview ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CombatPreviewQuery {
    pub attacker_id: String,
    pub defender_q: i32,
    pub defender_r: i32,
}

pub async fn combat_preview(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CombatPreviewQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    web_projection::build_combat_preview_from_room(
        &view,
        &room,
        &params.attacker_id,
        params.defender_q,
        params.defender_r,
    )
    .map(Json)
    .ok_or_else(|| crate::server::rest::auth::not_found("attacker not found"))
}

// ── mutations: city production ───────────────────────────────────────────────

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct QueueProductionBody {
    pub item_id: String,
    pub item_type: String, // "unit" | "building" | "wonder" | "district" | "project"
}

/// `POST /api/v1/cities/{id}/production` — append `{item_id, item_type}` to the
/// city's production queue.
pub async fn queue_production(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<QueueProductionBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let city_ulid: ulid::Ulid = id
        .parse()
        .map_err(|_| crate::server::rest::auth::bad_request("invalid_id", "invalid city id"))?;
    let city_id = open4x_protocol::v1::ids::CityId::from_ulid(city_ulid);

    let item_ulid: ulid::Ulid = body
        .item_id
        .parse()
        .map_err(|_| crate::server::rest::auth::bad_request("invalid_id", "invalid item id"))?;
    let item = match body.item_type.as_str() {
        "unit" => open4x_protocol::v1::enums::ProductionItemView::Unit(
            open4x_protocol::v1::ids::UnitTypeId::from_ulid(item_ulid),
        ),
        "building" => open4x_protocol::v1::enums::ProductionItemView::Building(
            open4x_protocol::v1::ids::BuildingId::from_ulid(item_ulid),
        ),
        "wonder" => open4x_protocol::v1::enums::ProductionItemView::Wonder(
            open4x_protocol::v1::ids::WonderId::from_ulid(item_ulid),
        ),
        "project" => open4x_protocol::v1::enums::ProductionItemView::Project(
            open4x_protocol::v1::ids::ProjectId::from_ulid(item_ulid),
        ),
        // District is a plain enum (not ULID); unsupported here.
        other => {
            return Err(crate::server::rest::auth::bad_request(
                "invalid_item_type",
                &format!("item_type {other:?} not supported via REST yet"),
            ));
        }
    };

    let action = open4x_protocol::v1::messages::GameAction::QueueProduction {
        city: city_id,
        item,
    };

    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;

    let view = view_after_mutation_city(&state, game_id, civ_id, &id)?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view,
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

/// `DELETE /api/v1/cities/{id}/production/{pos}` — remove queue entry at index.
pub async fn cancel_production(
    State(state): State<Arc<AppState>>,
    Path((id, pos)): Path<(String, usize)>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let city_ulid: ulid::Ulid = id
        .parse()
        .map_err(|_| crate::server::rest::auth::bad_request("invalid_id", "invalid city id"))?;
    let city_id = open4x_protocol::v1::ids::CityId::from_ulid(city_ulid);

    let action = open4x_protocol::v1::messages::GameAction::CancelProduction {
        city: city_id,
        index: pos,
    };
    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;

    let view = view_after_mutation_city(&state, game_id, civ_id, &id)?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view,
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

// ── mutations: city focus ────────────────────────────────────────────────────

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AssignCityFocusBody {
    /// Lowercase variant name: "default" | "food" | "production" | "gold"
    /// | "science" | "culture" | "faith".
    pub focus: String,
}

/// `POST /api/v1/cities/{id}/focus` — set the city's player-selected
/// production focus (mirrors the wireframe's per-city focus dropdown).
/// Returns the updated city wire row.
pub async fn assign_city_focus(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<AssignCityFocusBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let city_ulid: ulid::Ulid = id
        .parse()
        .map_err(|_| crate::server::rest::auth::bad_request("invalid_id", "invalid city id"))?;
    let city_id = open4x_protocol::v1::ids::CityId::from_ulid(city_ulid);

    let focus = match body.focus.to_lowercase().as_str() {
        "default" => open4x_protocol::v1::enums::CityFocus::Default,
        "food" => open4x_protocol::v1::enums::CityFocus::Food,
        "production" => open4x_protocol::v1::enums::CityFocus::Production,
        "gold" => open4x_protocol::v1::enums::CityFocus::Gold,
        "science" => open4x_protocol::v1::enums::CityFocus::Science,
        "culture" => open4x_protocol::v1::enums::CityFocus::Culture,
        "faith" => open4x_protocol::v1::enums::CityFocus::Faith,
        other => {
            return Err(crate::server::rest::auth::bad_request(
                "invalid_focus",
                &format!("unknown focus: {other:?}"),
            ));
        }
    };

    let action = open4x_protocol::v1::messages::GameAction::AssignCityFocus {
        city: city_id,
        focus,
    };
    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;

    let view = view_after_mutation_city(&state, game_id, civ_id, &id)?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view,
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

// ── mutations: city rename ───────────────────────────────────────────────────

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RenameCityBody {
    /// New city name. Server enforces 1..=64 chars after trim.
    pub name: String,
}

/// `POST /api/v1/cities/{id}/rename` — replace `City.name`. Returns the
/// updated city wire row.
pub async fn rename_city(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<RenameCityBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let city_ulid: ulid::Ulid = id
        .parse()
        .map_err(|_| crate::server::rest::auth::bad_request("invalid_id", "invalid city id"))?;
    let city_id = open4x_protocol::v1::ids::CityId::from_ulid(city_ulid);

    // Validate at the boundary — match the rule the engine enforces, but
    // surface a structured 400 rather than a generic 'apply_action failed'.
    let trimmed = body.name.trim().to_string();
    if trimmed.is_empty() {
        return Err(crate::server::rest::auth::bad_request(
            "invalid_name",
            "city name must not be empty",
        ));
    }
    if trimmed.chars().count() > 64 {
        return Err(crate::server::rest::auth::bad_request(
            "invalid_name",
            "city name must be 64 characters or fewer",
        ));
    }

    let action = open4x_protocol::v1::messages::GameAction::RenameCity {
        city: city_id,
        name: trimmed,
    };
    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;

    let view = view_after_mutation_city(&state, game_id, civ_id, &id)?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view,
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

// ── mutations: unit actions ──────────────────────────────────────────────────

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UnitActionBody {
    pub action_id: String, // "move" | "attack" | "fortify" | "sleep" | "found_city"
    #[serde(default)]
    pub target_q: Option<i32>,
    #[serde(default)]
    pub target_r: Option<i32>,
    /// City name when `action_id == "found_city"`. Defaults to "New City".
    #[serde(default)]
    pub name: Option<String>,
}

/// `POST /api/v1/units/{id}/action` — dispatch a unit action through libciv.
pub async fn unit_action(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<UnitActionBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let unit_ulid: ulid::Ulid = id
        .parse()
        .map_err(|_| crate::server::rest::auth::bad_request("invalid_id", "invalid unit id"))?;
    let unit_id = open4x_protocol::v1::ids::UnitId::from_ulid(unit_ulid);

    let target = match (body.target_q, body.target_r) {
        (Some(q), Some(r)) => Some(open4x_protocol::v1::coord::HexCoord { q, r, s: -q - r }),
        _ => None,
    };

    let action = match body.action_id.as_str() {
        "move" => {
            let to = target.ok_or_else(|| {
                crate::server::rest::auth::bad_request(
                    "missing_target",
                    "move requires target_q/target_r",
                )
            })?;
            open4x_protocol::v1::messages::GameAction::MoveUnit { unit: unit_id, to }
        }
        "attack" => {
            // Resolve defender by coord lookup against the player's view.
            let to = target.ok_or_else(|| {
                crate::server::rest::auth::bad_request(
                    "missing_target",
                    "attack requires target_q/target_r",
                )
            })?;
            let view = view_only(&state, game_id, civ_id)?;
            let defender = view
                .units
                .iter()
                .find(|u| u.coord == to && !u.is_own)
                .ok_or_else(|| crate::server::rest::auth::not_found("no enemy unit at target"))?;
            open4x_protocol::v1::messages::GameAction::Attack {
                attacker: unit_id,
                defender: defender.id,
            }
        }
        "found_city" => open4x_protocol::v1::messages::GameAction::FoundCity {
            settler: unit_id,
            name: body.name.clone().unwrap_or_else(|| "New City".into()),
        },
        "fortify" | "sleep" => {
            // No matching GameAction variant yet; treat as a UI no-op so the
            // wireframe doesn't fail. Will plumb through libciv in Phase 4.
            let new_turn = state.games.get(&game_id).map(|r| r.state.turn).unwrap_or(0);
            return Ok((
                StatusCode::ACCEPTED,
                Json(MutationResponse {
                    ok: true,
                    view: serde_json::Value::Null,
                    turn_status: TurnStatusBlock {
                        turn: new_turn,
                        ended: false,
                    },
                }),
            ));
        }
        other => {
            return Err(crate::server::rest::auth::bad_request(
                "unknown_action",
                &format!("unit action {other:?} not supported"),
            ));
        }
    };

    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;

    let view = view_only(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    let unit = web_projection::build_units_from_room(&view, &room)
        .units
        .into_iter()
        .find(|u| u.id == id);

    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view: serde_json::to_value(unit).unwrap_or(serde_json::Value::Null),
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

// ── mutation helpers ─────────────────────────────────────────────────────────

fn mutate_room<F>(state: &Arc<AppState>, game_id: GameId, f: F) -> Result<u32, ApiError>
where
    F: FnOnce(&mut GameRoom) -> Result<(), String>,
{
    let mut room = state
        .games
        .get_mut(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    f(&mut room).map_err(|e| crate::server::rest::auth::bad_request("rule_violation", &e))?;
    Ok(room.state.turn)
}

fn view_only(state: &Arc<AppState>, game_id: GameId, civ_id: CivId) -> Result<GameView, ApiError> {
    let (view, _) = view_and_turn_limit(state, game_id, civ_id)?;
    Ok(view)
}

fn view_after_mutation_city(
    state: &Arc<AppState>,
    game_id: GameId,
    civ_id: CivId,
    city_id: &str,
) -> Result<open4x_protocol::v1::web::city_data::CityRow, ApiError> {
    let view = view_only(state, game_id, civ_id)?;
    web_projection::build_cities(&view)
        .cities
        .into_iter()
        .find(|c| c.id == city_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("city not found"))
}

// ── /tech ────────────────────────────────────────────────────────────────────

pub async fn tech(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_tech_tree(&view)))
}

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TechResearchBody {
    pub tech_id: String,
}

pub async fn tech_research(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<TechResearchBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let tech_ulid: ulid::Ulid = body
        .tech_id
        .parse()
        .map_err(|_| crate::server::rest::auth::bad_request("invalid_id", "invalid tech id"))?;
    let tech = open4x_protocol::v1::ids::TechId::from_ulid(tech_ulid);
    let action = open4x_protocol::v1::messages::GameAction::QueueResearch { tech };

    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;
    let view = view_only(&state, game_id, civ_id)?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view: web_projection::build_tech_tree(&view),
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

/// `DELETE /api/v1/tech/research` — drop the active (front) entry from the
/// civ's research queue. Idempotent. Returns the updated tech tree.
pub async fn cancel_research(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let action = open4x_protocol::v1::messages::GameAction::CancelResearch;
    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;
    let view = view_only(&state, game_id, civ_id)?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view: web_projection::build_tech_tree(&view),
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

// ── /civics ──────────────────────────────────────────────────────────────────

pub async fn civics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_civics_tree(&view)))
}

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CivicResearchBody {
    pub civic_id: String,
}

pub async fn civic_research(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CivicResearchBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let civic_ulid: ulid::Ulid = body
        .civic_id
        .parse()
        .map_err(|_| crate::server::rest::auth::bad_request("invalid_id", "invalid civic id"))?;
    let civic = open4x_protocol::v1::ids::CivicId::from_ulid(civic_ulid);
    let action = open4x_protocol::v1::messages::GameAction::QueueCivic { civic };

    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;
    let view = view_only(&state, game_id, civ_id)?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view: web_projection::build_civics_tree(&view),
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

/// `DELETE /api/v1/civics/research` — clear the civ's `civic_in_progress`
/// slot. Idempotent. Returns the updated civics tree.
pub async fn cancel_civic(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let action = open4x_protocol::v1::messages::GameAction::CancelCivic;
    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;
    let view = view_only(&state, game_id, civ_id)?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view: web_projection::build_civics_tree(&view),
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

// ── /government ──────────────────────────────────────────────────────────────

pub async fn government(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    Ok(Json(web_projection::build_government_from_room(
        &view, &room, civ_id,
    )))
}

#[derive(Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChangeGovernmentBody {
    /// Government name as it appears in the wire registry (e.g. "Chiefdom",
    /// "Monarchy", "Democracy"). Case-sensitive — matches `Government.name`.
    pub government: String,
}

/// `POST /api/v1/government/change` — switch the civ's active government.
/// Rejects unknown or not-yet-unlocked governments with a structured 400.
/// On success, returns the updated `/government` block.
pub async fn change_government(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ChangeGovernmentBody>,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let libciv_civ = libciv::CivId::from_ulid(civ_id.as_ulid());

    let trimmed = body.government.trim();
    if trimmed.is_empty() {
        return Err(crate::server::rest::auth::bad_request(
            "invalid_government",
            "government name must not be empty",
        ));
    }

    let action = open4x_protocol::v1::messages::GameAction::ChangeGovernment {
        name: trimmed.to_string(),
    };
    let new_turn = mutate_room(&state, game_id, |room| {
        room.apply_action(libciv_civ, &action)
    })?;

    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    Ok((
        StatusCode::OK,
        Json(MutationResponse {
            ok: true,
            view: web_projection::build_government_from_room(&view, &room, civ_id),
            turn_status: TurnStatusBlock {
                turn: new_turn,
                ended: false,
            },
        }),
    ))
}

// ── /map/overlays ────────────────────────────────────────────────────────────

pub async fn map_overlays(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_map_overlays(&view)))
}

// ── /registry ────────────────────────────────────────────────────────────────

pub async fn registry(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_registry(&view)))
}

// ── /diplomacy ───────────────────────────────────────────────────────────────

pub async fn diplomacy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_diplomacy(&view)))
}

pub async fn diplomacy_civ(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let dip = web_projection::build_diplomacy(&view);
    dip.civs
        .into_iter()
        .find(|c| c.id == id)
        .map(Json)
        .ok_or_else(|| crate::server::rest::auth::not_found("civ not found"))
}

// ── /empire/overview ─────────────────────────────────────────────────────────

pub async fn empire_overview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    Ok(Json(web_projection::build_empire_overview(&view)))
}

// ── /victory ─────────────────────────────────────────────────────────────────

pub async fn victory(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, turn_limit) = view_and_turn_limit(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    Ok(Json(web_projection::build_victory_from_room(
        &view, &room, civ_id, turn_limit,
    )))
}

// ── /notifications + /turn-queue ─────────────────────────────────────────────

pub async fn notifications(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    Ok(Json(web_projection::build_notifications_from_room(
        &view, &room, civ_id,
    )))
}

pub async fn dismiss_notification(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let mut room = state
        .games
        .get_mut(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    room.notifications.dismiss(civ_id, &id);
    Ok((StatusCode::NO_CONTENT, ""))
}

pub async fn dismiss_all_notifications(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let mut room = state
        .games
        .get_mut(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    room.notifications.dismiss_all(civ_id);
    Ok((StatusCode::NO_CONTENT, ""))
}

pub async fn turn_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let (view, _) = view_and_turn_limit(&state, game_id, civ_id)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    Ok(Json(web_projection::build_turn_queue_from_room(
        &view, &room, civ_id,
    )))
}
