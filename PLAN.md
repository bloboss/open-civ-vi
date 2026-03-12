# Terrain Feature Completion Plan

## Overview

Core terrain feature systems are defined (terrain, features, resources, improvements,
roads, edges) but several gameplay-critical wiring points are missing. This plan
completes the terrain feature layer in order of dependency.

---

## Part 1 — Terrain Combat Modifiers

**Goal:** Hills and terrain features grant defense bonuses in combat, matching Civ VI rules.

**Acceptance Criteria:**
- `score_tile_defense(tile: &WorldTile) -> i32` in `libciv/src/world/tile.rs`
  returns the flat combat-strength defense bonus for a defending unit on that tile:
  - Grassland/Plains/Desert/Tundra/Snow flat = 0
  - Hills (Elevation::Level(1)) = +3
  - Forest = +3
  - Rainforest = +3
  - Marsh = -2 (attacker-favored terrain)
  - Combined: Forest on Hills = +6 (additive)
- `attack()` in `libciv/src/game/rules.rs` applies `score_tile_defense` to the
  **defender's tile** by adding it to the defender's effective combat strength before
  the damage formula. Attacker tile has no bonus.
- At least 3 new tests in `libciv/tests/combat_terrain.rs` covering:
  - Unit on hills takes less damage than unit on flat ground
  - Unit in forest takes less damage
  - Unit in marsh takes more damage

**Rust changes:**
- `libciv/src/world/tile.rs`: add `pub fn terrain_defense_bonus(&self) -> i32`
- `libciv/src/game/rules.rs`: in `attack()`, compute `def_cs + target_tile.terrain_defense_bonus()`

---

## Part 2 — Resource Concealment by Features

**Goal:** Resources hidden under Forest/Rainforest are not visible to civilizations
that have not researched the reveal tech AND have not cleared the feature.

**Current state:** `FeatureDef::conceals_resources()` returns `true` for
Forest and Rainforest but is never checked at runtime.

**Acceptance Criteria:**
- `tile_yields_gated()` in `rules.rs` already suppresses yields when reveal tech is
  missing. Extend it to also suppress when `tile.feature.as_ref().map_or(false, |f| f.as_def().conceals_resources())` is true, **unless** the civ owns the tile and the
  feature has been cleared.
- A new helper `fn feature_conceals(&self, tile: &WorldTile) -> bool` on
  `DefaultRulesEngine` performs the check.
- The concealment is purely a **yield suppression** (like tech-gating) — the resource
  still exists on the tile, it just contributes no yields and does not appear in the
  civ's resource inventory.
- 2 new tests in `libciv/tests/terrain_features.rs` verifying:
  - Deer under Forest: no yield without reveal tech
  - Deer under Forest on owned tile with feature cleared (feature = None): yield appears

**Rust changes:**
- `libciv/src/game/rules.rs`: extend `tile_yields_gated()` with feature concealment check

---

## Part 3 — Cliff Edge LOS Blocking

**Goal:** `WorldEdge::Cliff` blocks line of sight between the two tiles it separates.

**Current state:** `WorldEdge::Cliff` has `blocks_los: true` but `has_los()` in
`libciv/src/game/board.rs` only checks elevation; it ignores edge features.

**Acceptance Criteria:**
- `has_los(from, to, board)` in `board.rs` additionally checks every edge crossed by
  the ray. If any crossed edge is a `WorldEdge::Cliff`, return `false`.
- "Crossed edge" = the canonical edge `(coord, dir)` for each hex boundary the ray
  passes through (i.e., each step between adjacent hex coords along the LOS path).
- 2 new tests in `libciv/tests/los_cliff.rs`:
  - LOS blocked when a cliff edge lies between the two tiles
  - LOS unblocked when cliff is on a non-crossed edge

**Rust changes:**
- `libciv/src/game/board.rs`: in `has_los()`, iterate steps of the ray and call
  `get_edge(coord, dir)` for each; if `WorldEdge::Cliff` is found, return `false`

---

## Part 4 — Natural Wonders

**Goal:** Define 5 canonical natural wonders as concrete `NaturalWonderDef`
implementors, place them on `WorldTile`, and apply their yield bonuses.

**Current state:** `libciv/src/world/wonder.rs` has a trait stub (`appeal_bonus()`)
with zero concrete implementations. `NaturalWonderId` exists in `ids.rs`.

**Acceptance Criteria:**

### 4.1 — Trait extension
Extend `NaturalWonderDef` trait (in `wonder.rs`) with:
```rust
pub trait NaturalWonderDef: Send + Sync {
    fn id(&self) -> NaturalWonderId;
    fn name(&self) -> &'static str;
    fn appeal_bonus(&self) -> i32;
    fn yield_bonus(&self) -> YieldBundle;  // NEW: yields granted to tile
    fn movement_cost(&self) -> MovementCost;  // NEW: override tile movement cost
    fn impassable(&self) -> bool { false }  // NEW: can units enter?
}
```

### 4.2 — 5 built-in wonders
Add `BuiltinNaturalWonder` enum in `wonder.rs` with variants:
- `Krakatoa` — appeal +4, +4 Production, impassable
- `GrandMesa` — appeal +4, +2 Production, +1 Food, passable (hills movement cost)
- `CliffsOfDover` — appeal +4, +2 Culture, +1 Gold, coastal, impassable
- `UluruAyersRock` — appeal +4, +3 Faith, impassable
- `GalapagosIslands` — appeal +4, +2 Science, passable (coast movement cost)

### 4.3 — WorldTile extension
Add `natural_wonder: Option<BuiltinNaturalWonder>` to `WorldTile`.
- When present: `WorldTile::total_yields()` adds `wonder.yield_bonus()` on top of
  terrain yields (no feature/improvement bonus applies to wonder tiles)
- `terrain_defense_bonus()` returns 0 for wonder tiles

### 4.4 — AdjacencyContext integration
`AdjacencyContext::adjacent_natural_wonders: Vec<NaturalWonderId>` already exists.
Populate it in any adjacency calculation calls (stub with empty vec is acceptable for
now; real population when districts are built is Phase 5+).

**Rust changes:**
- `libciv/src/world/wonder.rs`: extend trait, add `BuiltinNaturalWonder` enum + impls
- `libciv/src/world/tile.rs`: add `natural_wonder` field, update `total_yields()`
- 3 tests in `libciv/tests/natural_wonders.rs` covering yield bonus and appeal

---

## Part 5 — Terrain–Improvement Validity

**Goal:** Enforce that improvements may only be placed on valid terrain/feature
combinations (preventing Farm on Ocean, Mine on Grassland without Hills, etc.).

**Acceptance Criteria:**

### 5.1 — Validity trait method
Add to `TileImprovement` trait in `improvement.rs`:
```rust
fn valid_on(&self, terrain: &dyn TerrainDef, feature: Option<&dyn FeatureDef>) -> bool;
```

Implement for each `BuiltinImprovement`:
- `Farm` — valid on: Grassland, Plains, Desert (Floodplain/Oasis feature), Tundra; NOT on Snow, Coast, Ocean, Mountain
- `Mine` — valid on: Hills (elevation >= Level(1)), resource tiles with strategic/bonus resource
- `LumberMill` — valid on: tiles with Forest feature only
- `TradingPost` — valid on: any passable land tile
- `Fort` — valid on: any passable land tile (not Ocean/Coast)
- `Airstrip` — valid on: flat land tiles only (not Hills, not Mountain)
- `MissileSilo` — valid on: flat land tiles only

### 5.2 — Enforcement in RulesEngine
Add `place_improvement()` to `RulesEngine` trait and implement in `DefaultRulesEngine`:
```rust
fn place_improvement(&mut self, state: &mut GameState, tile: HexCoord,
                     improvement: BuiltinImprovement) -> Result<GameStateDiff, RulesError>;
```
- Validates `valid_on()` → returns `RulesError::InvalidImprovement` on failure
- Sets `tile.improvement = Some(improvement)`, `tile.improvement_pillaged = false`
- Returns `StateDelta::ImprovementPlaced { coord, improvement }`

Add `StateDelta::ImprovementPlaced { coord: HexCoord, improvement: BuiltinImprovement }`.

- 3 new tests in `libciv/tests/improvements.rs`

---

## Ordering and Dependencies

```
Part 1 (combat modifiers)       — independent, can be done first
Part 2 (resource concealment)   — independent
Part 3 (cliff LOS)              — independent
Part 4 (natural wonders)        — depends on tile.rs (coordinate with Part 1 edit)
Part 5 (improvement validity)   — depends on Part 4 (wonder tile guard)
```

Recommended implementation order: **1 → 2 → 3 → 4 → 5** (each part is self-contained
enough to be a single commit).

---

## What This Plan Explicitly Excludes

- Builder units / construction queue (large separate feature, Phase 5+)
- Road placement / upgrade system (Phase 5+)
- District adjacency scoring (Phase 5+)
- Appeal rendering in TUI (Phase 5+)
- Natural wonder discovery events (Phase 5+)
