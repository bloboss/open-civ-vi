#![allow(dead_code)]

use axum::Router;
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use open4x_server::server;

#[tokio::main]
async fn main() {
    let state = server::state::AppState::new();

    // Static file directory for the trunk-built frontend.
    let static_dir = std::env::var("OPEN4X_STATIC_DIR")
        .unwrap_or_else(|_| "./open4x-client-web/dist".to_string());

    let app = Router::new()
        .route("/ws", get(server::ws::ws_handler))
        .route("/health", get(|| async { "ok" }))
        .route("/api/demo-game", get(demo_game_handler))
        // REST v1 — see book/src/roadmap/web-ui.md
        .nest("/api/v1", server::rest::v1_router())
        .fallback_service(ServeDir::new(&static_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("0.0.0.0:{port}");
    println!("open4x-server listening on {addr}");
    println!("  static files: {static_dir}");
    println!(
        "  data dir:     {}",
        std::env::var("OPEN4X_DATA_DIR").unwrap_or_else(|_| "./data".into())
    );
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}

/// GET /api/demo-game?seed=42&width=20&height=14&turns=100
async fn demo_game_handler(
    params: axum::extract::Query<DemoParams>,
) -> axum::Json<server::demo::DemoGameResult> {
    let result = server::demo::run_demo_game(
        params.seed.unwrap_or(42),
        params.width.unwrap_or(20),
        params.height.unwrap_or(14),
        params.turns.unwrap_or(100),
        params.players.unwrap_or(2),
    );
    axum::Json(result)
}

#[derive(serde::Deserialize)]
struct DemoParams {
    seed: Option<u64>,
    width: Option<u32>,
    height: Option<u32>,
    turns: Option<u32>,
    players: Option<u32>,
}
