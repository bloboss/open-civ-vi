//! REST surface — `/api/v1/*`. Nested into the top-level router by
//! `main.rs` after the session-cookie middleware runs.

#![cfg(feature = "ssr")]

pub mod email_auth;
pub mod friends;
pub mod games;
pub mod me;
pub mod presets;
pub mod pubkey_auth;

use axum::Router;
use axum::routing::{delete, get, patch, post};

use super::AppState;

pub fn v1_router() -> Router<AppState> {
    Router::new()
        .route("/auth/email/start", post(email_auth::start))
        .route("/auth/email/verify", get(email_auth::verify))
        .route("/auth/pubkey/challenge", post(pubkey_auth::challenge))
        .route("/auth/pubkey/verify", post(pubkey_auth::verify))
        .route("/auth/signout", post(email_auth::signout))
        .route("/me", get(me::get_me))
        .route("/me", patch(me::patch_me))
        .route("/me", delete(me::delete_me))
        .route("/me/identities/{id}", delete(me::unlink_identity))
        .route(
            "/me/identities/{id}/verify-start",
            post(me::verify_email_identity),
        )
        .route("/me/avatar", post(me::upload_avatar))
        .route("/games", get(games::list))
        .route("/games", post(games::create))
        .route("/games/{id}", get(games::get_one))
        .route("/games/{id}", delete(games::delete_one))
        .route("/games/{id}/notes", post(games::set_notes))
        .route("/games/{id}/resume", post(games::resume))
        .route("/games/{id}/thumbnail", get(games::thumbnail))
        .route("/friends", get(friends::list))
        .route("/friends/request", post(friends::request))
        .route("/friends/search", post(friends::search))
        .route("/friends/{id}/accept", post(friends::accept))
        .route("/friends/{id}", delete(friends::unfriend))
        .route("/presets", get(presets::list))
        .route("/presets", post(presets::create))
        .route("/presets/{id}", delete(presets::delete_one))
}
