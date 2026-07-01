//! Round-trip the SDK's typed endpoint functions against the in-process
//! Axum router via `tower::ServiceExt::oneshot` — no TCP, no server
//! lifecycle, no port allocation.
//!
//! The harness here mirrors `open4x-server/tests/rest_api.rs`. The
//! material difference is that we drive every call through a
//! `OneshotTransport` wrapper instead of reqwest, which exercises the
//! `Transport` trait and the typed endpoint functions at the same time.
//!
//! `OneshotTransport` lives in this test file (not in `open4x-sdk/src/`)
//! because it depends on `open4x-server`, which is a `[dev-dependencies]`
//! relationship only.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt as _;
use tower::ServiceExt;

use open4x_sdk::endpoints;
use open4x_sdk::error::ApiError;
use open4x_sdk::transport::{Method, Transport};

use open4x_server::server::rest::v1_router;
use open4x_server::server::state::AppState;

// ── transport wrapping the Axum router ──────────────────────────────────────

struct OneshotTransport {
    router: Router,
    token: Mutex<Option<String>>,
}

impl OneshotTransport {
    fn new(state: Arc<AppState>) -> Self {
        let router = Router::new().nest("/api/v1", v1_router()).with_state(state);
        Self {
            router,
            token: Mutex::new(None),
        }
    }

    fn set_token(&self, token: impl Into<String>) {
        *self.token.lock().unwrap() = Some(token.into());
    }
}

impl Transport for OneshotTransport {
    fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&[u8]>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, ApiError>> + Send {
        let mut builder = Request::builder().method(method.as_str()).uri(path);
        if let Some(token) = self.token.lock().unwrap().clone() {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let request = if let Some(b) = body {
            builder
                .header("content-type", "application/json")
                .body(Body::from(b.to_vec()))
                .expect("build request")
        } else {
            builder.body(Body::empty()).expect("build request")
        };
        let router = self.router.clone();
        async move {
            let resp = router
                .oneshot(request)
                .await
                .map_err(|e| ApiError::transport(format!("oneshot: {e}")))?;
            let status = resp.status().as_u16();
            let collected = resp
                .into_body()
                .collect()
                .await
                .map_err(|e| ApiError::transport(format!("body: {e}")))?;
            let bytes = collected.to_bytes();
            if (200..300).contains(&status) {
                Ok(bytes.to_vec())
            } else {
                let body = std::str::from_utf8(&bytes).unwrap_or("");
                Err(ApiError::from_response(status, body))
            }
        }
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn build_transport() -> OneshotTransport {
    let state = AppState::new();
    OneshotTransport::new(state)
}

async fn bootstrap(transport: &OneshotTransport) -> endpoints::games::NewGameResponse {
    let req = endpoints::games::NewGameRequest {
        width: Some(12),
        height: Some(8),
        seed: Some(7),
        num_ai: Some(0),
        turn_limit: Some(50),
        ..Default::default()
    };
    let resp = endpoints::games::new_game(transport, &req)
        .await
        .expect("bootstrap should succeed");
    transport.set_token(resp.token.clone());
    resp
}

// ── tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_round_trip() {
    let transport = build_transport();
    let resp = endpoints::health::health(&transport)
        .await
        .expect("health should succeed");
    assert!(resp.ok);
    assert_eq!(resp.api, "v1");
}

#[tokio::test]
async fn unauthenticated_endpoint_returns_structured_error() {
    let transport = build_transport();
    let err = endpoints::player_state::get(&transport)
        .await
        .expect_err("expected 401");
    assert_eq!(err.status, 401);
    assert_eq!(err.code, "missing_or_invalid_token");
}

#[tokio::test]
async fn games_new_mints_token_and_player_state_uses_it() {
    let transport = build_transport();
    let game = bootstrap(&transport).await;
    assert_eq!(game.turn, 0);
    assert!(!game.token.is_empty());

    let ps = endpoints::player_state::get(&transport)
        .await
        .expect("player_state");
    assert_eq!(ps.turn, 0);
    assert_eq!(ps.turn_max, 50);
    assert_eq!(ps.era, "Ancient");
}

#[tokio::test]
async fn world_snapshot_returns_tiles_and_dimensions() {
    let transport = build_transport();
    bootstrap(&transport).await;

    let snap = endpoints::world::snapshot(&transport, None, None, Some(4))
        .await
        .expect("snapshot");
    assert_eq!(snap.world.width, 12);
    assert_eq!(snap.world.height, 8);
    assert!(!snap.tiles.is_empty(), "expected at least one tile in view");
}

#[tokio::test]
async fn cities_list_units_list_and_diplomacy_round_trip() {
    let transport = build_transport();
    bootstrap(&transport).await;

    let cities = endpoints::cities::list(&transport).await.expect("cities");
    assert!(!cities.cities.is_empty(), "expected at least one own city");

    let units = endpoints::units::list(&transport).await.expect("units");
    assert!(!units.units.is_empty(), "expected at least one unit");

    let dip = endpoints::diplomacy::get(&transport)
        .await
        .expect("diplomacy");
    // Single-player solo game: 0 other civs.
    assert_eq!(dip.civs.len(), 0);
}

#[tokio::test]
async fn tech_get_and_research_and_end_turn_advance() {
    let transport = build_transport();
    bootstrap(&transport).await;

    // Pick a research target so 'choose_research' clears.
    let tt = endpoints::tech::get(&transport).await.expect("tech");
    let tech_id = tt
        .techs
        .iter()
        .find(|t| t.status == "available")
        .map(|t| t.id.clone())
        .expect("at least one available tech");
    endpoints::tech::research(&transport, &endpoints::tech::TechResearchBody { tech_id })
        .await
        .expect("research tech");

    // …and a civic so 'choose_civic' clears (RulesEngine::pending_actions
    // surfaces both as required on a fresh game).
    let ct = endpoints::civics::get(&transport).await.expect("civics");
    let civic_id = ct
        .civics
        .iter()
        .find(|c| c.status == "available")
        .map(|c| c.id.clone())
        .expect("at least one available civic");
    endpoints::civics::research(
        &transport,
        &endpoints::civics::CivicResearchBody { civic_id },
    )
    .await
    .expect("research civic");

    let end = endpoints::turn::end(&transport).await.expect("end turn");
    assert!(end.ok);
    assert_eq!(end.view.turn, 1);
    assert_eq!(end.turn_status.turn, 1);
}

#[tokio::test]
async fn end_turn_blocks_when_required_action_pending() {
    let transport = build_transport();
    bootstrap(&transport).await;

    // Fresh game: research queue empty -> 'choose_research' is required.
    let err = endpoints::turn::end(&transport)
        .await
        .expect_err("should reject");
    assert_eq!(err.status, 400);
    assert_eq!(err.code, "unresolved_required_actions");
}

#[tokio::test]
async fn smoke_remaining_endpoints_all_return_2xx() {
    // One combined smoke test that exercises every endpoint not covered
    // by the focussed tests above. The assertion is only "no panic, no
    // 5xx" — the focussed tests above already check the wire shapes for
    // the load-bearing endpoints.
    let transport = build_transport();
    bootstrap(&transport).await;

    endpoints::map::overlays(&transport)
        .await
        .expect("map overlays");
    endpoints::registry::get(&transport)
        .await
        .expect("registry");
    endpoints::empire::overview(&transport)
        .await
        .expect("empire overview");
    endpoints::victory::get(&transport).await.expect("victory");
    endpoints::government::get(&transport)
        .await
        .expect("government");
    endpoints::notifications::list(&transport)
        .await
        .expect("notifications");
    endpoints::notifications::dismiss_all(&transport)
        .await
        .expect("dismiss all");
    endpoints::armies::list(&transport).await.expect("armies");
    endpoints::turn::queue(&transport)
        .await
        .expect("turn queue");

    // world tile lookup — pick a tile we know is in view (the snapshot
    // returned tiles around 0,0).
    let snap = endpoints::world::snapshot(&transport, None, None, Some(4))
        .await
        .expect("snapshot");
    let some_tile = snap.tiles.first().expect("at least one tile");
    endpoints::world::tile(&transport, some_tile.q, some_tile.r)
        .await
        .expect("world tile");

    // Per-civ diplomacy lookup. Solo game has no other civs, so a fetch
    // by id of "not_a_civ" should 404 — that's the surface we want to
    // verify (structured-error decoding), not a successful read.
    let err = endpoints::diplomacy::civ(&transport, "not_a_civ")
        .await
        .expect_err("expected 404 for unknown civ");
    assert_eq!(err.status, 404);

    // City detail + tiles for the first owned city.
    let cities = endpoints::cities::list(&transport).await.expect("cities");
    let city_id = cities.cities[0].id.clone();
    endpoints::cities::detail(&transport, &city_id)
        .await
        .expect("city detail");
    endpoints::cities::tiles(&transport, &city_id)
        .await
        .expect("city tiles");

    // Unit detail for the first owned unit.
    let units = endpoints::units::list(&transport).await.expect("units");
    let unit_id = units
        .units
        .iter()
        .find(|u| u.is_own)
        .map(|u| u.id.clone())
        .expect("at least one own unit");
    endpoints::units::detail(&transport, &unit_id)
        .await
        .expect("unit detail");

    // Combat preview points at a tile with no defender → server returns
    // 404 with a structured body. Either way the SDK decodes it.
    let cp = endpoints::combat::preview(&transport, &unit_id, 99, 99).await;
    if let Err(e) = cp {
        assert_eq!(e.status, StatusCode::NOT_FOUND.as_u16());
    }
}

#[tokio::test]
async fn city_mutations_round_trip() {
    let transport = build_transport();
    bootstrap(&transport).await;

    let cities = endpoints::cities::list(&transport).await.expect("cities");
    let city_id = cities.cities[0].id.clone();

    // rename
    let rename = endpoints::cities::rename(
        &transport,
        &city_id,
        &endpoints::cities::RenameCityBody {
            name: "  New Capital  ".into(),
        },
    )
    .await
    .expect("rename");
    assert!(rename.ok);
    assert_eq!(rename.view.name, "New Capital");

    // focus
    let focus = endpoints::cities::assign_focus(
        &transport,
        &city_id,
        &endpoints::cities::AssignCityFocusBody {
            focus: "production".into(),
        },
    )
    .await
    .expect("focus");
    assert_eq!(focus.view.focus, "production");

    // production queue: queue a warrior then cancel it
    let reg = endpoints::registry::get(&transport)
        .await
        .expect("registry");
    let warrior_type = reg
        .unit_types
        .iter()
        .find(|u| u.name == "warrior")
        .map(|u| u.id.clone())
        .expect("warrior unit type");
    let queued = endpoints::cities::queue_production(
        &transport,
        &city_id,
        &endpoints::cities::QueueProductionBody {
            item_id: warrior_type,
            item_type: "unit".into(),
        },
    )
    .await
    .expect("queue production");
    assert_eq!(queued.view.production_queue[0], "Warrior");

    let cancelled = endpoints::cities::cancel_production(&transport, &city_id, 0)
        .await
        .expect("cancel production");
    assert!(cancelled.view.production_queue.is_empty());
}

#[tokio::test]
async fn tech_cancel_is_idempotent() {
    let transport = build_transport();
    bootstrap(&transport).await;

    // Queue then cancel.
    let tt = endpoints::tech::get(&transport).await.expect("tech");
    let tech_id = tt
        .techs
        .iter()
        .find(|t| t.status == "available")
        .map(|t| t.id.clone())
        .expect("available tech");
    endpoints::tech::research(&transport, &endpoints::tech::TechResearchBody { tech_id })
        .await
        .expect("queue tech");

    let cancel = endpoints::tech::cancel(&transport)
        .await
        .expect("cancel tech");
    assert!(cancel.view.research_queue.is_empty());

    // Second cancel is a no-op.
    endpoints::tech::cancel(&transport)
        .await
        .expect("second cancel still ok");
}

#[tokio::test]
async fn government_change_rejects_unknown_with_structured_error() {
    let transport = build_transport();
    bootstrap(&transport).await;

    let err = endpoints::government::change(
        &transport,
        &endpoints::government::ChangeGovernmentBody {
            government: "Atlantean Republic".into(),
        },
    )
    .await
    .expect_err("unknown government should 400");
    assert_eq!(err.status, 400);
    // The code is rule-violation flavoured; just verify the SDK
    // surfaced *something* and didn't swallow the status.
    assert!(!err.code.is_empty());
}
