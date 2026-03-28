# Trait-Based Extensibility

The engine is designed for extensibility through Rust traits. New game content -- civilizations, units, buildings, terrain, wonders -- can be added by implementing the relevant traits and linking at compile time. There is no runtime scripting layer.

## Extension Points

### Terrain & Features

Built-in terrain uses the `BuiltinTerrain` enum with direct method dispatch:

```rust
enum BuiltinTerrain {
    Grassland, Plains, Desert, Tundra, Snow, Coast, Ocean, Mountain,
}

impl BuiltinTerrain {
    fn name(&self) -> &'static str { ... }
    fn base_yields(&self) -> YieldBundle { ... }
    fn movement_cost(&self) -> MovementCost { ... }
}
```

The same pattern is used for `BuiltinFeature`, `BuiltinResource`, and `BuiltinImprovement`.

### Road and Edge Features

Roads and edge features use a trait + enum wrapper pattern:

```rust
trait RoadDef {
    fn name(&self) -> &'static str;
    fn movement_cost(&self) -> MovementCost;
    fn maintenance(&self) -> i32;
}

enum BuiltinRoad {
    Ancient(AncientRoad),
    Medieval(MedievalRoad),
    // ...
}

impl BuiltinRoad {
    fn as_def(&self) -> &dyn RoadDef { ... }
}
```

### Natural Wonders

```rust
trait NaturalWonder {
    fn id(&self) -> NaturalWonderId;
    fn name(&self) -> &'static str;
    fn appeal_bonus(&self) -> i32;
    fn yield_bonus(&self) -> YieldBundle;
    fn movement_cost(&self) -> MovementCost;
    fn impassable(&self) -> bool;
}
```

### Victory Conditions

```rust
trait VictoryCondition {
    fn id(&self) -> VictoryId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn kind(&self) -> VictoryKind;
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress;
}
```

New victory conditions (e.g., Science, Diplomatic, Religious) implement this trait and are registered on `GameState.victory_conditions`.

### Civilization Abilities

```rust
trait LeaderAbility {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn modifiers(&self) -> Vec<Modifier>;
}

trait Agenda {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn attitude(&self, toward: CivId) -> DiplomaticStatus;
}
```

### Great People

```rust
trait GreatPersonAbility {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn uses(&self) -> u8;
}
```

### Governors

```rust
trait GovernorDef {
    fn id(&self) -> GovernorId;
    fn name(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn base_ability_description(&self) -> &'static str;
}
```

### Religious Beliefs

```rust
trait Belief {
    fn id(&self) -> BeliefId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
}
```

## Design Constraints

### No Clone for Trait Objects

Structs containing `Box<dyn Trait>` fields (e.g., `Leader`, `Civilization`) cannot derive `Clone`. This is documented on affected types. If you need to copy such a struct, implement a manual cloning strategy or use reference counting.

### Static Strings for Built-in Content

All built-in content uses `&'static str` for names and descriptions. This avoids allocation overhead for compile-time-known data. Only user-provided data (player names, custom game names) uses `String`.

### Compile-Time Linking

Extensions are linked at compile time by adding them as dependencies and registering them with the `GameState`. There is no plugin system or dynamic loading. This ensures type safety and eliminates runtime errors from malformed content definitions.

## Adding New Content

To add a new civilization, for example:

1. Implement `LeaderAbility` for the leader's abilities
2. Implement `Agenda` for the AI personality
3. Create a `Leader` with the abilities and agenda
4. Create a `Civilization` with the leader
5. Register any unique units, buildings, or improvements
6. Add the civilization to the `CivRegistry` or game setup code

The modifier system handles most gameplay effects -- you rarely need to modify the rules engine itself.
