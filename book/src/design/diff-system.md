# Semantic Diffs

Every state-changing operation in the engine returns a `GameStateDiff` -- a structured record of what changed. This is a core architectural decision enabling replay, UI updates, and RL observation.

## GameStateDiff

```rust
struct GameStateDiff {
    deltas: Vec<StateDelta>,
}
```

A simple wrapper around a vector of deltas, with `new()` and `push()` methods.

## StateDelta

The `StateDelta` enum has 40+ variants covering every possible state change:

### Turn
- `TurnAdvanced { new_turn }` -- game turn incremented

### Units
- `UnitMoved { unit, from, to }` -- unit position changed
- `UnitCreated { unit, coord, owner }` -- new unit spawned
- `UnitDestroyed { unit, coord }` -- unit killed/consumed
- `UnitAttacked { attacker, defender, damage_to_attacker, damage_to_defender, attack_type }` -- combat occurred
- `ChargesChanged { unit, remaining }` -- builder charges decremented

### Cities
- `CityFounded { city, coord, owner }` -- new city established
- `CityCaptured { city, old_owner, new_owner }` -- city changed hands
- `PopulationGrew { city, new_pop }` -- city grew
- `CitizenAssigned { city, tile }` -- citizen placed on tile

### Economics
- `GoldChanged { civ, old_gold, new_gold }` -- treasury changed

### Research
- `TechResearched { civ, tech }` -- technology completed
- `CivicCompleted { civ, civic }` -- civic completed
- `EurekaTriggered { civ, tech }` -- eureka bonus applied
- `InspirationTriggered { civ, civic }` -- inspiration bonus applied

### Production
- `BuildingCompleted { city, building }` -- building finished
- `DistrictBuilt { city, district, coord }` -- district placed
- `WonderBuilt { city, wonder }` -- wonder completed
- `ProductionStarted { city, item }` -- new item began production

### World
- `ImprovementPlaced { coord, improvement }` -- tile improvement built
- `RoadPlaced { coord, road }` -- road constructed
- `TileClaimed { coord, city, civ }` -- territory expanded
- `TileReassigned { coord, old_city, new_city }` -- tile moved between cities
- `TilesRevealed { civ, tiles }` -- fog of war updated

### Unlocks
- `ResourceRevealed { civ, resource }` -- strategic resource became visible
- `UnitUnlocked { civ, unit_type }` -- new unit type available
- `BuildingUnlocked { civ, building }` -- new building available
- `ImprovementUnlocked { civ, improvement }` -- new improvement available
- `FreeUnitGranted { civ, unit_type, coord }` -- free unit spawned
- `FreeBuildingGranted { city, building }` -- free building added

### Resources
- `StrategicResourceChanged { civ, resource, old, new }` -- stockpile changed

### Diplomacy
- `DiplomacyChanged { civ_a, civ_b, old_status, new_status }` -- relation changed

### Victory
- `VictoryAchieved { civ, condition }` -- game won

### Era
- `HistoricMomentEarned { civ, moment_name, era_score }` -- era score gained
- `EraAdvanced { civ, new_era, era_age }` -- era transition

### Tourism
- `TourismGenerated { civ, amount }` -- tourism produced
- `LifetimeCultureUpdated { civ, total }` -- culture accumulated

### Loyalty
- `LoyaltyChanged { city, old, new }` -- city loyalty shifted
- `CityRevolted { city, old_owner, new_owner }` -- city flipped

## Design Benefits

### Replay

A game can be reconstructed from:
1. Initial `GameState` (or just the seed)
2. Sequence of player actions
3. All `GameStateDiff`s

This enables save/load, spectator mode, and post-game analysis.

### RL Observation

Agents receive structured `StateDelta` observations instead of having to diff raw state. This makes reward shaping straightforward:
- `CityFounded` -> positive reward
- `UnitDestroyed(own_unit)` -> negative reward
- `TechResearched` -> positive reward
- etc.

### UI Updates

The web client can use diffs to update only the changed portions of the UI, avoiding full-state re-renders.

### Debugging

Diffs provide a complete audit trail of every state change, making it easy to trace bugs to their exact cause.
