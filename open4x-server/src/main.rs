#![allow(dead_code)]

mod auth;
mod game_room;
mod persist;
mod projection;
mod session;
mod state;
mod templates;
mod ws;

use axum::Router;
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use state::AppState;

#[tokio::main]
async fn main() {
    let state = AppState::new();

    // Static file directory for the trunk-built frontend.
    // In Docker this is /app/static; locally fall back to open4x-web/dist.
    let static_dir = std::env::var("OPEN4X_STATIC_DIR")
        .unwrap_or_else(|_| "./open4x-web/dist".to_string());

    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .route("/health", get(|| async { "ok" }))
        .fallback_service(ServeDir::new(&static_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("open4x-server listening on {addr}");
    println!("  static files: {static_dir}");
    println!("  data dir:     {}", std::env::var("OPEN4X_DATA_DIR").unwrap_or_else(|_| "./data".into()));
    let listener = tokio::net::TcpListener::bind(&addr).await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
