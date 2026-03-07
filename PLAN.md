# Phase 2 Plan

Phase 2 turns the Phase 1 skeleton into a functional rules core. The acceptance
criteria are the five `#[ignore]`d tests plus three `todo!()` stubs in
`DefaultRulesEngine`. Each commit is self-contained, tests-green before merging.

---

## Commits

### 1. `fix(rules): implement modifier stacking in resolve_modifiers()`

**File:** `libciv/src/rules/modifier.rs`

`resolve_modifiers()` currently returns all effects unprocessed. Replace the body
with a proper grouped reduction:

1. Partition the input slice into groups keyed on `(EffectType discriminant, YieldType
   if applicable, StackingRule)`.
2. For each group apply the stacking rule:
   - `Additive` → sum all amounts; emit one `EffectType` with the total.
   - `Max` → keep the largest amount; emit one `EffectType`.
   - `Replace` → keep the last element in the slice; emit one `EffectType`.
3. Return the reduced `Vec<EffectType>`.

Remove the three `#[ignore]` attributes from:
- `test_modifier_stacking_additive`
- `test_modifier_stacking_max`
- `test_modifier_stacking_replace`

All three must pass.

---

### 2. `fix(game/board): implement LOS elevation blocking`

**File:** `libciv/src/game/board.rs`

`has_los(from, to)` uses hex ray interpolation but does not correctly apply
elevation. The fix:

1. After computing each interpolated midpoint tile coordinate, look up the tile.
2. If the tile's `elevation()` is `Elevation::High`, LOS is blocked *unless* both
   the `from` tile and the `to` tile are also `Elevation::High` (peak-to-peak sightlines
   are permitted).
3. The test (`test_los_blocked_by_high`) needs the midpoint tile set to a terrain
   with `elevation() = Elevation::High` (mountain). Update the test setup: assign a
   `Mountain` tile (or any terrain returning `Elevation::High`) at the blocking coord.

Remove `#[ignore]` from `test_los_blocked_by_high`. Must pass.

---

### 3. `fix(game/board): apply road movement cost in Dijkstra`

**File:** `libciv/src/game/board.rs`

In `find_path()`, the tile movement cost is currently read as
`tile.terrain.movement_cost()`. Change the lookup to:

```rust
// Can this use unwrap_or_else?
// Prefer match statements
let tile_cost = if tile.road.is_some() {
    tile.road.as_ref().unwrap().movement_cost()
} else {
    tile.movement_cost()  // HexTile impl (terrain + feature)
};
```

Add edge crossing cost on top as before. The road cost should always be lower than
base terrain, so a path through roaded tiles will be preferred.

Update the test (`test_dijkstra_prefers_roads`): set up a 1-wide corridor with road
tiles alongside an equivalent unroaded path; assert that `find_path` returns the
roaded route.

Remove `#[ignore]` from `test_dijkstra_prefers_roads`. Must pass.

---

### 4. `impl(game/rules): implement DefaultRulesEngine::move_unit()`

**File:** `libciv/src/game/rules.rs`

Replace the `todo!()` in `move_unit()`:

1. Find the unit in `state.civilizations` (scan all civs) or add a `units` vec to
   `GameState` — if units are only on `BasicUnit` embedded in `Civilization`, decide
   the lookup strategy here.
2. Call `state.board.find_path(unit.coord(), to, unit.movement_left())`.
3. If `None`, return `Err(RulesError::DestinationImpassable)` or
   `Err(RulesError::InsufficientMovement)` as appropriate.
4. Compute path cost, deduct from `movement_left`.
5. Update `unit.coord` to `to`.
6. Return `Ok(GameStateDiff)` containing `StateDelta::UnitMoved { unit, from, to }`.

Write at least three integration tests:
1. spawn a `BasicUnit`, call `move_unit`, assert the diff contains the expected 
   `StateDelta` and that the unit's position has changed.
2. spawn a `BasicUnit`, call `move_unit` to an unreachable position, assert a
   failure and that the unit has not moved and that the game state is unaffected
3. spawn a `BasicUnit`, call `move_unit` to a tile outside its movement range,
   assert that the unit has moved the maximum distance along the path, that the
   call to `move_unit` provided an `Err(RulesError::InsufficientMovement` and
   that the game state reflects the unit's movement correctly

---

### 5. `impl(game/rules): implement DefaultRulesEngine::compute_yields()`

**File:** `libciv/src/game/rules.rs`

Replace the `todo!()` in `compute_yields()`:

1. Find all cities owned by `civ_id`.
2. For each city, sum `tile.total_yields()` for tiles worked by that city (for now,
   the 6 adjacent tiles + city center).
3. For each `BuildingId` in `city.buildings`, look up the building definition and add
   `building.yields()`.
4. Collect all active `Modifier`s for the civ (from policies, governments, leader
   abilities). Call `resolve_modifiers()` and apply flat then percent effects to the
   accumulated `YieldBundle`.
5. Return the total.

Write at least one test: build a `GameState` with one city on Grassland, assert
`compute_yields` returns at least 2 Food (Grassland base).

---

### 6. `impl(game/rules): implement DefaultRulesEngine::advance_turn()`

**File:** `libciv/src/game/rules.rs`

Replace the `todo!()` in `advance_turn()`. Process in order:

1. **Gold** — for each civ add `compute_yields(civ).gold` to `civ.gold`, push
   `StateDelta::GoldChanged`.
2. **Science** — add science yield to `tech_in_progress.progress`; if
   `progress >= tech_node.cost`, mark tech complete, push `StateDelta::TechResearched`,
   add to `civ.researched_techs`, clear `tech_in_progress`.
3. **Culture** — same pattern for `civic_in_progress`.
4. **Food** — for each city, add food yield to `city.food_stored`; if
   `food_stored >= food_to_grow`, increment population, reset `food_stored`,
   recalculate `food_to_grow = 15 + 6 * (population - 1)`.
5. **Production** — add production yield to `city.production_stored`; if
   `production_stored >= item_cost`, complete `current_production`, push appropriate
   `StateDelta`, clear queue.
6. **Turn counter** — increment `state.turn`, push `StateDelta::TurnAdvanced`.

Return the accumulated `GameStateDiff`.

Write at least one test: one city on Grassland with a known food yield; after N
calls to `advance_turn`, assert population has grown.

### 7. `impl: simple visualization tool`

**File:** `libciv/src/visualize.rs`

Implement a minimal, terminal based visualization tool so that a human might be
able to see and debug erroneous states.

1. **Basic Visualization** - build a Visualize trait tool that creates a square
   (nxn) segment of ascii characters that reflect the given hex state and a
   trait method that gives the value of n
2. **Storing Multiple Hexes** - build a `struct Visualizer<T: visualize>` with
   a doubly nested array of T. There should be one method in its impl that
   creates a buffer of the representation.
2. **Alignment** - printing multiple rows of VisualizeHex should offset every
   second row by n ascii characters. That is to say, if the Visualizer storage
   array has a high dimension, then it will print the hexes offset by n
3. **Adapting to HexTile** - impl Visualize for HexTile, this should also gather
   any game elements on the hex

Provide simple visualization tests. This is a utility for my convenience.

---

## Not in Phase 2

The following are Phase 3 work and should not be added to this patch:

- Tile appeal calculation
- Loyalty pressure and city flips
- Combat resolution
- District placement and adjacency bonuses
- Eureka/inspiration trigger evaluation
- Trade route yield calculation
- Diplomacy state transitions
- Religion spread
- Great person recruitment
- Governor establishment timers
- Era transition evaluation
- Victory condition checking
