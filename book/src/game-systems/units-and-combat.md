# Units & Combat

## Unit Types

Units are defined by `UnitTypeDef` and instantiated as `BasicUnit`:

```rust
struct UnitTypeDef {
    id: UnitTypeId,
    name: &'static str,
    production_cost: u32,
    domain: UnitDomain,        // Land, Sea, Air
    category: UnitCategory,    // Civilian, Combat, Support, Religious, GreatPerson, Trader
    max_movement: u32,
    combat_strength: Option<u32>,
    range: Option<u32>,
    vision_range: u8,
    can_found_city: bool,
    resource_cost: Option<(BuiltinResource, u32)>,
    siege_bonus: i32,
    max_charges: u8,           // 3 for builders, 0 for most units
    exclusive_to: Option<&'static str>,  // civ-exclusive unit
    replaces: Option<UnitTypeId>,        // base unit this replaces
}
```

### Unit Categories

| Category | Examples | Notes |
|----------|----------|-------|
| Civilian | Settler, Builder | Cannot attack; captured on contact |
| Combat | Warrior, Archer, Swordsman | Can attack and defend |
| Support | Battering Ram, Siege Tower | Provides bonuses to adjacent combat units |
| Religious | Missionary, Apostle | Spreads religion (not yet implemented) |
| GreatPerson | Great General, Great Merchant | Unique abilities on retirement |
| Trader | Trader | Establishes trade routes between cities |

## Movement

Movement uses Dijkstra pathfinding on the hex grid:

```rust
fn move_unit(state: &mut GameState, unit_id: UnitId, path: &[HexCoord])
    -> Result<GameStateDiff, RulesError>;
```

- Movement costs are scaled by 100 (`ONE=100`)
- Roads override tile movement cost when present
- River crossings add `ONE` (100) to the cost
- Features add their movement modifier on top of terrain cost
- Mountains are impassable; ice is impassable
- Units must have sufficient `movement_left` to enter a tile
- Movement resets to `max_movement` at the start of each turn

## Combat

### Melee Combat

```rust
fn attack(state: &mut GameState, attacker: UnitId, defender: UnitId)
    -> Result<GameStateDiff, RulesError>;
```

Both attacker and defender deal damage simultaneously. Damage is calculated as:

```
base_damage = 30 * (attacker_cs / defender_cs)
```

Where `cs` is effective combat strength after all modifiers.

### Combat Modifiers

Terrain defense bonuses:
- **Hills**: +3% defense
- **Forest**: +3% defense
- **Marsh**: -3% defense (penalty)

Additional modifiers from:
- Great General/Admiral retirement bonuses (+5 CS)
- Civilization-specific abilities (Hoplite adjacency, Samurai damage resistance, Varu debuff)
- Policy modifiers
- Promotions

### City Combat

Cities with walls can perform ranged bombardment attacks each turn (`has_attacked_this_turn` flag). Wall HP must be reduced to zero before the city can be captured by melee attack.

When all defending units in a city tile are destroyed and a melee unit attacks:
- The city is captured
- Ownership transfers to the attacker's civilization
- `CityOwnership` changes to `Occupied`
- Territory transfers to the new owner

### Ranged Combat

Ranged units attack without receiving damage. The `range` field on `UnitTypeDef` determines attack distance. `AttackType::Ranged` is used for ranged attacks and city bombardment.

## Builder Units

Builders have `charges` (default 3) that are consumed when:
- Placing an improvement (`place_improvement`)
- Placing a road (`place_road`)

When charges reach 0, the builder is destroyed. Validation ensures:
- The builder is at the target coordinate
- The tile is owned by the builder's civilization
- Tech/terrain requirements are met
- For roads: no downgrade (must upgrade tier)

## Visibility

Each unit has a `vision_range` (typically 2). The visibility system:
1. Computes all tiles within vision radius of each unit and city
2. Checks line-of-sight using elevation-based blocking
3. Updates `visible_tiles` and `explored_tiles` on the owning civilization
4. Cities always have vision radius 2

LOS is blocked when an intervening tile's elevation is higher than both the observer and the target (cliff blocking).
