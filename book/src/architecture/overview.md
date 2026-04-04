# Architecture Overview

Open Civ VI is structured as a Rust workspace with four crates arranged in a strict dependency hierarchy:

```
libhexgrid      (pure geometry, zero game knowledge)
    |
libciv          (all game state and rules)
    |           |
open4x-cli  open4x-server   (CLI binary / merged server + frontend)
```

> **History**: `open4x-api` (shared wire types) and `open4x-web` (Leptos frontend)
> were merged into `open4x-server` using feature flags (`ssr` for Axum server,
> `csr` for Leptos/WASM client).

## Design Principles

### Single GameState

All game data lives in one `GameState` struct passed by reference to every system. There is no global state, no singletons, and no interior mutability. This makes the engine trivially serializable and safe for concurrent access in a server context.

### Trait-Based Extensibility

Game content -- terrain, features, units, buildings, civilizations -- is defined by implementing Rust traits. Built-in content uses enums wrapping concrete structs (e.g., `BuiltinTerrain`, `BuiltinImprovement`). New content can be added by implementing the same traits and linking at compile time. There is no scripting runtime.

### Modifier Pipeline

Every numeric effect in the game (tech bonuses, policy bonuses, building yields, wonder effects, religious beliefs) is expressed as a `Modifier` struct. Modifiers are collected and resolved at query time -- stored state is never mutated directly by modifiers. This ensures correctness when effects stack, expire, or conflict.

### Semantic Diffs

All `RulesEngine` operations return a `GameStateDiff` -- a `Vec<StateDelta>` describing exactly what changed. This enables:
- **Replay**: reconstruct any game state from an initial state + diff sequence
- **RL observation**: agents observe structured state changes, not raw state
- **UI updates**: clients render only what changed

### Deterministic Simulation

The engine uses seeded RNG (`rand_chacha`) throughout. Given the same seed, the same sequence of actions produces identical results. This is critical for replay, testing, and reproducible AI training.

## Key Conventions

| Convention | Detail |
|-----------|--------|
| **IDs** | ULID-backed newtypes via `define_id!` macro (22 distinct ID types) |
| **Movement costs** | Scaled by 100: `ONE=100`, `TWO=200`, `THREE=300` |
| **Names** | `&'static str` for all built-in content; `String` only at system boundaries |
| **Collections** | `Vec` with linear scan; indexed maps only if profiling demands |
| **Yields** | Never cached on `City` -- always computed via `RulesEngine::compute_yields()` |
| **Edge storage** | Canonical forward-half form `{E, NE, NW}`; backward lookups normalize automatically |
| **Clone** | Structs with `Box<dyn Trait>` fields (e.g., `Leader`) do not derive `Clone` |
