# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
civsim        — CLI binary (clap: `new` and `run` subcommands)
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

### Key design decisions

- **Single GameState** — one struct passed by reference to all systems; no global state. All collections are `Vec` with linear-scan lookups (indexed maps only if profiling demands).
- **Trait-based extensibility** — game content implements traits; built-in variants are enums wrapping concrete structs (e.g. `BuiltinTerrain`). Extensions link at compile time; no scripting runtime.
- **Modifier pipeline** — every numeric effect (tech, policy, building, wonder, belief) is a `Modifier` struct. Modifiers are collected and applied at query time; stored state is never mutated directly. `resolve_modifiers()` groups by stacking rule: `Additive` sums, `Max` keeps highest, `Replace` keeps last.
- **Semantic diffs** — all `RulesEngine` operations return `GameStateDiff` (a `Vec<StateDelta>`) to support replay and RL observation.
- **Edge canonicalization** — edges stored as `(HexCoord, HexDir)` with forward-half canonical form (`{E, NE, NW}`). Backward-half lookups (`{W, SW, SE}`) normalize to the adjacent tile with the opposite direction. Use `WorldBoard::set_edge()` for automatic canonicalization.
- **Movement costs scaled by 100** — `ONE=100`, `TWO=200`, `THREE=300`. Road cost overrides tile cost in Dijkstra when `tile.road.is_some()`.
- **`Box<dyn Trait>` prevents Clone** — `Leader` and `Civilization` contain trait objects and do not derive `Clone`. Document this on any new structs with `Box<dyn>` fields.
- **`&'static str` for built-in names** — compile-time game content never uses `String`. Only external/user data at system boundaries uses `String`.
- **Yields/amenities/housing never stored on City** — computed via `RulesEngine` queries so modifiers apply correctly. The `yields` field on `City` is a cache only.
- **CityState as City** — city-states are stored as `City` with `kind = CityKind::CityState(CityStateData)`. Access via `GameState::city_state_by_civ(CivId)`.

### VCS

Use **jj** (Jujutsu), not git. Commit style: conventional commits — `infra:`, `impl:`, `fix:`, `tests:`, `docs:`.

### WASM frontend

`open4x-server` with the `csr` feature compiles to `wasm32-unknown-unknown` via Leptos. The `.cargo/config.toml` sets `rustflags = ["--cfg", "getrandom_backend=\"wasm_js\""]` for all wasm targets — required for `getrandom 0.3` transitive deps to agree on the WASM backend. The `ssr` feature builds the native Axum server binary.
