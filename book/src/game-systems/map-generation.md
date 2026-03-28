# Map Generation

The `world::mapgen` module provides a procedural map generation pipeline that creates realistic hex-grid worlds with continents, climate zones, terrain features, rivers, resources, and starting positions.

## Configuration

```rust
struct MapGenConfig {
    width: usize,       // board width in tiles
    height: usize,      // board height in tiles
    seed: u64,          // RNG seed for determinism
    land_fraction: Option<f64>,   // target land/water ratio (default ~0.35)
    num_continents: Option<usize>, // continent count (default: width-based)
    num_zone_seeds: Option<usize>, // climate zone seeds
    num_starts: usize,  // number of starting positions to find
}
```

`MapGenConfig::standard()` provides sensible defaults. The generation is fully deterministic -- the same seed always produces the same map.

## Pipeline

Generation proceeds through six phases, each building on the previous:

### Phase 1: Continents (`continents.rs`)

Seed placement + frontier BFS growth algorithm:
1. Place continent seeds on the grid
2. Grow continents outward using breadth-first search
3. Each tile is assigned to a continent or marked as ocean
4. The target `land_fraction` controls when growth stops

### Phase 2: Climate Zones (`zones.rs`)

Latitude-biased climate zone assignment:
- Polar regions near top/bottom -> Snow, Tundra
- Temperate bands -> Grassland, Plains
- Tropical equatorial band -> Plains, Desert
- Zone seeds add local variation within bands

### Phase 3: Terrain Features (`features.rs`)

Terrain, mountains, and hills placement:
- Mountain ranges follow continent edges and ridgelines
- Hills generated adjacent to mountains and on elevated terrain
- Feature placement (Forest, Rainforest, Marsh, Floodplain, Oasis, Reef) based on climate zone and terrain type
- Floodplains only on Desert adjacent to rivers
- Reefs only on Coast tiles

### Phase 4: Rivers (`rivers.rs`)

River generation following elevation gradients:
- Rivers originate from mountains and hills
- Flow downhill toward the coast
- Stored as edge features between tiles
- Multiple rivers per map with varied lengths

### Phase 5: Resources (`resources.rs`)

Resource placement with quotas and terrain constraints:
- Each resource type has terrain/feature affinity
- Placement density controlled by quota system
- Strategic resources distributed to ensure viable starts
- Bonus and luxury resources fill remaining eligible tiles

### Phase 6: Starting Positions (`starts.rs`)

Starting position selection for civilizations:
- Evaluates tiles for suitability (food, production, fresh water access)
- Enforces minimum distance between starts
- Prefers coastal locations for naval access
- Returns `num_starts` positions ordered by quality

## Result

```rust
struct MapGenResult {
    starting_positions: Vec<HexCoord>,
}
```

The pipeline modifies the `WorldBoard` in place and returns the starting positions. Tiles are fully populated with terrain, features, resources, rivers, and natural wonders.

## Testing

Six integration tests validate the pipeline:
- Land fraction within tolerance (30-60%)
- No invalid terrain combinations (mountain + hills)
- Land tiles have varied terrain (not all Grassland)
- Starting positions are on valid land tiles
- Deterministic: same seed -> same map
- Different seeds -> different maps
