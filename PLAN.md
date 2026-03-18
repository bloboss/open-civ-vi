# Map Generation Pipeline — Implementation Plan

## Status legend
- [ ] pending
- [x] done

---

## Checklist

- [x] Write PLAN.md
- [x] Phase 0 — module scaffold (mapgen/ mod.rs, world/mod.rs)
- [x] Phase 1 — continents.rs: seed placement + frontier BFS growth + coast pass
- [x] Phase 2 — zones.rs: latitude-biased seed placement + simultaneous BFS
- [x] Phase 3 — features.rs: base terrain, linear mountain ranges, hills, interior features
- [x] Phase 4 — rivers.rs: source selection + Dijkstra to coast + edge placement
- [x] Phase 5 — resources.rs: per-zone quota placement
- [x] Phase 6 — starts.rs: habitable starting position selection
- [x] Wire into civsim: remove randomize_terrain(), use mapgen in build_session/build_ai_demo
- [x] Integration test (libciv/tests/mapgen.rs — 6 tests)
- [x] cargo test --workspace + cargo clippy --workspace -- -D warnings

---

## Module: libciv/src/world/mapgen/mod.rs

Public surface:

```
MapGenConfig {
    width, height, seed: u64,
    land_fraction: Option<f32>,   // None = auto from map size
    num_continents: Option<u32>,  // None = area/400 clamped 2..=7
    num_zone_seeds: Option<u32>,  // None = area/300 clamped 2..=8
    num_starts: u32,              // 0 = skip Phase 6
}

MapGenResult {
    starting_positions: Vec<HexCoord>,
}

pub fn generate(config: &MapGenConfig, board: &mut WorldBoard, rng: &mut SmallRng)
    -> MapGenResult
```

Land fraction formula (linear interpolation):
  area_tiny = 2_280   -> fraction 0.50
  area_huge = 7_000   -> fraction 0.35
  t = ((area - 2280) / (7000 - 2280)).clamp(0.0, 1.0)
  fraction = 0.50 - t * 0.15

---

## Phase 1 — continents.rs

Inputs:  board (all Grassland after WorldBoard::new), config, rng
Outputs: HashMap<HexCoord, u8> continent_map (0-indexed continent IDs)
         board tiles set to Ocean/Coast/Grassland placeholder

Seed placement (jitter grid):
  Divide map into n equal-area rectangular sectors.
  Per sector: sample random tile; reject if within min_sep hexes of placed seeds
              (min_sep = sqrt(area/n) * 0.55); retry up to 8 times.

Frontier BFS growth:
  per_continent_target = total_tiles * land_fraction / n
  Loop until total_land >= target_land:
    Shuffle frontier each pass.
    For (coord, cid) in frontier:
      For each board.neighbors(coord) not yet assigned:
        p = (1.0 - size[cid] / per_continent_target).clamp(0.25, 0.95)
        if rng.random::<f32>() < p:
          assign tile, add to frontier

Coast post-pass:
  Any Ocean tile with >= 1 land neighbor -> Coast terrain.

---

## Phase 2 — zones.rs

Internal enum EcoZone { Polar, Tundra, Temperate, Desert, Tropical }
Inputs: board, config, rng
Output: HashMap<HexCoord, EcoZone>

Latitude: lat(r) = |r - height/2| as f32 / (height/2) as f32   (0=equator, 1=poles)

Zone affinity curves (piecewise linear, values clamped 0..=1):
  Polar:     0 below lat 0.80, rises to 1.0 at lat 1.0
  Tundra:    peaks 0.9 at lat 0.70, 0 below 0.45
  Temperate: peaks 0.9 at lat 0.40, falls to 0 below 0.15 and above 0.70
  Desert:    peaks 0.9 at lat 0.25, 0 below 0.05 and above 0.45
  Tropical:  0 above lat 0.15, rises to 1.0 at lat 0.0

Seed placement per zone type:
  k = num_zone_seeds per type.
  For each zone: draw candidates weighted by affinity(zone, lat(r)), pick k tiles.

Simultaneous BFS:
  All zone frontiers advance in round-robin order (one step each per round).
  First assignment wins (no overwrites).
  Stop when all tiles assigned.

---

## Phase 3 — features.rs

Inputs: board (terrain set to Grassland placeholder / Ocean / Coast), zone_map, continent_map, rng
Outputs: board tiles fully written (terrain, hills, feature)

Step 3a — base terrain from zone:
  Polar:    Snow (100%)
  Tundra:   Tundra (100%)
  Temperate: Grassland 60%, Plains 40%
  Desert:   Desert 90%, Plains 10%
  Tropical: Grassland 70%, Plains 30%
  Ocean/Coast tiles: unchanged

  Special: Polar ocean coastal -> Ice feature (40%)
           Tropical ocean coastal -> Reef feature (25%)

Step 3b — linear mountain ranges per continent:
  Range count:
    < 40 tiles  -> 0 ranges
    40..=120    -> 1 range
    121..=300   -> 2 ranges
    > 300       -> 3 ranges

  Per range:
    start = random interior land tile (distance >= 4 from coast), prefer zone boundary
    direction = random HexDir
    inertia = 0.70 (70% chance to continue same direction, else turn +/-60deg)
    length = continent_diagonal * uniform(0.20, 0.40)
    Walk: set spine tiles to Mountain; tiles adjacent to spine (non-Mountain) -> hills=true
    If next step would leave land/map: terminate walk early.

  Accent pass (secondary scatter):
    Temperate<->Desert and Desert<->Tropical boundary land tiles: 12% -> Mountain

Step 3c — hill scatter:
  For each land, non-Mountain tile:
    cross = count of neighbors in a different EcoZone / 6.0
    p = 0.30 * cross + 0.06
    if rng.random::<f32>() < p: hills = true

Step 3d — interior features:
  Temperate non-hill land: Forest 35%
  Tropical non-hill land:  Rainforest 60%
  Tropical coast-adjacent land: Marsh 15% (after Rainforest pass; don't double-assign)
  Desert non-hill land:    Oasis 2-4 per continent, min 4-tile separation between oases
  Polar ocean coastal:     Ice feature 40%
  Tropical ocean coastal:  Reef feature 25%

---

## Phase 4 — rivers.rs

Inputs: board (terrain finalized), rng
Output: river edges set on board via set_edge(); tile.rivers updated

Source selection:
  Candidates: land, non-Mountain, non-Coast
  Prefer hills (2x weight).
  Density: 1 per 200 land tiles (min 1 per continent that has >= 40 tiles).
  Min separation between sources: 5 tiles (rejection sampling).

Flow cost for Dijkstra (lower = preferred flow direction):
  Mountain: u32::MAX (skip)
  Hills:    1
  Plains/Tundra/Desert: 2
  Grassland/Snow: 3
  Coast/Ocean: 0 (terminus)

Dijkstra from source to nearest Coast/Ocean tile.
  If no path found: skip source.

Edge placement per path step (A -> B):
  dir = direction from A to B (find from HexDir::ALL which unit_vec matches B-A)
  board.set_edge(A, dir, WorldEdge with River feature)
  Append BuiltinEdgeFeature::River(River) to board.tile_mut(A).rivers
  Append BuiltinEdgeFeature::River(River) to board.tile_mut(B).rivers

Post-river terrain:
  Desert tile with >= 2 adjacent river edges: 45% -> Floodplain feature
  Grassland/Plains tile within last 2 steps before coast: 15% -> Marsh feature

---

## Phase 5 — resources.rs

Inputs: board (terrain + features set), zone_map, rng
Output: board tile.resource set

Quotas (1 resource per N eligible tiles):
  Zone       | Bonus | Luxury | Strategic
  Temperate  |   8   |   20   |   30
  Tropical   |   8   |   18   |   35
  Desert     |  16   |   25   |   40
  Tundra     |  12   |   30   |   25
  Polar      |  20   |  ---   |   20

Placement (per zone, strategic first):
  1. Collect candidates: correct zone, terrain matches resource requirements, no existing resource.
  2. Strategic resources prefer hills=true (2x weight).
  3. Shuffle candidates with rng.
  4. Place up to quota, enforcing min_separation = 2 tiles between same-type resources.

Resource -> zone affinity (which zones a resource appears in):
  Bonus:
    Wheat: Temperate, Tropical (flat land only)
    Rice: Tropical, Temperate (flat)
    Cattle: Temperate, Tundra
    Sheep: Temperate, Tundra, Desert (hills preferred)
    Fish: Ocean/Coast tiles (all zones)
    Stone: Temperate, Tundra (hills)
    Copper: Temperate, Desert (hills)
    Deer: Temperate, Tundra (Forest preferred)
  Luxury:
    Wine: Temperate (Plains, non-feature)
    Silk: Temperate, Tropical (Forest)
    Spices: Tropical (Rainforest)
    Incense: Desert, Tropical
    Cotton: Tropical, Temperate (flat)
    Ivory: Tropical, Desert
    Sugar: Tropical (Marsh/coast-adjacent)
    Salt: Desert, Tundra
  Strategic:
    Horses: Temperate, Tundra, Desert (Plains/Grassland)
    Iron: Temperate, Tundra, Desert (Hills or Mountain-adjacent)
    Coal: Temperate (Hills)
    Oil: Desert (flat), Ocean
    Aluminum: Desert (Hills)
    Niter: Desert, Tundra
    Uranium: Tundra, Temperate (Hills)

---

## Phase 6 — starts.rs

Inputs: board, num_starts, rng
Output: Vec<HexCoord> starting positions

Eligibility:
  terrain in {Grassland, Plains}
  hills == false
  feature == None or Some(Forest)
  no adjacent Mountain tile

Score per candidate:
  2 * adjacent Grassland tiles
  + adjacent Plains tiles
  + 2 if any adjacent river edge
  + 1 if any adjacent bonus resource

min_sep = max(8, (land_tile_count / num_starts).sqrt() * 0.7) as u32

Algorithm:
  Sort candidates by score desc.
  Greedily pick highest-scored, reject all within min_sep, repeat.
  If < num_starts found: relax to allow hills, then lower score threshold to >= 2.

---

## Integration

GameState::new: after WorldBoard::new, construct MapGenConfig::standard(seed, w, h)
  and call generate(&config, &mut state.board, &mut terrain_rng).
  The terrain_rng is seeded independently from id_gen to avoid changing existing ID sequences.

civsim: remove randomize_terrain(); build_session uses MapGenResult.starting_positions
  to place the starting unit instead of hardcoded (7,3).

---

## Tests (libciv/tests/mapgen.rs)

1. land_fraction_within_tolerance: 40x25 map, assert 35%-45% land tiles
2. all_tiles_have_zone (internal - via MapGenConfig test helper)
3. no_mountain_with_hills: assert no tile has terrain==Mountain && hills==true
4. rivers_terminate_at_coast: walk each river edge chain to verify it reaches Coast/Ocean
5. starting_positions_valid: all returned coords are Grassland/Plains, non-hill, non-mountain-adj
6. deterministic: two calls with same seed produce identical boards
