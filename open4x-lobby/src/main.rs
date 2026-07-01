//! Native binary for the open4x-lobby — Axum + ServeDir + auth surface.
//!
//! Boots an `AppState` (sqlite db + magic-link signer + LogMailer +
//! account store), wires the session-cookie middleware on every
//! request, serves the Trunk-built SPA + /health for liveness checks.
//! HTTP routes for email auth + /me come online incrementally as
//! Phase 3 progresses.

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use open4x_lobby::server::{self, AppState};

#[tokio::main]
async fn main() {
    // Empty-string env values fall back to the next option so an
    // operator can deliberately disable the on-disk path.
    let static_dir = std::env::var("OPEN4X_LOBBY_STATIC_DIR")
        .ok()
        .filter(|s| !s.is_empty());
    let data_dir =
        std::env::var("OPEN4X_LOBBY_DATA_DIR").unwrap_or_else(|_| "./data/lobby".to_string());
    let book_dir = std::env::var("OPEN4X_LOBBY_BOOK_DIR")
        .ok()
        .filter(|s| !s.is_empty());

    let state = AppState::boot(&data_dir)
        .await
        .expect("AppState::boot failed — check db / key file permissions");

    let mut app = Router::new()
        .route("/health", get(health_handler))
        .nest("/api/v1", server::rest::v1_router())
        // Per-player PNGs land here as
        // `<data_dir>/avatars/<player_id_hex>.png`. ServeDir 404s
        // when missing, which is fine — the SPA falls back to the
        // initial-letter circle.
        .nest_service("/avatars", ServeDir::new(state.avatar_dir.clone()));

    // Book: prefer the on-disk path if set, then fall back to
    // embedded assets, then 404 with a hint.
    app = match book_dir.as_deref() {
        Some(path) => app.nest_service("/book", ServeDir::new(path)),
        None if server::embed::book_assets_present() => {
            app.route("/book/{*path}", get(server::embed::book_handler))
        }
        None => app,
    };

    app = app.layer(axum::middleware::from_fn_with_state(
        state.clone(),
        server::auth::session_layer,
    ));

    // SPA: same priority order. ServeDir handles index-fallback via
    // .fallback_service; the embedded handler does its own.
    app = match static_dir.as_deref() {
        Some(path) => app.fallback_service(ServeDir::new(path)),
        None if server::embed::spa_assets_present() => app.fallback(server::embed::spa_fallback),
        None => app, // No SPA — non-SPA deploys (e.g. headless API) still work.
    };

    let app = app.layer(CorsLayer::permissive()).with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3002".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("open4x-lobby listening on {addr}");
    println!(
        "  static files: {}",
        match static_dir.as_deref() {
            Some(p) => format!("{p} (on-disk)"),
            None if server::embed::spa_assets_present() => "embedded".into(),
            None => "(none — API only)".into(),
        }
    );
    println!("  data dir:     {data_dir}");
    println!(
        "  book dir:     {}",
        match book_dir.as_deref() {
            Some(p) => format!("{p} (on-disk)"),
            None if server::embed::book_assets_present() => "embedded".into(),
            None => "(empty — run `mdbook build book/` and rebuild)".into(),
        }
    );
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server error");
    println!("open4x-lobby exited cleanly");
}

/// `GET /health` — returns 200 `ok` when the sqlite pool answers a
/// trivial SELECT. Returns 503 `db_unreachable` otherwise so a load
/// balancer / supervisor can pull the instance out of rotation.
async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => (StatusCode::OK, "ok").into_response(),
        Err(e) => {
            eprintln!("[health] db ping failed: {e}");
            (StatusCode::SERVICE_UNAVAILABLE, "db_unreachable").into_response()
        }
    }
}

/// Wait for SIGINT (Ctrl-C) or SIGTERM. Triggers
/// axum::serve(...).with_graceful_shutdown to drain in-flight
/// requests before closing the listening socket.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => println!("\n[shutdown] received SIGINT, draining…"),
        _ = terminate => println!("\n[shutdown] received SIGTERM, draining…"),
    }
}
