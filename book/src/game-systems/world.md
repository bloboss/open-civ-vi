# World & Terrain

The game world is a hex grid of `WorldTile` objects, each combining terrain, elevation, features, resources, improvements, and roads.

## Terrain

`BuiltinTerrain` defines the 8 base terrain types:

| Terrain | Yields | Movement | Land/Water |
|---------|--------|----------|------------|
| Grassland | 2 Food | 1 | Land |
| Plains | 1 Food, 1 Production | 1 | Land |
| Desert | -- | 1 | Land |
| Tundra | 1 Food | 1 | Land |
| Snow | -- | 1 | Land |
| Coast | 1 Food, 1 Gold | 1 | Water |
| Ocean | 1 Food | 1 | Water |
| Mountain | -- | Impassable | Land |

Mountains are impassable (`Elevation::High`). Hills are represented by the `hills: bool` flag on `WorldTile`, granting +1 Production and a 3% defense bonus.

## Features

`BuiltinFeature` adds modifiers on top of base terrain:

| Feature | Yield Modifier | Movement Modifier | Notes |
|---------|---------------|-------------------|-------|
| Forest | +1 Production | +1 | Conceals resources |
| Rainforest | +1 Food | +1 | Conceals resources |
| Marsh | -1 Production | +1 | Defense penalty |
| Floodplain | +3 Food | -- | Desert only |
| Reef | +1 Food, +1 Production | -- | Water only |
| Ice | -- | Impassable | |
| Volcanic Soil | +1 Food, +1 Production | -- | |
| Oasis | +3 Food, +1 Gold | -- | Desert only |

Features that conceal resources (Forest, Rainforest) hide the underlying resource from yield calculations until the feature is cleared.

## Resources

`BuiltinResource` includes 23 resources in three categories:

### Bonus Resources
Wheat, Rice, Cattle, Sheep, Fish, Stone, Copper, Deer -- provide yield bonuses when improved.

### Luxury Resources
Wine, Silk, Spices, Incense, Cotton, Ivory, Sugar, Salt -- provide amenities when improved and connected.

### Strategic Resources
Horses, Iron, Coal, Oil, Aluminum, Niter, Uranium -- required for specific units and buildings. Revealed by researching specific techs (e.g., Iron requires Iron Working, Horses require Animal Husbandry).

## Improvements

`BuiltinImprovement` defines 9 tile improvements:

| Improvement | Yields | Tech Required | Constraints |
|-------------|--------|---------------|-------------|
| Farm | +1 Food | Pottery | Land, flat or hills |
| Mine | +1 Production | Mining | Land, hills or more |
| Lumber Mill | +1 Production | (future tech) | Requires Forest |
| Trading Post | +1 Gold | -- | Land |
| Fort | -- | -- | Land, not mountain |
| Airstrip | -- | -- | Land, flat |
| Missile Silo | -- | -- | Land, flat |
| Sphinx | +1 Faith, +1 Culture | -- | Egypt exclusive, desert |
| Stepwell | +1 Food, +1 Housing | -- | India exclusive, not hills |

Improvements are placed by Builder units, consuming one charge per placement. Each improvement has an `ImprovementRequirements` struct specifying terrain, elevation, feature, resource, tech, and proximity constraints.

## Roads

Four road tiers with increasing movement benefits and tech requirements:

| Road | Movement Cost | Maintenance | Tech Required |
|------|--------------|-------------|---------------|
| Ancient | 50 (0.5) | 0 | None |
| Medieval | 34 (~0.33) | 1 gold/turn | Engineering |
| Industrial | 25 (0.25) | 2 gold/turn | Steam Power |
| Railroad | 10 (0.1) | 3 gold/turn | Railroads |

Roads override tile movement cost in pathfinding when `tile.road.is_some()`. Roads cannot be downgraded; they can only be upgraded to a higher tier. Built by Builder units, consuming one charge.

## Edges

Edge features sit between two adjacent tiles:

| Edge Feature | Crossing Cost | Notes |
|-------------|---------------|-------|
| River | +100 (ONE) | Blocks LOS at elevation boundaries |
| Canal | +50 (0.5) | |
| Mountain Pass | +200 (TWO) | |

Edges are stored in canonical forward-half form -- see the [libhexgrid chapter](../architecture/libhexgrid.md#edge-canonicalization) for details.

## Natural Wonders

Five built-in natural wonders with unique yields and effects:

| Wonder | Yield Bonus | Appeal | Impassable |
|--------|------------|--------|------------|
| Krakatoa | +3 Science | +2 | Yes |
| Grand Mesa | +2 Production | +1 | No |
| Cliffs of Dover | +2 Gold, +2 Culture | +2 | No |
| Uluru | +2 Faith, +2 Culture | +3 | No |
| Galapagos Islands | +2 Science | +2 | Yes |

## WorldTile

The `WorldTile` struct combines all of the above:

```rust
struct WorldTile {
    coord: HexCoord,
    terrain: BuiltinTerrain,
    hills: bool,
    feature: Option<BuiltinFeature>,
    resource: Option<BuiltinResource>,
    improvement: Option<BuiltinImprovement>,
    improvement_pillaged: bool,
    road: Option<BuiltinRoad>,
    rivers: Vec<HexDir>,
    natural_wonder: Option<BuiltinNaturalWonder>,
    owner: Option<CivId>,
}
```

`total_yields()` computes the combined yields from terrain + hills + feature + resource + improvement. The `terrain_defense_bonus()` method returns combat bonuses from hills (3%), forest (3%), and marsh (-3%).
