# Modifier Pipeline

The modifier system is the backbone of all numeric effects in the game. Every bonus, penalty, and effect -- from techs, policies, buildings, wonders, beliefs, great people, and civilization abilities -- is expressed as a `Modifier`.

## Modifier Structure

```rust
struct Modifier {
    source: ModifierSource,
    target: TargetSelector,
    effect: EffectType,
    stacking: StackingRule,
    condition: Option<Condition>,
}
```

### ModifierSource

Tracks where the modifier came from:

| Source | Origin |
|--------|--------|
| `Tech` | Technology research |
| `Civic` | Civic completion |
| `Policy` | Active policy card |
| `Building` | Constructed building |
| `Wonder` | World wonder |
| `Leader` | Leader ability |
| `Religion` | Religious belief |
| `Era` | Era age bonus |
| `CivAbility` | Civilization unique ability |
| `Custom` | Extension point |

### TargetSelector

Determines what the modifier affects:

| Selector | Applies To |
|----------|-----------|
| `AllTiles` | All tile yield calculations |
| `AllUnits` | All unit stat calculations |
| `UnitDomain(domain)` | Units of a specific domain (Land/Sea/Air) |
| `Civilization(civ_id)` | A specific civilization |
| `Global` | Everything |
| `UnitType` | A specific unit type |
| `AdjacentEnemyUnits` | Enemy units adjacent to the source |
| `TradeRoutesOwned` | Trade routes owned by the civ |
| `ProductionQueue` | Items in production |
| `DistrictAdjacency` | District adjacency bonuses |

### EffectType

The actual numeric effect:

| Effect | Description |
|--------|-------------|
| `YieldFlat(YieldType, i32)` | Add flat yield bonus |
| `YieldPercent(YieldType, i32)` | Percentage yield bonus |
| `CombatStrengthFlat(i32)` | Flat combat strength modifier |
| `CombatStrengthPercent(i32)` | Percentage combat strength modifier |
| `MovementFlat(i32)` | Movement point modifier |
| `HousingFlat(i32)` | Housing modifier |
| `AmenityFlat(i32)` | Amenity modifier |
| `ProductionPercent(i32)` | Production speed modifier |
| `PolicySlotExtra(PolicyType)` | Extra policy slot |
| `DistrictSlotExtra` | Extra district slot per city |
| `BuildingMaintenanceReduction(i32)` | Reduced building upkeep |

### StackingRule

Controls how multiple modifiers of the same type combine:

| Rule | Behavior |
|------|----------|
| `Additive` | All values sum together |
| `Max` | Only the highest value applies |
| `Replace` | The last applied value wins |

## Conditions

Modifiers can have optional conditions that gate their application:

### Tile Conditions
- `AdjacentToRiver` -- tile is next to a river
- `OnHills` -- tile has hills
- `OnCoast` -- tile is coastal

### Scaling Conditions
These return `ConditionResult::Scale(n)` to multiply the effect:
- `PerCityStateSuzerain` -- scales by number of city-state suzerainties
- `PerAdjacentDistrict` -- scales by adjacent district count
- `PerTradingPostInRoute` -- scales by trading posts along route
- `PerForeignCityWithWorshipBuilding` -- scales by foreign worship buildings
- `PerCivMetWithReligionNotAtWar` -- scales by peaceful religious contacts

### Composite Conditions
- `And(Box<Condition>, Box<Condition>)` -- both conditions must pass

## Resolution

`resolve_modifiers()` takes a collection of modifiers and produces final values:

1. Group modifiers by `(TargetSelector, EffectType)`
2. Within each group, apply the stacking rule:
   - `Additive`: sum all values
   - `Max`: keep the highest value
   - `Replace`: keep the last value
3. Evaluate conditions, applying scaling factors where relevant
4. Return the resolved modifier set

## Query-Time Application

Modifiers are **never baked into stored state**. Instead, every time a value is needed (city yields, combat strength, movement points), the engine:

1. Collects all active modifiers from all sources
2. Filters by target selector
3. Evaluates conditions against the current context
4. Resolves using stacking rules
5. Returns the computed value

This ensures correctness when modifiers are added, removed, or changed (e.g., swapping policies, completing research, losing a building to pillage).
