//! Read-only `/great-people` projection.
//!
//! Mirrors the `government` read-route pattern: authenticate, fetch the
//! `GameRoom`, then project directly from `room.state` (libciv `GameState`)
//! because per-civ great-person points and the great-person roster live on the
//! simulation state rather than the projected `GameView`.

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;

use crate::server::rest::auth::{auth_or_401, not_found, ApiError};
use crate::server::state::{AppState, GameRoom};
use open4x_protocol::v1::ids::CivId;
use open4x_protocol::v1::web::great_people as gp_view;

/// All great-person classes, in a stable display order.
const GREAT_PERSON_CLASSES: [libciv::GreatPersonType; 9] = [
    libciv::GreatPersonType::General,
    libciv::GreatPersonType::Admiral,
    libciv::GreatPersonType::Engineer,
    libciv::GreatPersonType::Merchant,
    libciv::GreatPersonType::Musician,
    libciv::GreatPersonType::Artist,
    libciv::GreatPersonType::Writer,
    libciv::GreatPersonType::Prophet,
    libciv::GreatPersonType::Scientist,
];

/// `GET /great-people` — per-class point progress plus the visible roster for
/// the authenticated civilization.
pub async fn great_people(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let (game_id, civ_id) = auth_or_401(&state, &headers)?;
    let room = state
        .games
        .get(&game_id)
        .ok_or_else(|| not_found("game not found"))?;
    Ok(Json(build_great_people_from_room(&room, civ_id)))
}

/// Project the great-people view for `civ` from a game room's simulation state.
pub fn build_great_people_from_room(
    room: &GameRoom,
    civ: CivId,
) -> gp_view::GreatPeopleView {
    let libciv_civ = libciv::CivId::from_ulid(civ.as_ulid());

    // Per-class progress toward the next recruitment threshold.
    let points = GREAT_PERSON_CLASSES
        .iter()
        .map(|&class| {
            let pts = room
                .state
                .civ(libciv_civ)
                .and_then(|c| c.great_person_points.get(&class).copied())
                .unwrap_or(0);
            let threshold = libciv::civ::recruitment_threshold(class, &room.state);
            let progress = if threshold > 0 {
                pts as f32 / threshold as f32
            } else {
                0.0
            };
            gp_view::GreatPersonProgress {
                class: format!("{class:?}"),
                points: pts,
                threshold,
                progress,
            }
        })
        .collect();

    // Roster: this civ's recruited great people plus those still available.
    let roster = room
        .state
        .great_people
        .iter()
        .filter_map(|gp| {
            let owned = gp.owner == Some(libciv_civ);
            let available = gp.is_available();
            if !owned && !available {
                return None;
            }
            Some(gp_view::GreatPersonEntry {
                id: gp.id.as_ulid().to_string(),
                name: gp.name.to_string(),
                class: format!("{:?}", gp.person_type),
                era: gp.era.to_string(),
                owned,
                available,
                retired: gp.is_retired,
                ability: gp.ability_names.join(", "),
            })
        })
        .collect();

    gp_view::GreatPeopleView { points, roster }
}
