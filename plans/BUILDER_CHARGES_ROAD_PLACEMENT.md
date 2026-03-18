# Plan: Builder Charges and Road Placement

## Context

Builder units exist conceptually in the architecture but are not registered as a unit type.
`place_improvement()` works but does not require or consume a builder unit. Road data types
(`BuiltinRoad`, `RoadDef`) and Dijkstra cost overrides are fully implemented, but no
`place_road()` action exists. Builder charges (limited uses before the unit is consumed) are
not tracked anywhere.

This plan implements three tightly coupled features from TODO sections 8.8 and 8.9:
1. **Builder charges** -- a `charges` field on `BasicUnit` decremented on each build action
2. **Road placement** -- a new `RulesEngine::place_road()` method
3. **Road maintenance** -- gold deduction per road tile in `advance_turn`

---

## Design

### 1. Builder Unit Type

Register a "Builder" unit type in `UnitTypeDef` with:
- `category: UnitCategory::Civilian`
- `combat_strength: None`
- `max_movement: 200` (2 movement points)
- New field `max_charges: u8` (default 0 for non-builder units; 3 for Builder)

### 2. Charges on BasicUnit

Add `charges: Option<u8>` to `BasicUnit`:
- `None` for units without charges (warriors, settlers, etc.)
- `Some(3)` for builders (initialized from `UnitTypeDef::max_charges`)

When a builder action consumes a charge:
1. Decrement `charges`
2. Emit `StateDelta::ChargesChanged { unit, remaining }`
3. If charges reach 0, destroy the unit and emit `StateDelta::UnitDestroyed`

### 3. New StateDelta Variants

```
RoadPlaced { coord: HexCoord, road: BuiltinRoad }
ChargesChanged { unit: UnitId, remaining: u8 }
```

### 4. New RulesError Variants

```
NotABuilder        -- unit is not a builder (category != Civilian or no charges)
NoChargesRemaining -- builder has 0 charges left
RoadDowngrade      -- attempting to place a lower-tier road over a higher one
```

### 5. RulesEngine::place_road()

Signature:
```rust
fn place_road(
    &self,
    state: &mut GameState,
    unit_id: UnitId,
    coord: HexCoord,
    road: BuiltinRoad,
) -> Result<GameStateDiff, RulesError>;
```

Validation:
1. Unit exists and is a builder (has charges)
2. Unit is at `coord`
3. Tile is land
4. Tile is owned by the builder's civ
5. Road tier is not a downgrade (Ancient < Medieval < Industrial < Railroad)
6. Tech requirements: AncientRoad = none, MedievalRoad = "Engineering",
   IndustrialRoad = "Steam Power", Railroad = "Railroads"
   (matched against `TechRefs` or tech tree by name)
7. Builder has charges remaining

On success:
- Set `tile.road = Some(road)`
- Decrement builder charges
- Emit `RoadPlaced`, `ChargesChanged`, and optionally `UnitDestroyed`

### 6. Wire Charges into place_improvement()

Modify `place_improvement()` to accept a `unit_id: Option<UnitId>` parameter (or add a
separate wrapper). When a unit_id is provided:
1. Validate unit is a builder at the coord
2. After successful placement, decrement charges
3. Emit `ChargesChanged` and optionally `UnitDestroyed`

To avoid breaking the existing API, we add a new method `place_improvement_with_builder()`
that wraps the existing validation and adds charge handling. Alternatively, modify the
existing method signature to include `unit_id: Option<UnitId>`.

Decision: Modify `place_improvement()` to add an `Option<UnitId>` parameter for the builder.
Existing callers that pass `None` get the current behavior (no charge decrement).

### 7. Road Maintenance in advance_turn

New phase between Phase 2b (trade routes) and Phase 3 (per-civ yields):
- For each civilization, count road tiles owned by that civ
- Sum `road.as_def().maintenance()` for each road tile
- Deduct from `civ.gold`
- Emit `GoldChanged` delta

### 8. Builder Registration in Test Scenario

Add a "builder" `UnitTypeDef` to `build_scenario()` in `common/mod.rs` with
`max_charges: 3`. Expose `builder_type: UnitTypeId` on `Scenario`.

---

## File Changes

| File | Changes |
|------|---------|
| `libciv/src/civ/unit.rs` | Add `charges: Option<u8>` to `BasicUnit` |
| `libciv/src/game/state.rs` | Add `max_charges: u8` to `UnitTypeDef` |
| `libciv/src/game/diff.rs` | Add `RoadPlaced`, `ChargesChanged` variants |
| `libciv/src/game/rules.rs` | Add `place_road()` trait method + impl; modify `place_improvement()` for charges; add road maintenance phase in `advance_turn`; add `NotABuilder`, `NoChargesRemaining`, `RoadDowngrade` errors |
| `libciv/src/world/road.rs` | Add `tier()` method to `BuiltinRoad` for ordering; derive `PartialEq, Eq` |
| `libciv/tests/common/mod.rs` | Register builder unit type; expose `builder_type` on `Scenario` |
| `libciv/tests/gameplay.rs` | New tests for builder charges and road placement |

---

## Test Plan

1. `builder_charges_decrement_on_improvement` -- place improvement with builder, assert charges decremented
2. `builder_destroyed_when_charges_exhausted` -- place 3 improvements, assert unit destroyed
3. `place_improvement_without_builder_still_works` -- existing behavior preserved
4. `place_road_ancient_succeeds` -- builder places ancient road on owned land tile
5. `place_road_rejects_downgrade` -- medieval road exists, ancient road rejected
6. `place_road_requires_tech` -- medieval road without Engineering tech rejected
7. `place_road_on_water_fails` -- road on ocean tile rejected
8. `place_road_decrements_charges` -- road placement consumes a builder charge
9. `road_maintenance_deducted_per_turn` -- advance_turn deducts gold for road tiles
10. `builder_must_be_at_coord` -- builder at wrong tile rejected

---

## Implementation Order

1. Add `charges` to `BasicUnit`, `max_charges` to `UnitTypeDef` (data layer)
2. Add `StateDelta::RoadPlaced`, `StateDelta::ChargesChanged` (diff layer)
3. Add `BuiltinRoad::tier()` and derive `PartialEq, Eq` (road ordering)
4. Add `RulesError` variants (error layer)
5. Implement `place_road()` on `RulesEngine` trait + `DefaultRulesEngine`
6. Wire charges into `place_improvement()`
7. Add road maintenance phase in `advance_turn`
8. Register builder in test scenario
9. Write and pass all tests
