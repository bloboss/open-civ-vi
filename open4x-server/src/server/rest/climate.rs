//! `GET /api/v1/climate` — read-only projection of the global climate /
//! sea-level simulation (libciv GS-2).
//!
//! Mirrors the `government` read pattern: `auth_or_401` → fetch the
//! `GameRoom` from `state.games` → project from `room.state` (libciv
//! `GameState`). The climate data lives on the authoritative state, not on
//! the per-player `GameView`, so we re-fetch the room rather than reuse the
//! projected view.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;

use libhexgrid::board::HexBoard;

use crate::server::rest::auth::{ApiError, auth_or_401, not_found};
use crate::server::state::{AppState, GameRoom};
use open4x_protocol::v1::ids::CivId;
use open4x_protocol::v1::web::climate::ClimateView;

/// `GET /api/v1/climate` — global CO2 / sea-level state plus the
/// authenticated player's per-turn emissions and the count of submerged
/// tiles. `recent_disasters` is best-effort and currently always empty: the
/// libciv `GameState` keeps no persistent disaster log.
pub async fn climate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| not_found("game not found"))?;
    Ok(Json(build_climate(&room, civ_id)))
}

/// Project a [`ClimateView`] from the authoritative `room.state`.
pub fn build_climate(room: &GameRoom, civ: CivId) -> ClimateView {
    let state = &room.state;
    let libciv_civ = libciv::CivId::from_ulid(civ.as_ulid());

    // CO2 emitted per turn by the player's owned cities. Only fossil-fuel
    // power plants carry a non-zero `co2_per_turn`; build a lookup once and
    // sum across every building in every owned city.
    let co2_by_building: HashMap<libciv::BuildingId, u32> = state
        .building_defs
        .iter()
        .map(|d| (d.id, d.co2_per_turn))
        .collect();
    let co2_per_turn: u32 = state
        .cities
        .iter()
        .filter(|c| c.owner == libciv_civ)
        .flat_map(|c| c.buildings.iter())
        .map(|bid| co2_by_building.get(bid).copied().unwrap_or(0))
        .sum();

    // Count coastal-lowland tiles that have been submerged by sea-level rise.
    let submerged_tiles: u32 = state
        .board
        .all_coords()
        .into_iter()
        .filter(|&coord| {
            state
                .board
                .tile(coord)
                .map(|t| t.submerged)
                .unwrap_or(false)
        })
        .count() as u32;

    ClimateView {
        global_co2: state.global_co2,
        climate_level: state.climate_level,
        thresholds: libciv::world::CLIMATE_THRESHOLDS.to_vec(),
        co2_per_turn,
        submerged_tiles,
        // No persistent disaster log on `GameState` — best-effort empty.
        recent_disasters: Vec::new(),
    }
}
