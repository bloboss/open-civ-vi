//! `GET /api/v1/governors` — read-only projection of the requesting civ's
//! governor roster.
//!
//! The governor simulation lives entirely in libciv (`GameState.governors`
//! and the per-civ `Civilization.governor_titles`); this endpoint only
//! reshapes it for the web UI. Mirrors the `/government` handler pattern:
//! `auth_or_401` → fetch the `GameRoom` → project from `room.state`.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;

use crate::server::rest::auth::{ApiError, auth_or_401};
use crate::server::state::{AppState, GameRoom};
use open4x_protocol::v1::ids::CivId;
use open4x_protocol::v1::web::governors as wire;

/// `GET /api/v1/governors` — list the authenticated civ's governors plus the
/// number of unspent governor titles.
pub async fn governors(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| crate::server::rest::auth::not_found("game not found"))?;
    Ok(Json(build_governors_from_room(&room, civ_id)))
}

/// Project the governor roster owned by `civ` out of `room.state`.
pub fn build_governors_from_room(
    room: &GameRoom,
    civ: CivId,
) -> wire::GovernorsView {
    let libciv_civ = libciv::CivId::from_ulid(civ.as_ulid());

    let titles_available = room
        .state
        .civilizations
        .iter()
        .find(|c| c.id == libciv_civ)
        .map(|c| c.governor_titles)
        .unwrap_or(0);

    let entries = room
        .state
        .governors
        .iter()
        .filter(|g| g.owner == libciv_civ)
        .map(|g| wire::GovernorEntry {
            id: g.id.as_ulid().to_string(),
            name: g.def_name.to_string(),
            assigned_city: g
                .assigned_city
                .and_then(|cid| room.state.city(cid))
                .map(|city| city.name.clone()),
            established: g.is_established(),
            turns_to_establish: g.turns_to_establish,
            promotions: g.promotions.iter().map(|p| p.to_string()).collect(),
        })
        .collect();

    wire::GovernorsView {
        titles_available,
        governors: entries,
    }
}
