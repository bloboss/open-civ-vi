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
```

## Architecture

### Workspace crates (dependency order)

```
libhexgrid    — pure hex geometry, no game knowledge
libciv        — all game state and rules (world, civ, rules, game, ai modules)
open4x-cli    — CLI binary (clap: `new-game`, `action`, `end-turn`, `view`, `status`, `list`)
open4x-server — merged server + frontend (feature flags: `ssr` for Axum server, `csr` for Leptos/WASM)
```

`libhexgrid` must remain zero-knowledge of game concepts. `libciv` contains everything else: world map, civilizations, rules engine, AI, and game orchestration.

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

Use **jj** (Jujutsu), not git. Commit style: conventional commits -- `infra:`, `impl:`, `fix:`, `tests:`, `docs:`.

### WASM frontend

`open4x-server` with the `csr` feature compiles to `wasm32-unknown-unknown` via Leptos. The `.cargo/config.toml` sets `rustflags = ["--cfg", "getrandom_backend=\"wasm_js\""]` for all wasm targets -- required for `getrandom 0.3` transitive deps to agree on the WASM backend. The `ssr` feature builds the native Axum server binary.

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
