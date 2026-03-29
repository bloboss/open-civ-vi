# Performance Optimization

Phase 6, item 19. Indexed lookups where profiling identifies hot paths.

## Current State

- All entity collections are `Vec` with linear-scan lookups: `state.units.iter().find(|u| u.id == id)`.
- Design decision (documented in AGENTS.md): "indexed maps only if profiling demands."
- No profiling has been done yet.
- Key lookup patterns in `advance_turn` and `RulesEngine` methods:
  - `state.civ(civ_id)` -- linear scan of `civilizations` (typically 2-8 entries)
  - `state.city(city_id)` -- linear scan of `cities` (typically 5-50 entries)
  - `state.units.iter().find(|u| u.id == unit_id)` -- linear scan of `units` (typically 10-100 entries)
  - `state.cities.iter().filter(|c| c.owner == civ_id)` -- full scan per civ
  - `state.placed_districts.iter().filter(|d| d.city_id == city_id)` -- full scan per city

## Design

### Profiling-first approach

Do not optimize blindly. Profile with realistic game states (50+ cities, 100+ units, turn 200+) to identify actual bottlenecks.

### Likely hot paths

Based on code analysis, the following patterns appear in inner loops:

| Pattern | Frequency | Current Cost | Optimization |
|---------|-----------|-------------|-------------|
| `units.find(id)` | Every combat, movement, turn | O(n) | `HashMap<UnitId, usize>` index |
| `cities.filter(owner)` | Per civ per turn (yields, tourism, loyalty) | O(n) | `HashMap<CivId, Vec<CityId>>` |
| `placed_districts.filter(city_id)` | Per city per turn (yields) | O(n) | Store on `City` directly |
| Modifier resolution | Per yield calculation | O(m) per modifier | Pre-compute per-civ modifier cache, invalidate on policy/tech change |
| Pathfinding (Dijkstra) | Per unit movement | O(V log V) | Pre-compute movement maps per turn, cache results |

### Index strategy

Add secondary index maps that are maintained alongside the primary `Vec` storage:

```rust
pub struct GameState {
    // Primary storage (authoritative):
    pub units: Vec<BasicUnit>,
    // Secondary index (derived, rebuilt on mutation):
    unit_index: HashMap<UnitId, usize>,
}
```

Indexes are rebuilt when entities are added/removed. Lookups become O(1) instead of O(n).

## Implementation Plan

### Step 1: Add benchmarks

Create `libciv/benches/` with criterion benchmarks for:
- `advance_turn` with various game sizes (10, 50, 200 cities)
- `compute_yields` per city
- `move_unit` with pathfinding
- `attack` with modifier resolution
- Full game simulation (100 turns)

### Step 2: Profile

Run benchmarks with `cargo bench` and flamegraph profiling. Identify the top 5 hotspots.

### Step 3: Add indexes for confirmed bottlenecks

Only add indexes for lookups that profiling shows are significant. Likely candidates:

1. **Unit index**: `HashMap<UnitId, usize>` -- O(1) unit lookup.
2. **City-by-owner index**: `HashMap<CivId, Vec<CityId>>` -- O(1) per-civ city list.
3. **District-by-city**: Already `city.districts: Vec<BuiltinDistrict>`, but `PlacedDistrict` lookups need indexing.

### Step 4: Modifier caching

Pre-compute the active modifier set per civ at the start of each turn. Invalidate only when policies, techs, or governments change. This avoids re-collecting modifiers for every yield calculation.

### Step 5: Pathfinding optimization

Cache shortest-path trees per unit at the start of a turn. When a unit hasn't moved, subsequent `move_unit` calls can reuse the cached tree.

### Step 6: Verify correctness

All optimizations must pass the existing 200+ integration tests. Add benchmark regression tests to CI.

## Complexity

Medium per optimization. The key challenge is maintaining index consistency across all state mutations (every place that pushes/removes from `units`, `cities`, etc. must update the index).

## Dependencies

- No hard dependencies. Can be done incrementally at any time.
- More impactful after content expansion (Phase 5) increases entity counts.
