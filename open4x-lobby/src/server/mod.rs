//! Server-only Axum surface for the lobby. ssr feature only.
//!
//! Phase 3 of `book/src/roadmap/accounts-and-login.md`. Owns the
//! shared `AppState`, session-cookie middleware, magic-link auth
//! route pair, and `/api/v1/me` profile reads/writes.

pub mod auth;
pub mod client_ip;
pub mod embed;
pub mod orchestrator;
pub mod process;
pub mod pubkey_challenge;
pub mod rest;
pub mod state;

pub use auth::{AuthCookie, RequireSession, SESSION_COOKIE_NAME, session_layer};
pub use state::AppState;

/// Assemble the `/api/v1` surface with the session-cookie middleware and
/// state applied — the core API app. `main.rs` layers deploy-specific
/// extras (health, avatars, book, SPA fallback) on top; integration tests
/// drive this directly via `tower::ServiceExt::oneshot`.
pub fn api_app(state: AppState) -> axum::Router {
    axum::Router::new()
        .nest("/api/v1", rest::v1_router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::session_layer,
        ))
        .with_state(state)
}
