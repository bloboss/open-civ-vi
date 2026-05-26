//! OpenAPI 3 document for the `/api/v1/*` surface.
//!
//! This module is the **single source of truth** for the REST contract. It
//! pairs each route registered in [`crate::server::rest::v1_router`] with a
//! `#[utoipa::path(...)]` annotation that describes its method, path,
//! parameters, request body, and response shape. The `ApiDoc` struct at the
//! bottom assembles all of those paths + every wire-type schema from
//! `open4x-protocol::v1::web::*` into one `utoipa::openapi::OpenApi` value.
//!
//! Two integration tests in `tests/openapi_contract.rs` keep this in sync:
//! one asserts every path string here is reachable through `v1_router`, and
//! one asserts every router path has a matching `#[utoipa::path]`. The
//! `gen-openapi` binary (`src/bin/gen_openapi.rs`) writes the result to
//! `book/src/multiplayer/openapi.json`.
//!
//! Compiled only with the `openapi` cargo feature; absent in default builds
//! and absent from the wasm32 client.

#![allow(non_snake_case, dead_code)]

use utoipa::OpenApi;

use crate::server::rest::handlers::{
    AssignCityFocusBody, ChangeGovernmentBody, CivicResearchBody, EndTurnView, HealthResponse,
    NewGameRequest, NewGameResponse, QueueProductionBody, RenameCityBody, TechResearchBody,
    UnitActionBody,
};
use open4x_protocol::v1::web::{
    ApiErrorBody, TurnStatusBlock,
    army_data::{Army, ArmyData},
    city_data::{CityData, CityRow as CityDataRow},
    city_tiles::{CityTiles, TileEntry},
    civics_tree::{CivicNode, CivicsTreeView},
    combat_preview::{CombatPreview, DefenderInfo},
    diplomacy::{CityStateRow, CivRow, DealDraft, Diplomacy, RelationModifier},
    empire_overview::{
        CityRow as EmpireCityRow, EmpireOverview, ResourceRow, Summary, TradeRow,
    },
    government::{ActivePolicy, Government, GovernmentPolicies, PolicyCard, Slots},
    map_overlays::{MapOverlays, Overlay},
    notifications::{Notification, NotificationTarget, Notifications},
    player_state::{Bucket, PlayerState, Resources},
    registry::{Building, Registry, UnitType},
    tech_tree::{TechNode, TechTreeView},
    turn_queue::{TurnQueue, TurnQueueItem},
    unit_data::{Unit, UnitAction, UnitData},
    victory::{Condition, LeaderRow, Victory},
    world::{
        Camera, Legend, TileCity, TileCoord, TileUnit, TileView, TileYields, WorldMeta,
        WorldSnapshot,
    },
};

// ── Tag constants ────────────────────────────────────────────────────────────
//
// Tags group endpoints in the rendered docs. Order matches the wireframe
// screen layout so the generated spec reads top-down like the UI.

const TAG_META: &str = "Meta";
const TAG_GAMES: &str = "Games";
const TAG_HUD: &str = "HUD";
const TAG_WORLD: &str = "World";
const TAG_CITIES: &str = "Cities";
const TAG_UNITS: &str = "Units";
const TAG_RESEARCH: &str = "Research";
const TAG_CIVICS: &str = "Civics";
const TAG_GOVERNMENT: &str = "Government";
const TAG_DIPLOMACY: &str = "Diplomacy";
const TAG_EMPIRE: &str = "Empire";
const TAG_VICTORY: &str = "Victory";
const TAG_NOTIFICATIONS: &str = "Notifications";
const TAG_TURN: &str = "Turn";
const TAG_REGISTRY: &str = "Registry";

// ── Doc-only proxy functions ─────────────────────────────────────────────────
//
// Every `#[utoipa::path]` below attaches to a no-op `_doc_*` function. We keep
// the metadata here instead of on the real handlers so `handlers.rs` stays
// readable. Each `_doc_*` mirrors the route registered in
// `rest::mod::v1_router()` 1:1; the contract-drift test fails if the two
// drift apart.

// ── Meta ─────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/health",
    tag = TAG_META,
    responses(
        (status = 200, description = "Liveness probe. Unauthenticated.", body = HealthResponse),
    ),
)]
fn _doc_health() {}

// ── Games (bootstrap) ────────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/games/new",
    tag = TAG_GAMES,
    request_body = NewGameRequest,
    responses(
        (status = 201, description = "New single-player game created. Returned token authenticates every subsequent /api/v1 call.", body = NewGameResponse),
    ),
)]
fn _doc_games_new() {}

// ── HUD ──────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/player-state",
    tag = TAG_HUD,
    responses(
        (status = 200, description = "Resource bar + global player state for the authenticated civ.", body = PlayerState),
        (status = 401, description = "Missing or invalid bearer token.", body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_player_state() {}

// ── World ────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/world/snapshot",
    tag = TAG_WORLD,
    params(
        ("q" = Option<i32>, Query, description = "Camera centre column (axial q). Defaults to 0."),
        ("r" = Option<i32>, Query, description = "Camera centre row (axial r). Defaults to 0."),
        ("radius" = Option<u32>, Query, description = "Cube-distance radius around (q, r). 0 = all explored. Hard-capped at 32."),
    ),
    responses(
        (status = 200, description = "Sparse fog-of-war-filtered tile list around the camera.", body = WorldSnapshot),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_world_snapshot() {}

#[utoipa::path(
    get, path = "/api/v1/world/tile/{q}/{r}",
    tag = TAG_WORLD,
    params(
        ("q" = i32, Path, description = "Axial q (column)."),
        ("r" = i32, Path, description = "Axial r (row)."),
    ),
    responses(
        (status = 200, description = "Single tile detail.", body = TileView),
        (status = 404, description = "Tile not in player view.", body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_world_tile() {}

#[utoipa::path(
    get, path = "/api/v1/map/overlays",
    tag = TAG_WORLD,
    responses(
        (status = 200, description = "Catalogue of toggleable map overlays. Toggle state is client-side.", body = MapOverlays),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_map_overlays() {}

// ── Cities ───────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/cities",
    tag = TAG_CITIES,
    responses(
        (status = 200, description = "All known cities. Own cities include full production detail; foreign cities are limited.", body = CityData),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_cities() {}

#[utoipa::path(
    get, path = "/api/v1/cities/{id}",
    tag = TAG_CITIES,
    params(("id" = String, Path, description = "City ULID.")),
    responses(
        (status = 200, body = CityDataRow),
        (status = 404, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_city_detail() {}

#[utoipa::path(
    get, path = "/api/v1/cities/{id}/tiles",
    tag = TAG_CITIES,
    params(("id" = String, Path, description = "City ULID.")),
    responses(
        (status = 200, description = "Territory + worked-tile flags for the city hex view.", body = CityTiles),
        (status = 404, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_city_tiles() {}

#[utoipa::path(
    post, path = "/api/v1/cities/{id}/production",
    tag = TAG_CITIES,
    params(("id" = String, Path, description = "City ULID.")),
    request_body = QueueProductionBody,
    responses(
        (status = 200, description = "Item appended to the production queue. Returns the updated city row.", body = MutationResponseCityRow),
        (status = 400, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_queue_production() {}

#[utoipa::path(
    delete, path = "/api/v1/cities/{id}/production/{pos}",
    tag = TAG_CITIES,
    params(
        ("id" = String, Path, description = "City ULID."),
        ("pos" = usize, Path, description = "Queue index. 0 = active."),
    ),
    responses(
        (status = 200, description = "Item removed. Returns the updated city row.", body = MutationResponseCityRow),
        (status = 400, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_cancel_production() {}

#[utoipa::path(
    post, path = "/api/v1/cities/{id}/focus",
    tag = TAG_CITIES,
    params(("id" = String, Path, description = "City ULID.")),
    request_body = AssignCityFocusBody,
    responses(
        (status = 200, description = "Focus updated. Returns the city row.", body = MutationResponseCityRow),
        (status = 400, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_assign_city_focus() {}

#[utoipa::path(
    post, path = "/api/v1/cities/{id}/rename",
    tag = TAG_CITIES,
    params(("id" = String, Path, description = "City ULID.")),
    request_body = RenameCityBody,
    responses(
        (status = 200, description = "City renamed (1..=64 chars after trim).", body = MutationResponseCityRow),
        (status = 400, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_rename_city() {}

// ── Units ────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/units",
    tag = TAG_UNITS,
    responses(
        (status = 200, body = UnitData),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_units() {}

#[utoipa::path(
    get, path = "/api/v1/units/{id}",
    tag = TAG_UNITS,
    params(("id" = String, Path, description = "Unit ULID.")),
    responses(
        (status = 200, body = Unit),
        (status = 404, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_unit_detail() {}

#[utoipa::path(
    post, path = "/api/v1/units/{id}/action",
    tag = TAG_UNITS,
    params(("id" = String, Path, description = "Unit ULID.")),
    request_body = UnitActionBody,
    responses(
        (status = 200, description = "Action applied. View payload is the updated unit (or null for no-op).", body = MutationResponseJson),
        (status = 202, description = "Stubbed action (fortify/sleep). View payload is null.", body = MutationResponseJson),
        (status = 400, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_unit_action() {}

#[utoipa::path(
    get, path = "/api/v1/armies",
    tag = TAG_UNITS,
    responses(
        (status = 200, description = "Army/corps formations (stub; empty until libciv ships the system).", body = ArmyData),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_armies() {}

#[utoipa::path(
    get, path = "/api/v1/combat/preview",
    tag = TAG_UNITS,
    params(
        ("attacker_id" = String, Query, description = "Attacker unit ULID."),
        ("defender_q" = i32, Query, description = "Target hex q."),
        ("defender_r" = i32, Query, description = "Target hex r."),
    ),
    responses(
        (status = 200, description = "Heuristic combat preview. `note` flags the libciv-driven replacement.", body = CombatPreview),
        (status = 404, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_combat_preview() {}

// ── Research (tech) ──────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/tech",
    tag = TAG_RESEARCH,
    responses(
        (status = 200, body = TechTreeView),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_tech() {}

#[utoipa::path(
    post, path = "/api/v1/tech/research",
    tag = TAG_RESEARCH,
    request_body = TechResearchBody,
    responses(
        (status = 200, description = "Queued research target.", body = MutationResponseTechTreeView),
        (status = 400, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_tech_research() {}

#[utoipa::path(
    delete, path = "/api/v1/tech/research",
    tag = TAG_RESEARCH,
    responses(
        (status = 200, description = "Active research dropped (idempotent — no-op on empty queue). Partial progress is discarded.", body = MutationResponseTechTreeView),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_cancel_research() {}

// ── Civics ───────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/civics",
    tag = TAG_CIVICS,
    responses(
        (status = 200, body = CivicsTreeView),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_civics() {}

#[utoipa::path(
    post, path = "/api/v1/civics/research",
    tag = TAG_CIVICS,
    request_body = CivicResearchBody,
    responses(
        (status = 200, description = "Queued civic target.", body = MutationResponseCivicsTreeView),
        (status = 400, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_civic_research() {}

#[utoipa::path(
    delete, path = "/api/v1/civics/research",
    tag = TAG_CIVICS,
    responses(
        (status = 200, description = "Active civic dropped (idempotent). Partial progress discarded.", body = MutationResponseCivicsTreeView),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_cancel_civic() {}

// ── Government ───────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/government",
    tag = TAG_GOVERNMENT,
    responses(
        (status = 200, body = GovernmentPolicies),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_government() {}

#[utoipa::path(
    post, path = "/api/v1/government/change",
    tag = TAG_GOVERNMENT,
    request_body = ChangeGovernmentBody,
    responses(
        (status = 200, description = "Government switched. Active policies that no longer fit the slot config are unslotted.", body = MutationResponseGovernmentPolicies),
        (status = 400, description = "Unknown or not-yet-unlocked government.", body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_change_government() {}

// ── Diplomacy ────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/diplomacy",
    tag = TAG_DIPLOMACY,
    responses(
        (status = 200, body = Diplomacy),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_diplomacy() {}

#[utoipa::path(
    get, path = "/api/v1/diplomacy/civs/{id}",
    tag = TAG_DIPLOMACY,
    params(("id" = String, Path, description = "Civ ULID.")),
    responses(
        (status = 200, body = CivRow),
        (status = 404, body = ApiErrorBody),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_diplomacy_civ() {}

// ── Empire ───────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/empire/overview",
    tag = TAG_EMPIRE,
    responses(
        (status = 200, body = EmpireOverview),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_empire_overview() {}

// ── Victory ──────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/victory",
    tag = TAG_VICTORY,
    responses(
        (status = 200, body = Victory),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_victory() {}

// ── Notifications + Turn queue ───────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/notifications",
    tag = TAG_NOTIFICATIONS,
    responses(
        (status = 200, body = Notifications),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_notifications() {}

#[utoipa::path(
    delete, path = "/api/v1/notifications",
    tag = TAG_NOTIFICATIONS,
    responses(
        (status = 204, description = "All notifications for the authenticated civ dismissed."),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_dismiss_all_notifications() {}

#[utoipa::path(
    delete, path = "/api/v1/notifications/{id}",
    tag = TAG_NOTIFICATIONS,
    params(("id" = String, Path, description = "Notification id.")),
    responses(
        (status = 204, description = "Notification dismissed (idempotent on unknown ids)."),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_dismiss_notification() {}

#[utoipa::path(
    get, path = "/api/v1/turn-queue",
    tag = TAG_TURN,
    responses(
        (status = 200, description = "Required and skippable pending actions for the current turn.", body = TurnQueue),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_turn_queue() {}

#[utoipa::path(
    post, path = "/api/v1/turn/end",
    tag = TAG_TURN,
    responses(
        (status = 200, description = "Turn advanced. View payload is `{ turn }`.", body = MutationResponseEndTurnView),
        (status = 400, description = "Required turn-queue items unresolved. Body shape: `{ error: \"unresolved_required_actions\", items: [...] }`.", body = serde_json::Value),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_end_turn() {}

// ── Registry ─────────────────────────────────────────────────────────────────

#[utoipa::path(
    get, path = "/api/v1/registry",
    tag = TAG_REGISTRY,
    responses(
        (status = 200, description = "Unit-type and building catalogues. Stable across turns.", body = Registry),
        (status = 401, body = ApiErrorBody),
    ),
    security(("bearer" = [])),
)]
fn _doc_registry() {}

// ── Generic MutationResponse<T> aliases ──────────────────────────────────────
//
// `utoipa::ToSchema` is monomorphic, so we name one alias per `T` that the
// handlers actually return. These mirror the real
// `open4x_protocol::v1::web::MutationResponse<T>` shape (`{ ok, view, turn_status }`).

#[derive(serde::Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct MutationResponseCityRow {
    pub ok: bool,
    pub view: CityDataRow,
    pub turn_status: TurnStatusBlock,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct MutationResponseTechTreeView {
    pub ok: bool,
    pub view: TechTreeView,
    pub turn_status: TurnStatusBlock,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct MutationResponseCivicsTreeView {
    pub ok: bool,
    pub view: CivicsTreeView,
    pub turn_status: TurnStatusBlock,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct MutationResponseGovernmentPolicies {
    pub ok: bool,
    pub view: GovernmentPolicies,
    pub turn_status: TurnStatusBlock,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct MutationResponseEndTurnView {
    pub ok: bool,
    pub view: EndTurnView,
    pub turn_status: TurnStatusBlock,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct MutationResponseJson {
    pub ok: bool,
    #[schema(value_type = Object, additional_properties)]
    pub view: serde_json::Value,
    pub turn_status: TurnStatusBlock,
}

// ── ApiDoc ───────────────────────────────────────────────────────────────────

#[derive(OpenApi)]
#[openapi(
    info(
        title = "open4x REST API",
        version = "1.0.0",
        description = "REST surface for the open4x single-player Civ-VI-style 4X. Every \
endpoint round-trips JSON; mutations return a `MutationResponse<T>` envelope \
with the freshly-projected slice and a turn-status block so the client can \
refresh the HUD bar without a second request.\n\n\
**Auth.** Bootstrap via `POST /api/v1/games/new` to mint a bearer token. \
Every other endpoint (besides `/health`) requires \
`Authorization: Bearer <token>`.\n\n\
**Source of truth.** This document is generated from the Rust source by \
`cargo run -p open4x-server --features openapi --bin gen-openapi`. Do not \
hand-edit `book/src/multiplayer/openapi.json`.\n\n\
**Multiplayer.** The WebSocket surface at `/ws` (Ed25519-authenticated; \
`ClientMessage` / `ServerMessage` JSON frames) is **not** described here. \
See `book/src/multiplayer/protocol.md` and \
`open4x-protocol::v1::messages` for those types.",
        contact(name = "open-civ-vi", url = "https://github.com/bloboss/open-civ-vi"),
        license(name = "Apache-2.0"),
    ),
    servers(
        (url = "/", description = "Same-origin (browser default)"),
        (url = "http://localhost:3001", description = "Local dev server (OPEN4X port)"),
    ),
    tags(
        (name = TAG_META,          description = "Liveness and metadata."),
        (name = TAG_GAMES,         description = "Game bootstrap. Mints a bearer token."),
        (name = TAG_HUD,           description = "Top resource bar / global player state."),
        (name = TAG_WORLD,         description = "Hex map, tile detail, overlay catalogue."),
        (name = TAG_CITIES,        description = "City list, detail, tile work, production queue, focus, rename."),
        (name = TAG_UNITS,         description = "Unit list, detail, action dispatch, combat preview, armies."),
        (name = TAG_RESEARCH,      description = "Tech tree + research queue."),
        (name = TAG_CIVICS,        description = "Civic tree + civic queue."),
        (name = TAG_GOVERNMENT,    description = "Current government, policy slots, catalogue."),
        (name = TAG_DIPLOMACY,     description = "Known civs and city-states."),
        (name = TAG_EMPIRE,        description = "Aggregate dashboard."),
        (name = TAG_VICTORY,       description = "Per-condition progress + leaderboard."),
        (name = TAG_NOTIFICATIONS, description = "Event feed (per-civ ring buffer, cap 64)."),
        (name = TAG_TURN,          description = "Turn queue + end-turn."),
        (name = TAG_REGISTRY,      description = "Static unit-type and building catalogue."),
    ),
    paths(
        _doc_health,
        _doc_games_new,
        _doc_player_state,
        _doc_world_snapshot,
        _doc_world_tile,
        _doc_map_overlays,
        _doc_cities,
        _doc_city_detail,
        _doc_city_tiles,
        _doc_queue_production,
        _doc_cancel_production,
        _doc_assign_city_focus,
        _doc_rename_city,
        _doc_units,
        _doc_unit_detail,
        _doc_unit_action,
        _doc_armies,
        _doc_combat_preview,
        _doc_tech,
        _doc_tech_research,
        _doc_cancel_research,
        _doc_civics,
        _doc_civic_research,
        _doc_cancel_civic,
        _doc_government,
        _doc_change_government,
        _doc_diplomacy,
        _doc_diplomacy_civ,
        _doc_empire_overview,
        _doc_victory,
        _doc_notifications,
        _doc_dismiss_all_notifications,
        _doc_dismiss_notification,
        _doc_turn_queue,
        _doc_end_turn,
        _doc_registry,
    ),
    components(schemas(
        // Server-defined DTOs.
        HealthResponse, NewGameRequest, NewGameResponse,
        QueueProductionBody, AssignCityFocusBody, RenameCityBody,
        UnitActionBody, TechResearchBody, CivicResearchBody,
        ChangeGovernmentBody, EndTurnView,
        // Common envelopes.
        ApiErrorBody, TurnStatusBlock,
        MutationResponseCityRow, MutationResponseTechTreeView,
        MutationResponseCivicsTreeView, MutationResponseGovernmentPolicies,
        MutationResponseEndTurnView, MutationResponseJson,
        // Wire types from open4x-protocol::v1::web::*.
        PlayerState, Resources, Bucket,
        WorldSnapshot, WorldMeta, Camera, Legend, TileView, TileYields, TileCity, TileUnit, TileCoord,
        CityData, CityDataRow,
        CityTiles, TileEntry,
        UnitData, Unit, UnitAction,
        ArmyData, Army,
        CombatPreview, DefenderInfo,
        TechTreeView, TechNode,
        CivicsTreeView, CivicNode,
        GovernmentPolicies, Government, Slots, ActivePolicy, PolicyCard,
        Diplomacy, CivRow, RelationModifier, CityStateRow, DealDraft,
        EmpireOverview, Summary, EmpireCityRow, ResourceRow, TradeRow,
        Victory, Condition, LeaderRow,
        Notifications, Notification, NotificationTarget,
        TurnQueue, TurnQueueItem,
        MapOverlays, Overlay,
        Registry, UnitType, Building,
    )),
    security(
        ("bearer" = [])
    ),
    modifiers(&BearerAuth),
)]
pub struct ApiDoc;

/// Attach `Authorization: Bearer <token>` as a global security scheme so the
/// generated docs surface it on every endpoint.
struct BearerAuth;

impl utoipa::Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
        let components = openapi
            .components
            .get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some(
                        "Token returned by `POST /api/v1/games/new`. Submit as \
`Authorization: Bearer <token>` on every subsequent request.",
                    ))
                    .build(),
            ),
        );
    }
}

/// Return the assembled OpenAPI 3 document. Calls `ApiDoc::openapi()` and is
/// the only entry point intended for use outside this module — the
/// `gen-openapi` binary and integration tests both call it.
pub fn document() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

/// Every path string registered with utoipa, in the order they appear in
/// `ApiDoc`. Used by the contract-drift test to assert 1:1 coverage with
/// `v1_router`.
pub fn declared_paths() -> Vec<(&'static str, &'static str)> {
    // (method, path) tuples. Method is lowercase to match utoipa internals.
    vec![
        ("get",    "/api/v1/health"),
        ("post",   "/api/v1/games/new"),
        ("get",    "/api/v1/player-state"),
        ("get",    "/api/v1/world/snapshot"),
        ("get",    "/api/v1/world/tile/{q}/{r}"),
        ("get",    "/api/v1/map/overlays"),
        ("get",    "/api/v1/cities"),
        ("get",    "/api/v1/cities/{id}"),
        ("get",    "/api/v1/cities/{id}/tiles"),
        ("post",   "/api/v1/cities/{id}/production"),
        ("delete", "/api/v1/cities/{id}/production/{pos}"),
        ("post",   "/api/v1/cities/{id}/focus"),
        ("post",   "/api/v1/cities/{id}/rename"),
        ("get",    "/api/v1/units"),
        ("get",    "/api/v1/units/{id}"),
        ("post",   "/api/v1/units/{id}/action"),
        ("get",    "/api/v1/armies"),
        ("get",    "/api/v1/combat/preview"),
        ("get",    "/api/v1/tech"),
        ("post",   "/api/v1/tech/research"),
        ("delete", "/api/v1/tech/research"),
        ("get",    "/api/v1/civics"),
        ("post",   "/api/v1/civics/research"),
        ("delete", "/api/v1/civics/research"),
        ("get",    "/api/v1/government"),
        ("post",   "/api/v1/government/change"),
        ("get",    "/api/v1/diplomacy"),
        ("get",    "/api/v1/diplomacy/civs/{id}"),
        ("get",    "/api/v1/empire/overview"),
        ("get",    "/api/v1/victory"),
        ("get",    "/api/v1/notifications"),
        ("delete", "/api/v1/notifications"),
        ("delete", "/api/v1/notifications/{id}"),
        ("get",    "/api/v1/turn-queue"),
        ("post",   "/api/v1/turn/end"),
        ("get",    "/api/v1/registry"),
    ]
}
