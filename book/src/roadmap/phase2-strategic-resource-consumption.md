# Strategic Resource Consumption

Phase 2, item 5. Enforce resource costs when producing units.

## Current State

This is **already implemented**. The status.md description is outdated.

### What exists

- **`UnitTypeDef::resource_cost: Option<(BuiltinResource, u32)>`** (`game/state.rs`) -- each unit type can require a strategic resource.
- **`Civilization::strategic_resources: HashMap<BuiltinResource, u32>`** -- per-civ stockpile tracking.
- **Resource accumulation** in `advance_turn` Phase 2 (`game/rules.rs:1080`): districts on tiles with strategic resources gain +1 per turn. Emits `StrategicResourceChanged`.
- **Resource consumption on production** (`game/rules.rs:1140`): when a unit completes production, the engine checks `resource_cost`, verifies `available >= required`, deducts from stockpile, and emits `StrategicResourceChanged { delta: -(required) }`. If insufficient, the unit completion is deferred (`continue`).
- **`RulesError::InsufficientStrategicResource`** error variant exists.
- **7 built-in strategic resources**: `BuiltinResource` enum includes Horses, Iron, Niter, Coal, Oil, Aluminum, Uranium.
- **Integration tests**: `test_strategic_resource_blocks_production` and `test_strategic_resource_consumed_on_completion` in `gameplay.rs` verify the full flow.

### Remaining work

None for core functionality. The system is complete with tests.

Optional enhancement: **production queue validation** -- currently, a player can queue a unit they can't afford resource-wise. The unit simply won't complete until resources are available. A UI-facing `can_produce(civ_id, unit_type_id) -> bool` helper that pre-checks resource availability would improve the player experience but is not a rules-engine concern.
