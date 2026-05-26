# REST API reference

> **Source of truth:** the Rust types in
> [`open4x-protocol`](https://github.com/bloboss/open-civ-vi/tree/main/open4x-protocol)
> and the path annotations in
> [`open4x-server/src/server/openapi.rs`](https://github.com/bloboss/open-civ-vi/blob/main/open4x-server/src/server/openapi.rs).
> The machine-readable spec is generated from those by
> `cargo run -p open4x-server --features openapi --bin gen-openapi` and
> shipped as [`openapi.json`](./openapi.json).
>
> Do not hand-edit `openapi.json`. The
> `openapi_paths_match_router` integration test fails CI if the doc and the
> live Axum router ever disagree.

## How the pipeline fits together

1. **Wire types** live in [`open4x-protocol::v1::web`](https://github.com/bloboss/open-civ-vi/tree/main/open4x-protocol/src/v1/web.rs).
   Every public struct derives `Serialize + Deserialize`, and under the
   `openapi` cargo feature it also derives `utoipa::ToSchema`.
2. **Handlers** in `open4x-server::server::rest::handlers` produce those
   types via projectors in `open4x-server::server::web_projection`.
3. **Route table.** `open4x-server::server::rest::v1_router` is the single
   place the routes are mounted; `main.rs` and the integration tests both
   use it.
4. **OpenAPI** assembly. `open4x-server::server::openapi::ApiDoc` carries
   one `#[utoipa::path]` per route, listing the request body, query params,
   path params, response shapes, and the bearer-auth requirement.
5. **Generator.** The `gen-openapi` binary emits
   `book/src/multiplayer/openapi.json`. CI regenerates and `git diff`s to
   catch silent drift.
6. **SDK.** `open4x-sdk::endpoints::*` is a thin typed client over the
   same wire types — every SDK function reads/writes types from
   `open4x-protocol::v1::web`.

## Conventions

- **Base path.** All endpoints live under `/api/v1`.
- **Auth.** Every endpoint except `/api/v1/health` requires
  `Authorization: Bearer <token>`. Tokens are minted by
  `POST /api/v1/games/new`.
- **IDs.** Path-level IDs (city, unit, civ, tech, civic, notification) are
  bare 26-character Crockford-base32 ULID strings (no `Type(...)` wrapper).
- **Coordinates.** Tile coords are axial `(q, r)` on the wire. The cube
  `s` component is recomputed server-side as `-q - r`.
- **Mutations** return the standard envelope:

  ```json
  { "ok": true, "view": <slice>, "turn_status": { "turn": 142, "ended": false } }
  ```

- **Errors** return one of:

  ```text
  400 → { "error": "<machine_code>", "message": "<human>" }
  400 → { "error": "unresolved_required_actions", "items": [...] }   (only on /turn/end)
  401 → { "error": "missing_or_invalid_token", "message": "..." }
  404 → { "error": "not_found", "message": "..." }
  ```

## Endpoint index

Grouped by tag. Read endpoints are idempotent; write endpoints round-trip a
mutation and return the freshly-projected slice in the standard envelope.

### Meta

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| GET | `/api/v1/health` | — | `HealthResponse` | none |

### Games

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| POST | `/api/v1/games/new` | `NewGameRequest` | `NewGameResponse` (201) | none |

### HUD

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| GET | `/api/v1/player-state` | — | `PlayerState` | bearer |

### World

| Method | Path | Query | Response | Auth |
|---|---|---|---|---|
| GET | `/api/v1/world/snapshot` | `q?, r?, radius?` (cap 32) | `WorldSnapshot` | bearer |
| GET | `/api/v1/world/tile/{q}/{r}` | — | `TileView` | bearer |
| GET | `/api/v1/map/overlays` | — | `MapOverlays` | bearer |

### Cities

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| GET    | `/api/v1/cities` | — | `CityData` | bearer |
| GET    | `/api/v1/cities/{id}` | — | `CityData::CityRow` | bearer |
| GET    | `/api/v1/cities/{id}/tiles` | — | `CityTiles` | bearer |
| POST   | `/api/v1/cities/{id}/production` | `QueueProductionBody` | `MutationResponse<CityRow>` | bearer |
| DELETE | `/api/v1/cities/{id}/production/{pos}` | — | `MutationResponse<CityRow>` | bearer |
| POST   | `/api/v1/cities/{id}/focus` | `AssignCityFocusBody` | `MutationResponse<CityRow>` | bearer |
| POST   | `/api/v1/cities/{id}/rename` | `RenameCityBody` (1..=64 chars) | `MutationResponse<CityRow>` | bearer |

### Units

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| GET  | `/api/v1/units` | — | `UnitData` | bearer |
| GET  | `/api/v1/units/{id}` | — | `Unit` | bearer |
| POST | `/api/v1/units/{id}/action` | `UnitActionBody` | `MutationResponse<Json>` | bearer |
| GET  | `/api/v1/armies` | — | `ArmyData` (stub) | bearer |
| GET  | `/api/v1/combat/preview` | `attacker_id, defender_q, defender_r` | `CombatPreview` | bearer |

Unit `action_id` values understood today: `move`, `attack`, `found_city`,
`fortify` (202 no-op), `sleep` (202 no-op). `fortify` and `sleep` are
accepted at the boundary but currently produce no engine-side effect;
plumbing through libciv is tracked under the Phase 4 work in
[`web-ui.md`](../roadmap/web-ui.md).

### Research

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| GET    | `/api/v1/tech` | — | `TechTreeView` | bearer |
| POST   | `/api/v1/tech/research` | `TechResearchBody` | `MutationResponse<TechTreeView>` | bearer |
| DELETE | `/api/v1/tech/research` | — | `MutationResponse<TechTreeView>` | bearer |

### Civics

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| GET    | `/api/v1/civics` | — | `CivicsTreeView` | bearer |
| POST   | `/api/v1/civics/research` | `CivicResearchBody` | `MutationResponse<CivicsTreeView>` | bearer |
| DELETE | `/api/v1/civics/research` | — | `MutationResponse<CivicsTreeView>` | bearer |

### Government

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| GET  | `/api/v1/government` | — | `GovernmentPolicies` | bearer |
| POST | `/api/v1/government/change` | `ChangeGovernmentBody` | `MutationResponse<GovernmentPolicies>` | bearer |

### Diplomacy

| Method | Path | Response | Auth |
|---|---|---|---|
| GET | `/api/v1/diplomacy` | `Diplomacy` | bearer |
| GET | `/api/v1/diplomacy/civs/{id}` | `CivRow` | bearer |

### Empire / Victory

| Method | Path | Response | Auth |
|---|---|---|---|
| GET | `/api/v1/empire/overview` | `EmpireOverview` | bearer |
| GET | `/api/v1/victory` | `Victory` | bearer |

### Notifications

| Method | Path | Response | Auth |
|---|---|---|---|
| GET    | `/api/v1/notifications` | `Notifications` | bearer |
| DELETE | `/api/v1/notifications` | 204 | bearer |
| DELETE | `/api/v1/notifications/{id}` | 204 | bearer |

### Turn

| Method | Path | Body | Response | Auth |
|---|---|---|---|---|
| GET  | `/api/v1/turn-queue` | — | `TurnQueue` | bearer |
| POST | `/api/v1/turn/end` | `{}` | `MutationResponse<EndTurnView>` (or 400 with `unresolved_required_actions`) | bearer |

### Registry

| Method | Path | Response | Auth |
|---|---|---|---|
| GET | `/api/v1/registry` | `Registry` (unit types + buildings) | bearer |

## Generating the spec

```bash
# Default location: book/src/multiplayer/openapi.json
cargo run -p open4x-server --features openapi --bin gen-openapi

# Custom output path:
cargo run -p open4x-server --features openapi --bin gen-openapi -- /tmp/o.json
```

The generator walks up from the current directory until it finds the
workspace `Cargo.toml`, so it works from anywhere inside the tree.

## Out of scope here

The WebSocket surface at `/ws` (Ed25519 auth handshake, `ClientMessage` /
`ServerMessage` JSON frames) is described in
[`protocol.md`](./protocol.md). OpenAPI does not model WebSockets; the
Rust types in
[`open4x-protocol::v1::messages`](https://github.com/bloboss/open-civ-vi/blob/main/open4x-protocol/src/v1/messages.rs)
are the canonical reference.
