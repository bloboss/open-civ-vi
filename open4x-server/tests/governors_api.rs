//! Integration test for the read-only `GET /api/v1/governors` projection.
//!
//! Mirrors the rig in `rest_api.rs`: build a fresh `AppState`, mint a bearer
//! token via `/api/v1/games/new`, then seed a governor directly onto the
//! room's libciv `GameState` and assert the wire shape reflects it.

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

async fn post_json(app: &Router, path: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
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
    let (status, body) = post_json(app, "/api/v1/games/new", body).await;
    assert_eq!(status, StatusCode::CREATED, "games/new: {body:?}");
    body["token"].as_str().expect("token in response").to_string()
}

#[tokio::test]
async fn governors_route_projects_owner_governors_and_titles() {
    let (app, state) = build_app();
    let token = bootstrap_token(&app).await;

    // Fresh game: no governors, zero titles.
    let (status, view) = get_with(&app, "/api/v1/governors", &token).await;
    assert_eq!(status, StatusCode::OK, "governors: {view:?}");
    assert_eq!(view["titles_available"], 0);
    assert_eq!(view["governors"].as_array().expect("array").len(), 0);

    // Seed: grant titles and appoint an established governor for the player's
    // civ (civilizations[0]), assigned to a city it owns if one exists.
    let assigned_city_name: Option<String> = {
        let game_id = {
            let entry = state.games.iter().next().expect("one game");
            *entry.key()
        };
        let mut room = state.games.get_mut(&game_id).expect("game present");
        let civ_id = room.state.civilizations[0].id;
        room.state.civilizations[0].governor_titles = 2;

        let city = room
            .state
            .cities
            .iter()
            .find(|c| c.owner == civ_id)
            .map(|c| (c.id, c.name.clone()));

        let gov_id = libciv::GovernorId::from_ulid(ulid::Ulid::from(42u128));
        let mut governor = libciv::civ::Governor::new(gov_id, "Magnus", civ_id);
        governor.turns_to_establish = 0; // established
        governor.promotions.push("Groundbreaker");
        governor.assigned_city = city.as_ref().map(|(id, _)| *id);
        room.state.governors.push(governor);

        city.map(|(_, name)| name)
    };

    // Re-read: the seeded governor and title count are projected.
    let (status, view) = get_with(&app, "/api/v1/governors", &token).await;
    assert_eq!(status, StatusCode::OK, "governors: {view:?}");
    assert_eq!(view["titles_available"], 2);

    let govs = view["governors"].as_array().expect("array");
    assert_eq!(govs.len(), 1, "exactly one owned governor: {view:?}");
    let g = &govs[0];
    assert_eq!(g["name"], "Magnus");
    assert_eq!(g["established"], true);
    assert_eq!(g["turns_to_establish"], 0);
    assert_eq!(
        g["promotions"].as_array().expect("promotions array").len(),
        1
    );
    assert_eq!(g["promotions"][0], "Groundbreaker");
    match assigned_city_name {
        Some(name) => assert_eq!(g["assigned_city"], name),
        None => assert!(g["assigned_city"].is_null()),
    }
}
