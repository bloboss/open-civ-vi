# Cities & Population

Cities are the primary economic and production centers. They grow population, produce units and buildings, and control surrounding territory.

## City Structure

```rust
struct City {
    id: CityId,
    name: String,
    owner: CivId,
    founded_by: CivId,
    coord: HexCoord,
    kind: CityKind,
    ownership: CityOwnership,
    is_capital: bool,

    // Population
    population: u32,
    food_stored: i32,
    food_to_grow: i32,

    // Production
    production_stored: i32,
    production_queue: Vec<ProductionItem>,

    // Military
    walls: WallLevel,
    wall_hp: i32,
    has_attacked_this_turn: bool,

    // Infrastructure
    buildings: Vec<BuildingId>,
    districts: Vec<BuiltinDistrict>,
    worked_tiles: Vec<HexCoord>,
    locked_tiles: Vec<HexCoord>,
    territory: Vec<HexCoord>,
    culture_border: CultureBorder,

    // Systems
    loyalty: f64,
    great_work_slots: Vec<GreatWorkSlot>,
}
```

## City Kinds

```rust
enum CityKind {
    Regular,
    CityState(CityStateData),
}
```

City-states are stored as regular `City` objects with `CityKind::CityState`. This avoids a separate collection and allows city-states to share all city mechanics. Access via `GameState::city_state_by_civ(CivId)`.

## Ownership

```rust
enum CityOwnership {
    Normal,    // founded or peacefully acquired
    Occupied,  // captured in war (loyalty penalty)
    Puppet,    // autonomous (reduced yields)
    Razed,     // being destroyed
}
```

Occupied cities suffer faster loyalty decay. See the [loyalty section](./culture-and-tourism.md) for revolt mechanics.

## Population Growth

Each turn, the city accumulates food toward the growth threshold (`food_to_grow`). When the threshold is reached:
1. Population increases by 1
2. `food_stored` resets
3. `food_to_grow` increases for the next level

The growth rate depends on `compute_yields()` -- specifically the net food after feeding the population (each citizen consumes 2 food).

## Production Queue

Cities maintain an ordered production queue:

```rust
enum ProductionItem {
    Unit(UnitTypeId),
    Building(BuildingId),
    District(BuiltinDistrict),
    Wonder(WonderId),
}
```

Each turn, production yield is applied to the first item in the queue. When accumulated production meets or exceeds the item's cost, it completes and the next item begins.

## Yields

**Yields are never stored on `City`** -- they are always computed at query time via `RulesEngine::compute_yields()`. This ensures modifiers (from techs, policies, buildings, wonders, religions) are always correctly applied.

The yield computation considers:
- Base yields from worked tiles (terrain + hills + feature + resource + improvement)
- Building yields
- District adjacency bonuses
- Trade route income
- Policy modifiers
- Tech/civic modifiers
- Civilization ability bonuses
- Base +1 science and +1 culture per city

## Territory

Each city controls a set of tiles (`territory: Vec<HexCoord>`). Territory is acquired through:
1. **Founding**: the city center tile + ring-1 neighbors are claimed immediately
2. **Cultural expansion**: tiles in rings 2-5 are gradually claimed as culture accumulates (see [Culture & Borders](./culture-and-tourism.md))
3. **Purchase**: (not yet implemented) spending gold to claim a tile
4. **Conquest**: capturing a city transfers its territory

## Walls

```rust
enum WallLevel {
    None,
    Ancient,
    Medieval,
    Renaissance,
}
```

Each wall level adds a defense bonus and grants the city a ranged bombardment attack. Wall HP is separate from city HP and must be reduced before the city can be captured.

## Citizen Assignment

Citizens can be assigned to specific tiles within the city's territory:
- `AssignCitizen { city, tile, lock }` -- assigns a citizen to work a tile
- `UnassignCitizen { city, tile }` -- frees a citizen
- `locked_tiles` prevents automatic reassignment during optimization
- The number of workable tiles equals the city's population
