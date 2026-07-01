# Open Civ VI -- Agent Instructions

This file provides guidance to AI coding agents working with this repository.

## Commands

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run a single test by name
cargo test --workspace test_name

# Run tests in a specific crate
cargo test -p libciv
cargo test -p libhexgrid

# Run a specific integration test file
cargo test --test gameplay

# Lint (must be clean with no warnings)
cargo clippy --workspace -- -D warnings

# Run the CLI binary  (package name is `open4x`)
cargo run -p open4x -- new
cargo run -p open4x -- run
cargo run -p open4x -- play

# Build WASM frontend (requires wasm-pack or trunk)
# The WASM target config is in .cargo/config.toml (sets getrandom_backend="wasm_js")

# Regenerate the REST OpenAPI 3 spec from source. Writes
# book/src/multiplayer/openapi.json. The openapi_paths_match_router test
# in open4x-server fails CI if the live Axum router and the
# #[utoipa::path] annotations in open4x-server/src/server/openapi.rs ever
# drift apart.
cargo xtask gen-openapi

# Full pre-commit check: build + test + clippy + openapi-feature pass +
# wasm32 client build + openapi.json freshness diff.
cargo xtask check
# (skip the wasm leg on a fresh machine without the target installed):
cargo xtask check --no-wasm

# Discover every recipe:
cargo xtask --help
```

`cargo xtask` is the workspace recipe runner (pure Rust, lives in `xtask/`,
wired up via an alias in `.cargo/config.toml`). Recipes orchestrate cargo,
trunk, and mdbook; nothing in there knows about game state.

## API documentation

`open4x-protocol::v1::web::*` is the single source of truth for REST wire
shapes. Path metadata (method, params, body, response) lives in
`open4x-server/src/server/openapi.rs` as `#[utoipa::path]` annotations on
no-op `_doc_*` functions; the `ApiDoc` struct at the bottom assembles them
into a `utoipa::openapi::OpenApi`. The `gen-openapi` binary serialises it.

When adding or modifying a REST endpoint:
1. Add/change the route in `open4x-server/src/server/rest/mod.rs::v1_router`.
2. Add/change the handler in `open4x-server/src/server/rest/handlers.rs`.
3. Add/change the `#[utoipa::path]` block in
   `open4x-server/src/server/openapi.rs` and update the `paths(...)` list
   in `ApiDoc` and the `declared_paths()` mirror.
4. Run `cargo test -p open4x-server --features openapi` — both the
   `openapi_paths_match_router` and `declared_paths_match_router` tests
   must pass.
5. Run `cargo run -p open4x-server --features openapi --bin gen-openapi`
   to refresh `book/src/multiplayer/openapi.json`.

The WebSocket surface (`/ws`) is documented in
`book/src/multiplayer/protocol.md`; its types live in
`open4x-protocol::v1::messages`. OpenAPI does not model WebSockets, so the
Rust types are the canonical reference for that path.

## Architecture

### Workspace crates (dependency order)

```
libhexgrid          — pure hex geometry, no game knowledge
libciv              — all game state and rules (world, civ, rules, game, ai modules)
open4x-protocol     — versioned wire types (v1::*); the contract between server and clients
open4x-sdk          — typed HTTP client over the protocol; native + wasm transports
open4x-cli          — CLI binary (clap: `new-game`, `action`, `end-turn`, `view`, `status`, `list`); remote mode wraps the SDK
open4x-server       — native-only Axum server (REST + WS + GameRoom); no Leptos, no cdylib
open4x-client-web   — Leptos CSR + cdylib (wasm32) browser client; consumes the SDK
```

Two additional crates provide the pre-game surface:

```
open4x-accounts     — identity substrate: accounts, sessions, magic-link/OIDC auth (sqlite, feature `persistence`)
open4x-lobby        — landing/login/new-game wizard (Leptos ssr+csr + Axum); consumes open4x-accounts
```

Both are full workspace members. `open4x-lobby`'s single-binary deploy embeds
`open4x-lobby/dist/` (trunk build) and `book/book/` (mdbook build) via
`rust-embed`; a `.gitkeep` keeps each directory present at compile time so the
default workspace build is green without first running trunk/mdbook.

`libhexgrid` must remain zero-knowledge of game concepts. `libciv` contains everything else: world map, civilizations, rules engine, AI, and game orchestration. `open4x-protocol` is the only crate both `open4x-server` and `open4x-client-web` depend on.

### libciv internal structure

```
libciv/src/
  ids.rs          — ULID-backed ID newtypes (define_id! macro)
  yields.rs       — YieldBundle (sparse HashMap), YieldType enum
  enums.rs        — ResourceCategory, UnitDomain, UnitCategory, GreatPersonType, AgeType, PolicyType
  world/          — terrain, feature, edge, improvement, road, tile, wonder
  rules/          — modifier, tech, policy, victory
  civ/            — civilization, city, city_state, diplomacy, district, governor, religion, great_people, era, trade
  game/           — state, board, rules, diff, turn, visibility
  ai/             — deterministic (Agent + HeuristicAgent)
  visualize.rs    — terminal rendering helpers

libciv/tests/
  common/         — shared Scenario setup used by all integration tests
  gameplay.rs     — end-to-end integration tests
  ai_agent.rs     — AI agent integration tests
```

### Key files

| File | Purpose |
|------|---------|
| `libciv/src/ids.rs` | Macro-generated ULID-backed newtype IDs for all entities |
| `libciv/src/yields.rs` | `YieldType` enum + `YieldBundle` (flat struct, not HashMap) |
| `libciv/src/enums.rs` | `ResourceCategory`, `UnitDomain`, `UnitCategory`, `GreatPersonType`, `AgeType`, `PolicyType` |
| `libhexgrid/src/coord.rs` | `HexCoord` with invariant enforcement, arithmetic, `HexDir`, distance, neighbors, ring |
| `libhexgrid/src/types.rs` | `MovementCost`, `Elevation` enum, `Vision` enum, `MovementProfile` enum |
| `libhexgrid/src/board.rs` | `HexTile`/`HexEdge`/`HexBoard` traits, `BoardTopology` enum |
| `libciv/src/world/tile.rs` | `WorldTile` (implements `HexTile`): terrain, hills, feature, resource, improvement, road, rivers, natural_wonder, owner |
| `libciv/src/world/terrain.rs` | `BuiltinTerrain` plain enum (Grassland/Plains/Desert/Tundra/Snow/Coast/Ocean/Mountain) |
| `libciv/src/world/feature.rs` | `BuiltinFeature` plain enum (Forest/Rainforest/Marsh/Floodplain/Reef/Ice/VolcanicSoil/Oasis) |
| `libciv/src/world/resource.rs` | `BuiltinResource` plain enum (8 bonus, 8 luxury, 7 strategic); `reveal_tech()` for gating |
| `libciv/src/world/edge.rs` | `EdgeFeatureDef` trait + `WorldEdge` (River/Canal/MountainPass); stores `(HexCoord, HexDir)` |
| `libciv/src/world/improvement.rs` | `BuiltinImprovement` (Farm/Mine/LumberMill/TradingPost/Fort/Airstrip/MissileSilo) + `ImprovementRequirements` |
| `libciv/src/world/road.rs` | `RoadDef` trait + `BuiltinRoad` (Ancient/Medieval/Industrial/Railroad); `as_def()` wrapper |
| `libciv/src/world/wonder.rs` | `NaturalWonder` trait + `BuiltinNaturalWonder` (22 wonders); `as_def()` wrapper |
| `libciv/src/rules/modifier.rs` | `Modifier`, `EffectType`, `TargetSelector`, `StackingRule`, `ModifierSource`, `resolve_modifiers()` |
| `libciv/src/rules/effect.rs` | `OneShotEffect` enum + `CascadeClass`; replaces old `Unlock` enum |
| `libciv/src/rules/tech.rs` | `TechNode`/`CivicNode`, `TechTree`/`CivicTree`; nodes carry `Vec<OneShotEffect>` |
| `libciv/src/rules/tech_tree_def.rs` | Full tech tree definition (Pottery, Mining, ...) with effects and eureka data |
| `libciv/src/rules/civic_tree_def.rs` | Full civic tree definition with inspiration data |
| `libciv/src/rules/policy.rs` | `Policy`, `Government`, `PolicySlots` |
| `libciv/src/rules/victory.rs` | `BuiltinVictoryCondition` enum (Score, Culture, Domination, Science, Diplomatic, Religious) |
| `libciv/src/civ/civilization.rs` | `Civilization`, `TechProgress`, `CivicProgress`, `Leader` (with `BuiltinAgenda` enum, no trait objects) |
| `libciv/src/civ/city.rs` | `City`, `CityKind`, `CityOwnership`, `WallLevel`, `ProductionItem`; `worked_tiles` + `locked_tiles` |
| `libciv/src/civ/city_state.rs` | `CityStateType`, `CityStateBonus` trait, `CityStateData` |
| `libciv/src/civ/diplomacy.rs` | `DiplomaticRelation`, `DiplomaticStatus`, `GrievanceRecord`, `GrievanceVisibility` |
| `libciv/src/civ/district.rs` | `DistrictDef`/`BuildingDef` traits, `AdjacencyContext`, `PlacedDistrict` |
| `libciv/src/civ/religion.rs` | `Religion`, `Belief` trait, `BeliefContext`, spread/conversion, theological combat |
| `libciv/src/civ/trade.rs` | `TradeRoute`; `is_international(&[City])` compares city owners; `compute_route_yields(bool)` returns gold yields |
| `libciv/src/game/state.rs` | `GameState` (single source of truth), `IdGenerator`, `UnitTypeDef`, `BuildingDef` registries, `effect_queue` |
| `libciv/src/game/board.rs` | `WorldBoard`: `HexBoard` impl, Dijkstra pathfinding (road override active), LOS, edge canonicalization |
| `libciv/src/game/rules/mod.rs` | `RulesEngine` trait (38+ methods) + `DefaultRulesEngine` |
| `libciv/src/game/diff.rs` | `StateDelta` enum (70+ variants), `GameStateDiff`, `AttackType` |
| `libciv/src/game/turn.rs` | `TurnEngine`: calls `advance_turn`, returns `GameStateDiff` |
| `libciv/src/game/apply_delta.rs` | `apply_delta()` / `apply_diff()` for state reconstruction |
| `libciv/src/game/save_load.rs` | `save_game()` / `load_game()` (serde feature-gated) |
| `libciv/src/game/replay.rs` | `ReplayRecorder` / `ReplayViewer` (serde feature-gated) |
| `libciv/src/game/production_helpers.rs` | `available_unit_defs()`, `resolve_unit_replacement()`, tech gating |
| `libciv/src/rl/env.rs` | `CivEnv` gym-like RL training harness |
| `libciv/src/game/visibility.rs` | `recalculate_visibility()` free function |
| `libciv/src/ai/deterministic.rs` | `Agent` trait + `HeuristicAgent` (exploration/production heuristic) |
| `libciv/tests/common/` | Shared `Scenario` setup used by all integration tests |
| `libciv/tests/gameplay.rs` | End-to-end integration tests |
| `open4x-cli/src/main.rs` | CLI entry point (non-REPL + legacy interactive) |
| `open4x-cli/src/remote/client.rs` | Thin shim over `open4x_sdk::native::NativeBlockingClient` |
| `open4x-protocol/src/v1/{coord,enums,ids,messages,profile,reports,view,web}.rs` | All wire types under the `v1::*` namespace — the single source of truth for the client/server contract |
| `open4x-protocol/tests/wire_schema.rs` | JSON-snapshot regression tests for each top-level wire type |
| `open4x-sdk/src/transport.rs` | `Transport` trait + `Method` enum — the SDK's transport abstraction |
| `open4x-sdk/src/error.rs` | `ApiError` + helpers for decoding server `{error, message}` envelopes |
| `open4x-sdk/src/native.rs` | `NativeBlockingClient` + `NativeAsyncClient` (feature `native-blocking` / `native-async`) |
| `open4x-sdk/src/wasm.rs` | `WasmClient` over `web_sys::fetch` (feature `wasm`, wasm32 only) |
| `open4x-sdk/src/endpoints/*` | One module per `/api/v1/*` resource (armies, cities, civics, combat, diplomacy, empire, games, government, health, map, notifications, player_state, registry, tech, turn, units, victory, world) |
| `open4x-server/src/main.rs` | Axum server entry point; `OPEN4X_STATIC_DIR` defaults to `./open4x-client-web/dist` |
| `open4x-server/src/server/rest/` | REST handlers; `v1_router()` is the shared route table for `main.rs` + integration tests |
| `open4x-server/src/server/web_projection.rs` | Pure functions projecting `GameView` into per-endpoint wire slices |
| `open4x-server/tests/rest_api.rs` | In-process integration tests for every `/api/v1/*` route via `tower::ServiceExt::oneshot` |
| `open4x-client-web/src/lib.rs` | `wasm_bindgen(start)` entry; mounts the Leptos app (module decls are `cfg(target_arch = "wasm32")`-gated so workspace native builds stay green) |
| `open4x-client-web/src/pages/` | Leptos routes (e.g. `RestGamePage`, `HomePage`, `MapConfigPage`) |
| `open4x-client-web/src/components/` | Reusable UI components (HexMap, HUD, drawers); SDK adapter helper in `pages/rest_game.rs` |
| `open4x-client-web/src/tabs/` | Tab views (Map, Science, Culture, City, etc.) |
| `open4x-client-web/index.html` | Trunk entry; styling lives here pending CSS split |

### Key design decisions

- **Single GameState** -- one struct passed by reference to all systems; no global state. All collections are `Vec` with linear-scan lookups (indexed maps only if profiling demands).
- **Trait-based extensibility** -- game content implements traits; built-in variants are enums wrapping concrete structs (e.g. `BuiltinTerrain`). Extensions link at compile time; no scripting runtime.
- **Modifier pipeline** -- every numeric effect (tech, policy, building, wonder, belief) is a `Modifier` struct. Modifiers are collected and applied at query time; stored state is never mutated directly. `resolve_modifiers()` groups by stacking rule: `Additive` sums, `Max` keeps highest, `Replace` keeps last.
- **Semantic diffs** -- all `RulesEngine` operations return `GameStateDiff` (a `Vec<StateDelta>`) to support replay and RL observation.
- **Edge canonicalization** -- edges stored as `(HexCoord, HexDir)` with forward-half canonical form (`{E, NE, NW}`). Backward-half lookups (`{W, SW, SE}`) normalize to the adjacent tile with the opposite direction. Use `WorldBoard::set_edge()` for automatic canonicalization.
- **Movement costs scaled by 100** -- `ONE=100`, `TWO=200`, `THREE=300`. Road cost overrides tile cost in Dijkstra when `tile.road.is_some()`.
- **Trait objects replaced with enums** -- `VictoryCondition` → `BuiltinVictoryCondition` enum, `Agenda` → `BuiltinAgenda` enum. `Leader.abilities` was removed (was always empty). This enables serde serialization.
- **`&'static str` for built-in names** -- compile-time game content never uses `String`. Only external/user data at system boundaries uses `String`.
- **Yields/amenities/housing never stored on City** -- computed via `RulesEngine` queries so modifiers apply correctly. The `yields` field on `City` is a cache only.
- **CityState as City** -- city-states are stored as `City` with `kind = CityKind::CityState(CityStateData)`. Access via `GameState::city_state_by_civ(CivId)`.

### Conventions

- All entity IDs are ULID-backed newtypes defined via `define_id!` macro in `libciv/src/ids.rs`
- `MovementCost` uses integer math scaled by 100 (ONE = 100, TWO = 200, THREE = 300)
- `BoardTopology` variants: `Flat`, `CylindricalEW` (wraps east-west), `Toroidal`
- Rust edition 2024; workspace resolver 2
- Built-in terrain, features, resources, and improvements use **plain enums** with direct method dispatch -- no trait objects, no inner zero-sized structs
- `BuiltinRoad`, `BuiltinEdgeFeature`, `BuiltinNaturalWonder` still use the wrapper-with-`as_def()` pattern (trait objects needed for extensibility)
- `&'static str` is appropriate for all built-in content names -- compile-time content, not user data
- Structs containing `Box<dyn Trait>` fields (`Leader`, `Civilization`) do **not** derive `Clone`
- `YieldBundle` is a flat struct (named `i32` fields), not HashMap-backed

### Design constraints

- `GameState` is passed by reference to all systems; no global state
- `libhexgrid` must remain zero-knowledge of game concepts
- All `RulesEngine` operations return `GameStateDiff` (supporting replay and RL observation)
- Collections are `Vec` with linear-scan lookups; indexed maps only if profiling demands

### VCS

Use **`but`** (GitButler) for ongoing work — see [`agents/skills/gitbutler/SKILL.md`](./agents/skills/gitbutler/SKILL.md). `jj` use is suspended for the duration of the crate-split / GitButler-workflow window; revisit after the migration settles. Never run `git` write commands (`git add`, `git commit`, `git push`, `git stash`, `git checkout`, `git merge`, `git rebase`); translate to the matching `but` command. Read-only git inspection (`git status`, `git log`, `git diff --stat`) is fine.

Commit style: **Conventional Commits with a MANDATORY scope** —
`type(scope): subject`. Allowed `type` values are listed in
[`TAGS.md`](./TAGS.md) and allowed `scope` values in
[`SCOPES.md`](./SCOPES.md); both are machine-readable and enforced by
the `commit-msg` hook (`.githooks/commit-msg`, run via pre-commit). A
scopeless message (`fix: …`) is rejected. Breaking changes use
`type(scope)!: …`.

### Dev environment & pre-commit hooks

Lint/format tooling is orchestrated by [pre-commit](https://pre-commit.com),
with the Python side (pre-commit, codespell, black) pinned in this repo's
**uv** project (`pyproject.toml` / `uv.lock`). One-time setup per clone:

```bash
uv sync                    # install the pinned Python tooling into .venv
uv run pre-commit install  # wire pre-commit, commit-msg, and pre-push hooks
```

What runs when (see [`.pre-commit-config.yaml`](./.pre-commit-config.yaml)):

| stage | hooks |
|-------|-------|
| commit | `rustfmt --check` (changed `.rs` only), `codespell`, `black` (Python) |
| commit-msg | conventional-commit `type(scope): subject` validation |
| push | `cargo clippy --workspace --all-targets` (compiles the tree) |

rustfmt/codespell scan only the files in the commit, so they were
introduced without a mass-reformat of the legacy tree; run
`cargo fmt --all` yourself if you want to normalise a crate. `black`
targets Python helper scripts (edition 2024 Rust is formatted by
rustfmt).

### WASM frontend

The browser client is `open4x-client-web` (Leptos CSR, cdylib). It builds for `wasm32-unknown-unknown` via trunk and depends on `open4x-sdk` (`features = ["wasm"]`) + `open4x-protocol`. The native `open4x-server` binary serves `/api/v1/*` REST + `/ws` and falls back to the trunk-built `dist/` for the SPA (default `OPEN4X_STATIC_DIR` is `./open4x-client-web/dist`). `getrandom`'s `wasm_js` feature is pinned in `open4x-sdk`'s wasm32 dependency block so the transitive `ulid → rand → getrandom` chain compiles for wasm; no workspace-level rustflag is required.

## Agent Skills

The [`agents/`](./agents/) directory contains reusable skill guides for common development patterns:

| Skill | When to use |
|-------|-------------|
| [add-rules-engine-method](./agents/add-rules-engine-method.md) | Adding a new game action to the `RulesEngine` trait |
| [write-integration-test](./agents/write-integration-test.md) | Writing integration tests using the `Scenario` pattern |
| [add-game-content](./agents/add-game-content.md) | Adding civilizations, units, buildings, improvements |
| [implement-roadmap-feature](./agents/implement-roadmap-feature.md) | Picking up a feature from the implementation roadmap |
| [advance-turn-phase](./agents/advance-turn-phase.md) | Adding per-turn processing to `advance_turn` |
| [make-todo](./agents/make-todo.md) | Adding a tracked TODO (code comment + `todo.md` entry) |
| [modify-todo](./agents/modify-todo.md) | Updating an existing TODO (description, priority, location) |
| [delete-todo](./agents/delete-todo.md) | Removing a completed or obsolete TODO |

## TODO Management

Code TODOs are tracked in two places simultaneously:
1. **Code comment**: `// TODO(<TAG>): description` at the relevant source location
2. **Global list**: `book/src/roadmap/todo.md` with file:line, description, and priority

Always use the `make-todo`, `modify-todo`, and `delete-todo` skills to keep both in sync. Never add ad-hoc TODO comments without a corresponding `todo.md` entry.

## Documentation

Full documentation including architecture details, all game systems, engine design, and multiplayer protocol is in the [mdBook](./book/). The [Implementation Status](./book/src/roadmap/status.md) shows current feature coverage. `cargo doc -p libciv` generates API documentation.
