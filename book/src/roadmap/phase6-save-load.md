# Save/Load

Phase 6, item 16. Serialize `GameState` for persistence.

## Current State

- `serde` is an optional feature in `libciv/Cargo.toml`: `serde = ["dep:serde", "libhexgrid/serde"]`.
- Many types have `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]`: `BuiltinTerrain`, `BuiltinFeature`, `BuiltinResource`, `BuiltinImprovement`, `BuiltinRoad`, `BasicUnit`, `UnitCategory`, `UnitDomain`, `VictoryKind`, `BuiltinDistrict`, `BuiltinCiv`, `BuiltinLeader`, etc.
- **Blockers**: `GameState` itself has no serde derives. Several fields contain trait objects (`Box<dyn VictoryCondition>`, `Box<dyn Agenda>` on `Leader`) that cannot be auto-serialized.
- `IdGenerator` contains internal RNG state that needs deterministic serialization.

## Design

### Serialization strategy

Use `serde` with JSON (or MessagePack for compactness) for the data layer. Handle trait objects with a **tag-based registry** pattern: serialize the type name as a string tag, deserialize by matching against known types.

### Trait object handling

| Type | Trait Object | Strategy |
|------|-------------|----------|
| `VictoryCondition` | `Box<dyn VictoryCondition>` | Serialize name + config; reconstruct from registry on load |
| `Leader::agenda` | `Box<dyn Agenda>` | Serialize agenda name; look up in built-in agenda registry |
| `Leader::abilities` | `Vec<Box<dyn LeaderAbility>>` | Serialize ability names; reconstruct from registry |

Alternative: replace trait objects with enums for serializable types. `VictoryCondition` has only 5 concrete types -- an enum dispatch is simpler than dynamic dispatch for serialization.

## Implementation Plan

### Step 1: Audit serde coverage

Add `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` to all remaining types:
- `GameState`, `City`, `Civilization`, `Religion`, `TradeRoute`, `PlacedDistrict`
- `GreatPerson`, `GreatPersonDef`, `Governor`
- `TechTree`, `TechNode`, `CivicTree`, `CivicNode`
- `Modifier`, `EffectType`, `TargetSelector`, `Condition`
- `DiplomaticRelation`, `GrievanceRecord`

### Step 2: Handle trait objects

Option A (enum replacement):

```rust
pub enum VictoryConditionKind {
    Score { turn_limit: u32 },
    Culture,
    Domination,
    Science,
    Religious,
    Diplomatic { threshold: u32 },
}
```

Replace `Vec<Box<dyn VictoryCondition>>` with `Vec<VictoryConditionKind>` + dispatch methods.

Option B (custom serde):

```rust
#[serde(tag = "type")]
enum SerializableVictory {
    Score { turn_limit: u32 },
    Culture,
    // ...
}
```

Implement `From<&dyn VictoryCondition> -> SerializableVictory` and back.

**Recommendation**: Option A is cleaner long-term. The trait-based extensibility can be preserved for modding while the built-in types use enum dispatch.

### Step 3: `IdGenerator` serialization

Serialize the seed and the internal counter state. On deserialization, reconstruct the RNG from seed and fast-forward the counter to the saved position.

```rust
#[derive(Serialize, Deserialize)]
pub struct IdGeneratorState {
    pub seed: u64,
    pub counter: u64,
}
```

### Step 4: Save/load API

```rust
impl GameState {
    pub fn save(&self) -> Result<Vec<u8>, SaveError>;
    pub fn load(data: &[u8]) -> Result<GameState, LoadError>;
}
```

JSON for human-readable saves, bincode/MessagePack for compact saves.

### Step 5: Tests

1. Create a game state, save, load, verify all fields match.
2. Advance several turns, save, load, advance more turns -- verify deterministic behavior continues.
3. Save with religions, great people, trade routes, wonders -- verify complex state round-trips.

## Complexity

Medium-high. The framework (`serde`) is standard, but the trait object handling requires careful design. ~200 lines for serde annotations, ~150 for trait object serialization, ~100 for save/load API and tests.

## Dependencies

- All game content types must have serde derives (incremental, can be done alongside feature work).
- The `IdGenerator` serialization strategy affects replay determinism.
