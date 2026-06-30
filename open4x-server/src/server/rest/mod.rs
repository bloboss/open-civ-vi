//! REST handlers for the `/api/v1/*` surface defined in
//! `book/src/roadmap/web-ui.md` §4.
//!
//! Single source of truth for the route table is [`v1_router`] — `main.rs`
//! mounts it under `/api/v1`, integration tests call it directly via
//! `tower::ServiceExt::oneshot` without binding a TCP socket.

pub mod auth;
pub mod climate;
pub mod governors;
pub mod handlers;

pub use handlers::*;

use std::sync::Arc;

use axum::Router;
use axum::routing::{delete, get, post};

use crate::server::state::AppState;

/// Build the `Router` carrying every `/api/v1/*` route. The returned router
/// is **not** prefixed; callers should `.nest("/api/v1", ...)`.
pub fn v1_router() -> Router<Arc<AppState>> {
    Router::new()
        // reads
        .route("/health", get(handlers::health))
        .route("/games/new", post(handlers::new_game))
        .route("/player-state", get(handlers::player_state))
        .route("/world/snapshot", get(handlers::world_snapshot))
        .route("/world/tile/{q}/{r}", get(handlers::world_tile))
        .route("/map/overlays", get(handlers::map_overlays))
        .route("/cities", get(handlers::cities))
        .route("/cities/{id}", get(handlers::city_detail))
        .route("/cities/{id}/tiles", get(handlers::city_tiles))
        .route("/units", get(handlers::units))
        .route("/units/{id}", get(handlers::unit_detail))
        .route("/armies", get(handlers::armies))
        .route("/combat/preview", get(handlers::combat_preview))
        .route("/tech", get(handlers::tech))
        .route("/civics", get(handlers::civics))
        .route("/government", get(handlers::government))
        .route("/government/change", post(handlers::change_government))
        .route("/governors", get(governors::governors))
        .route("/diplomacy", get(handlers::diplomacy))
        .route("/diplomacy/civs/{id}", get(handlers::diplomacy_civ))
        .route("/empire/overview", get(handlers::empire_overview))
        .route("/victory", get(handlers::victory))
        .route("/registry", get(handlers::registry))
        .route("/notifications", get(handlers::notifications).delete(handlers::dismiss_all_notifications))
        .route("/notifications/{id}", delete(handlers::dismiss_notification))
        .route("/turn-queue", get(handlers::turn_queue))
        .route("/climate", get(climate::climate))
        // writes
        .route("/cities/{id}/production", post(handlers::queue_production))
        .route("/cities/{id}/production/{pos}", delete(handlers::cancel_production))
        .route("/cities/{id}/focus", post(handlers::assign_city_focus))
        .route("/cities/{id}/rename", post(handlers::rename_city))
        .route("/units/{id}/action", post(handlers::unit_action))
        .route("/tech/research", post(handlers::tech_research).delete(handlers::cancel_research))
        .route("/civics/research", post(handlers::civic_research).delete(handlers::cancel_civic))
        .route("/turn/end", post(handlers::end_turn))
}
