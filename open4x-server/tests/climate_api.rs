//! Integration test for the `GET /api/v1/climate` read projection.
//!
//! Follows the same `tower::ServiceExt::oneshot` rig as `rest_api.rs`: build
//! a fresh `AppState` + v1 `Router`, mint a bearer token via
//! `POST /api/v1/games/new`, then assert the climate block's shape on a
//! freshly bootstrapped game (zero emissions, no submerged tiles).

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as _;
use serde_json::Value;
use tower::ServiceExt;

use open4x_server::server::rest::v1_router;
use open4x_server::server::state::AppState;

fn build_app() -> (Router, Arc<AppState>) {
    let state = AppState::new();
    let router = Router::new()
        .nest("/api/v1", v1_router())
        .with_state(state.clone());
    (router, state)
}

async fn json_body(resp: axum::response::Response) -> (StatusCode, Value) {
    let status = resp.status();
    let collected = resp.into_body().collect().await.expect("collect body");
    let bytes = collected.to_bytes();
    let value: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("response was not json")
    };
    (status, value)
}

async fn bootstrap_token(app: &Router) -> String {
    let body = serde_json::json!({
        "width": 12,
        "height": 8,
        "seed": 7,
        "num_ai": 0,
        "turn_limit": 50,
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/games/new")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = json_body(resp).await;
    assert_eq!(status, StatusCode::CREATED, "games/new: {body:?}");
    body["token"].as_str().expect("token in response").to_string()
}

async fn get_with(app: &Router, path: &str, token: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    json_body(resp).await
}

#[tokio::test]
async fn climate_projects_fresh_game_baseline() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, c) = get_with(&app, "/api/v1/climate", &token).await;
    assert_eq!(status, StatusCode::OK, "climate: {c:?}");

    // A fresh game has emitted no CO2 and risen no sea levels.
    assert_eq!(c["global_co2"], 0);
    assert_eq!(c["climate_level"], 0);
    assert_eq!(c["co2_per_turn"], 0);
    assert_eq!(c["submerged_tiles"], 0);

    // Thresholds mirror libciv's CLIMATE_THRESHOLDS exactly.
    let thresholds = c["thresholds"].as_array().expect("thresholds array");
    assert_eq!(
        thresholds.iter().map(|v| v.as_u64().unwrap()).collect::<Vec<_>>(),
        vec![200, 400, 600, 800, 1000, 1200, 1500],
    );

    // No persistent disaster log → best-effort empty list, but always present.
    assert!(c["recent_disasters"].is_array());
    assert_eq!(c["recent_disasters"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn climate_requires_auth() {
    let (app, _state) = build_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/climate")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, _) = json_body(resp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
