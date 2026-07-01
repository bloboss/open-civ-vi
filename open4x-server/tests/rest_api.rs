//! End-to-end integration tests for the `/api/v1/*` REST surface.
//!
//! These exercise the same `Router` `main.rs` mounts but skip the TCP
//! listener — every request goes through `tower::ServiceExt::oneshot`. That
//! means the tests are deterministic, parallel-safe, and add no port-binding
//! dance to CI.
//!
//! Each test follows the same shape:
//!   1. build a fresh `AppState` and the v1 `Router`
//!   2. POST `/api/v1/games/new` to get a bearer token
//!   3. exercise the endpoint(s) under test, asserting status + key fields
//!
//! Plan: book/src/roadmap/web-ui.md §4.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as _;
use serde_json::Value;
use tower::ServiceExt;

use open4x_server::server::rest::v1_router;
use open4x_server::server::state::AppState;

// ── test rig ────────────────────────────────────────────────────────────────

fn build_app() -> (Router, Arc<AppState>) {
    let state = AppState::new();
    let router = Router::new()
        .nest("/api/v1", v1_router())
        .with_state(state.clone());
    (router, state)
}

async fn json_body(resp: axum::response::Response) -> (StatusCode, Value) {
    let status = resp.status();
    let body = resp.into_body();
    let collected = body.collect().await.expect("collect body");
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

async fn post_with(
    app: &Router,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let resp = app
        .clone()
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    json_body(resp).await
}

async fn delete_with(app: &Router, path: &str, token: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    json_body(resp).await
}

/// Bootstrap a single-player session and return the bearer token.
async fn bootstrap_token(app: &Router) -> String {
    let body = serde_json::json!({
        "width": 12,
        "height": 8,
        "seed": 7,
        "num_ai": 0,
        "turn_limit": 50,
    });
    let (status, body) = post_with(app, "/api/v1/games/new", None, body).await;
    assert_eq!(status, StatusCode::CREATED, "games/new: {body:?}");
    body["token"].as_str().expect("token in response").to_string()
}

// ── tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_is_unauthenticated() {
    let (app, _state) = build_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = json_body(resp).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["api"], "v1");
}

#[tokio::test]
async fn unauthenticated_endpoint_rejects_missing_token() {
    let (app, _state) = build_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/player-state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, _) = json_body(resp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn games_new_mints_token_and_player_state_uses_it() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, ps) = get_with(&app, "/api/v1/player-state", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ps["turn"], 0);
    assert_eq!(ps["turn_max"], 50);
    assert_eq!(ps["era"], "Ancient");
    assert!(ps["resources"]["gold"].is_object());
}

#[tokio::test]
async fn world_snapshot_returns_tiles_and_dimensions() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, snap) = get_with(&app, "/api/v1/world/snapshot?radius=4", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(snap["world"]["width"], 12);
    assert_eq!(snap["world"]["height"], 8);
    let tiles = snap["tiles"].as_array().unwrap();
    assert!(!tiles.is_empty(), "expected at least one tile in view");
}

#[tokio::test]
async fn cities_units_and_diplomacy_round_trip() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, cities) = get_with(&app, "/api/v1/cities", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(cities["cities"].as_array().unwrap().len() >= 1);

    let (status, units) = get_with(&app, "/api/v1/units", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(units["units"].as_array().unwrap().len() >= 1);

    let (status, dip) = get_with(&app, "/api/v1/diplomacy", &token).await;
    assert_eq!(status, StatusCode::OK);
    // Single-player solo game: 0 other civs.
    assert_eq!(dip["civs"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn end_turn_blocks_when_required_action_pending() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    // Fresh game: research queue empty -> 'choose_research' is required.
    let (status, body) = post_with(&app, "/api/v1/turn/end", Some(&token), serde_json::json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "turn/end body: {body:?}");
    assert_eq!(body["error"], "unresolved_required_actions");
    let items = body["items"].as_array().expect("items array");
    assert!(items.iter().any(|it| it["id"] == "choose_research"));
}

#[tokio::test]
async fn end_turn_advances_after_research_chosen() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    // Pick a research target so 'choose_research' clears.
    let (status, tt) = get_with(&app, "/api/v1/tech", &token).await;
    assert_eq!(status, StatusCode::OK);
    let tech_id = tt["techs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["status"] == "available")
        .and_then(|t| t["id"].as_str())
        .expect("at least one available tech")
        .to_string();

    let (status, _) = post_with(
        &app,
        "/api/v1/tech/research",
        Some(&token),
        serde_json::json!({"tech_id": tech_id}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Pick a civic target so 'choose_civic' clears (RulesEngine::pending_actions
    // surfaces both as required on a fresh game).
    let (status, ct) = get_with(&app, "/api/v1/civics", &token).await;
    assert_eq!(status, StatusCode::OK);
    let civic_id = ct["civics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["status"] == "available")
        .and_then(|c| c["id"].as_str())
        .expect("at least one available civic")
        .to_string();

    let (status, _) = post_with(
        &app,
        "/api/v1/civics/research",
        Some(&token),
        serde_json::json!({"civic_id": civic_id}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = post_with(&app, "/api/v1/turn/end", Some(&token), serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK, "after research: {body:?}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["view"]["turn"], 1);
}

#[tokio::test]
async fn production_queue_and_cancel_round_trip() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (_, cities) = get_with(&app, "/api/v1/cities", &token).await;
    let city_id = cities["cities"][0]["id"].as_str().unwrap().to_string();

    // Pull a unit-type id from /api/v1/registry.
    let (_, reg) = get_with(&app, "/api/v1/registry", &token).await;
    let warrior_type = reg["unit_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "warrior")
        .and_then(|d| d["id"].as_str())
        .expect("warrior unit type")
        .to_string();

    let (status, body) = post_with(
        &app,
        &format!("/api/v1/cities/{city_id}/production"),
        Some(&token),
        serde_json::json!({"item_id": warrior_type, "item_type": "unit"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "queue_production: {body:?}");
    assert_eq!(body["view"]["production_queue"][0], "Warrior");

    let (status, body) = delete_with(
        &app,
        &format!("/api/v1/cities/{city_id}/production/0"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "cancel_production: {body:?}");
    assert!(body["view"]["production_queue"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn units_actions_are_engine_derived_and_role_specific() {
    // Session builder spawns one warrior + one builder for the player.
    // Engine-derived actions: warrior gets attack/fortify; builder gets
    // build (because builders spawn with charges).
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, u) = get_with(&app, "/api/v1/units", &token).await;
    assert_eq!(status, StatusCode::OK);
    let units = u["units"].as_array().expect("units array");
    let own: Vec<_> = units.iter().filter(|u| u["is_own"] == true).collect();
    assert!(!own.is_empty(), "expected at least one own unit, got {units:?}");

    // Every own unit has Move + Sleep.
    for u in &own {
        let actions = u["actions"].as_array().expect("actions");
        let kinds: Vec<&str> = actions.iter().map(|a| a["id"].as_str().unwrap()).collect();
        assert!(kinds.contains(&"move"), "unit missing 'move': {u:?}");
        assert!(kinds.contains(&"sleep"), "unit missing 'sleep': {u:?}");
    }

    // At least one own unit (the warrior) has attack + fortify.
    assert!(
        own.iter().any(|u| {
            let actions = u["actions"].as_array().unwrap();
            let kinds: Vec<&str> = actions.iter().map(|a| a["id"].as_str().unwrap()).collect();
            kinds.contains(&"attack") && kinds.contains(&"fortify")
        }),
        "expected at least one combat unit with attack + fortify in {own:?}"
    );
}

#[tokio::test]
async fn change_government_rejects_unknown_and_locked_then_succeeds_when_unlocked() {
    let (app, state) = build_app();
    let token = bootstrap_token(&app).await;

    // Unknown government → 400.
    let (status, body) = post_with(
        &app,
        "/api/v1/government/change",
        Some(&token),
        serde_json::json!({"government": "Atlantean Republic"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "unknown gov body: {body:?}");

    // Known but not-yet-unlocked government → 400.
    let (status, _) = post_with(
        &app,
        "/api/v1/government/change",
        Some(&token),
        serde_json::json!({"government": "Monarchy"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Empty name → 400.
    let (status, _) = post_with(
        &app,
        "/api/v1/government/change",
        Some(&token),
        serde_json::json!({"government": "   "}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Pre-seed the only game's only civ with Chiefdom unlocked, then
    // verify a switch succeeds and the wire shape reflects it.
    {
        // There's exactly one game per test (bootstrap_token mints it).
        let game_entry = state.games.iter().next().expect("one game");
        let game_id = *game_entry.key();
        drop(game_entry);
        let mut room = state.games.get_mut(&game_id).expect("game still present");
        room.state.civilizations[0].unlocked_governments.push("Chiefdom");
    }

    let (status, body) = post_with(
        &app,
        "/api/v1/government/change",
        Some(&token),
        serde_json::json!({"government": "Chiefdom"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "unlock+switch body: {body:?}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["view"]["government"]["name"], "Chiefdom");

    // Re-read /government to confirm persistence.
    let (_, gov) = get_with(&app, "/api/v1/government", &token).await;
    assert_eq!(gov["government"]["name"], "Chiefdom");
}

#[tokio::test]
async fn cancel_civic_clears_active_and_is_idempotent() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    // Pick an available civic and start it.
    let (_, ct) = get_with(&app, "/api/v1/civics", &token).await;
    let civic_id = ct["civics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["status"] == "available")
        .and_then(|c| c["id"].as_str())
        .expect("at least one available civic")
        .to_string();
    let (status, _) = post_with(
        &app,
        "/api/v1/civics/research",
        Some(&token),
        serde_json::json!({"civic_id": civic_id}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // civic_queue should now have one entry.
    let (_, ct) = get_with(&app, "/api/v1/civics", &token).await;
    assert_eq!(
        ct["civic_queue"].as_array().unwrap().len(),
        1,
        "expected civic_queue to have one entry, got {ct:?}"
    );

    // DELETE — clears the slot.
    let (status, body) = delete_with(&app, "/api/v1/civics/research", &token).await;
    assert_eq!(status, StatusCode::OK, "cancel: {body:?}");
    assert_eq!(body["ok"], true);
    assert!(
        body["view"]["civic_queue"].as_array().unwrap().is_empty(),
        "expected empty civic_queue after cancel, got {body:?}"
    );

    // Idempotent: a second DELETE on an empty slot still 200s.
    let (status, _) = delete_with(&app, "/api/v1/civics/research", &token).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn cancel_research_drops_active_tech_and_is_idempotent() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    // Pick an available tech and queue it.
    let (_, tt) = get_with(&app, "/api/v1/tech", &token).await;
    let tech_id = tt["techs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["status"] == "available")
        .and_then(|t| t["id"].as_str())
        .expect("at least one available tech")
        .to_string();
    let (status, _) = post_with(
        &app,
        "/api/v1/tech/research",
        Some(&token),
        serde_json::json!({"tech_id": tech_id}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Confirm the tech is now active (research_queue is non-empty).
    let (_, tt) = get_with(&app, "/api/v1/tech", &token).await;
    assert!(
        !tt["research_queue"].as_array().unwrap().is_empty(),
        "expected research queue to have one entry after queue, got {tt:?}"
    );

    // DELETE — drops the active tech.
    let (status, body) = delete_with(&app, "/api/v1/tech/research", &token).await;
    assert_eq!(status, StatusCode::OK, "cancel: {body:?}");
    assert_eq!(body["ok"], true);
    assert!(
        body["view"]["research_queue"].as_array().unwrap().is_empty(),
        "expected empty research queue after cancel, got {body:?}"
    );

    // Idempotent: a second DELETE on an empty queue still succeeds.
    let (status, _) = delete_with(&app, "/api/v1/tech/research", &token).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn city_rename_round_trip_persists_via_post_rename() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (_, cities) = get_with(&app, "/api/v1/cities", &token).await;
    let city_id = cities["cities"][0]["id"].as_str().unwrap().to_string();
    let original = cities["cities"][0]["name"].as_str().unwrap().to_string();
    assert!(!original.is_empty());

    // Set a new name.
    let (status, body) = post_with(
        &app,
        &format!("/api/v1/cities/{city_id}/rename"),
        Some(&token),
        serde_json::json!({"name": "  New Capital  "}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "rename body: {body:?}");
    assert_eq!(body["ok"], true);
    // Server trims whitespace.
    assert_eq!(body["view"]["name"], "New Capital");

    // Re-read to confirm persistence.
    let (_, cities) = get_with(&app, "/api/v1/cities", &token).await;
    let row = cities["cities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == city_id.as_str())
        .unwrap();
    assert_eq!(row["name"], "New Capital");

    // Empty-name rejection.
    let (status, _) = post_with(
        &app,
        &format!("/api/v1/cities/{city_id}/rename"),
        Some(&token),
        serde_json::json!({"name": "   "}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // 65-char name rejection.
    let too_long = "x".repeat(65);
    let (status, _) = post_with(
        &app,
        &format!("/api/v1/cities/{city_id}/rename"),
        Some(&token),
        serde_json::json!({"name": too_long}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn city_focus_round_trip_persists_via_post_focus() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (_, cities) = get_with(&app, "/api/v1/cities", &token).await;
    let city_id = cities["cities"][0]["id"].as_str().unwrap().to_string();
    // Fresh game: focus defaults to "default".
    assert_eq!(cities["cities"][0]["focus"], "default");

    let (status, body) = post_with(
        &app,
        &format!("/api/v1/cities/{city_id}/focus"),
        Some(&token),
        serde_json::json!({"focus": "production"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "set focus body: {body:?}");
    assert_eq!(body["ok"], true);
    assert_eq!(body["view"]["focus"], "production");

    // Re-read /cities to confirm persistence.
    let (_, cities) = get_with(&app, "/api/v1/cities", &token).await;
    let row = cities["cities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == city_id.as_str())
        .unwrap();
    assert_eq!(row["focus"], "production");

    // Bad-focus rejection.
    let (status, _) = post_with(
        &app,
        &format!("/api/v1/cities/{city_id}/focus"),
        Some(&token),
        serde_json::json!({"focus": "wishful_thinking"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn government_catalogue_lists_engine_registered_policies_as_locked() {
    // GameState::new registers all 127 builtin policies. A fresh civ
    // has unlocked_policies == [] and active_policies == [], so the
    // catalogue should be all-locked.
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, g) = get_with(&app, "/api/v1/government", &token).await;
    assert_eq!(status, StatusCode::OK);
    let cat = g["catalogue"].as_array().expect("catalogue array");
    assert_eq!(cat.len(), 127, "expected 127 policies, got {}", cat.len());
    assert!(
        cat.iter().all(|p| p["status"] == "locked"),
        "expected every policy to be locked on a fresh game"
    );
    // Spot-check one well-known policy is present with the right type.
    let discipline = cat
        .iter()
        .find(|p| p["name"] == "Discipline")
        .expect("Discipline must be in the catalogue");
    assert_eq!(discipline["type"], "military");
    assert_eq!(discipline["unlock_civic"], "Code of Laws");
}

#[tokio::test]
async fn turn_queue_lists_required_choose_research_on_fresh_game() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, q) = get_with(&app, "/api/v1/turn-queue", &token).await;
    assert_eq!(status, StatusCode::OK);
    let items = q["items"].as_array().unwrap();
    assert!(items
        .iter()
        .any(|it| it["id"] == "choose_research" && it["required"] == true));
}

#[tokio::test]
async fn turn_queue_includes_civic_and_city_items_via_pending_actions() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, q) = get_with(&app, "/api/v1/turn-queue", &token).await;
    assert_eq!(status, StatusCode::OK);
    let items = q["items"].as_array().unwrap();

    // pending_actions surfaces choose_civic as required on a fresh game.
    assert!(
        items
            .iter()
            .any(|it| it["id"] == "choose_civic" && it["required"] == true),
        "expected choose_civic in {items:?}"
    );
    // …and at least one city-needs-production item (kind = "city").
    assert!(
        items.iter().any(|it| it["kind"] == "city"),
        "expected at least one kind=city item in {items:?}"
    );
}

#[tokio::test]
async fn victory_includes_six_conditions_and_ranks_player() {
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, v) = get_with(&app, "/api/v1/victory", &token).await;
    assert_eq!(status, StatusCode::OK);
    let conditions = v["conditions"].as_array().unwrap();
    assert_eq!(conditions.len(), 6);
    let names: Vec<&str> = conditions
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"score"));
    assert!(names.contains(&"culture"));
    assert!(names.contains(&"science"));
    let leaderboard = v["leaderboard"].as_array().unwrap();
    assert!(leaderboard.iter().any(|r| r["is_player"] == true));
}

#[tokio::test]
async fn victory_score_pct_is_engine_derived_not_zero() {
    // Session builder registers Score with turn_limit=500. compute_score is
    // > 0 on a fresh civ (founder bonus + capital), so player_pct on the
    // score row must be > 0 — proves we're going through
    // RulesEngine::victory_progress, not the legacy placeholder.
    let (app, _state) = build_app();
    let token = bootstrap_token(&app).await;

    let (status, v) = get_with(&app, "/api/v1/victory", &token).await;
    assert_eq!(status, StatusCode::OK);
    let score_row = v["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "score")
        .expect("score row");
    assert!(
        score_row["player_pct"].as_u64().unwrap() > 0,
        "expected score pct > 0, got {}",
        score_row["player_pct"]
    );
}

#[tokio::test]
async fn notifications_surface_fired_dynamic_event() {
    let (app, state) = build_app();
    let token = bootstrap_token(&app).await;

    // Seed the room so the next turn fires the `scientific_breakthrough`
    // event: queue a Eureka for the human civ. When the turn resolves it
    // emits an `EurekaTriggered` delta, the events phase turns that into an
    // `EventFired` delta, and `emit_notifications_from_diff` records a
    // notification — all reachable through the real `/notifications` route.
    {
        let game_entry = state.games.iter().next().expect("one game");
        let game_id = *game_entry.key();
        drop(game_entry);
        let mut room = state.games.get_mut(&game_id).expect("game present");
        let civ_id = room.players.first().expect("one player").civ_id;
        let tech = libciv::TechId::from_ulid(room.state.id_gen.next_ulid());
        room.state
            .effect_queue
            .push_back((civ_id, libciv::rules::OneShotEffect::TriggerEureka { tech }));
        room.resolve_turn();
    }

    let (status, notifs) = get_with(&app, "/api/v1/notifications", &token).await;
    assert_eq!(status, StatusCode::OK);
    let items = notifs["notifications"].as_array().expect("notifications array");
    let event = items
        .iter()
        .find(|n| n["title"] == "Scientific Breakthrough")
        .unwrap_or_else(|| panic!("no scientific_breakthrough notification in {items:?}"));
    assert_eq!(event["kind"], "good");
    assert_eq!(event["category"], "research");
}
