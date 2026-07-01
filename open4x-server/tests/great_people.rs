//! Integration test for the read-only `/api/v1/great-people` route.
//!
//! Exercises the same `Router` `main.rs` mounts, skipping the TCP listener via
//! `tower::ServiceExt::oneshot` (see `rest_api.rs` for the shared shape).

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as _;
use serde_json::Value;
use tower::ServiceExt;

use open4x_server::server::rest::v1_router;
use open4x_server::server::state::AppState;

fn build_app() -> Router {
    let state = AppState::new();
    Router::new().nest("/api/v1", v1_router()).with_state(state)
}

async fn json_body(resp: axum::response::Response) -> (StatusCode, Value) {
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let value: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("response was not json")
    };
    (status, value)
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
    body["token"]
        .as_str()
        .expect("token in response")
        .to_string()
}

#[tokio::test]
async fn great_people_requires_auth() {
    let app = build_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/great-people")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn great_people_returns_per_class_progress() {
    let app = build_app();
    let token = bootstrap_token(&app).await;

    let (status, gp) = get_with(&app, "/api/v1/great-people", &token).await;
    assert_eq!(status, StatusCode::OK, "great-people: {gp:?}");

    // The view always reports progress for every great-person class.
    let points = gp["points"].as_array().expect("points array");
    assert_eq!(points.len(), 9, "one progress entry per great-person class");

    // Each entry exposes the wire schema and a base recruitment threshold.
    for entry in points {
        assert!(entry["class"].is_string(), "class is a string: {entry:?}");
        assert!(entry["points"].is_u64(), "points is numeric: {entry:?}");
        assert_eq!(
            entry["threshold"].as_u64(),
            Some(60),
            "base recruitment threshold should be 60 for a fresh game: {entry:?}"
        );
        assert!(
            entry["progress"].is_number(),
            "progress is numeric: {entry:?}"
        );
    }

    // A fresh game has no recruited or pooled great people yet.
    assert!(gp["roster"].as_array().expect("roster array").is_empty());
}
