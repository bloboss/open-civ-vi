# Terrain Feature Completion Plan

## Overview

Core terrain feature systems are defined (terrain, features, resources, improvements,
roads, edges) but several gameplay-critical wiring points are missing. This plan
completes the improvement/terrain system

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

## What This Plan Explicitly Excludes

- Builder units / construction queue (large separate feature, Phase 5+)
- Road placement / upgrade system (Phase 5+)
- District adjacency scoring (Phase 5+)
- Appeal rendering in TUI (Phase 5+)
- Natural wonder discovery events (Phase 5+)
