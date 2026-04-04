# GS-2: Climate Change & Environmental Disasters

> **Reference**: `original-xml/DLC/Expansion2/Data/Expansion2_RandomEvents.xml`,
> `Expansion2_Features.xml`

## Overview

Two interacting systems: progressive climate change driven by CO2 emissions,
and random environmental disasters that damage tiles and units but may add
fertility.

## Climate Change

### Sea Level Rise

7 progressive levels driven by `global_co2` thresholds:

| Level | CO2 Threshold | Effect |
|---|---|---|
| 1 | 200 | Warning — coastal lowlands at risk |
| 2 | 400 | 1m rise — submerge lowest coastal tiles |
| 3 | 600 | 2m rise — submerge more tiles |
| 4 | 800 | 3m rise — significant coastal loss |
| 5–7 | 1000+ | Progressive catastrophe |

**Coastal lowland tiles**: Pre-tagged during mapgen (1m, 2m, 3m elevation
categories). When sea level reaches a threshold, matching tiles become Coast.

**Flood Barrier**: Building that protects a city's coastal tiles from
submersion. Cost scales with city size.

### Data Model

**On `GameState`:**
```rust
pub climate_level: u8,           // 0–7 current sea level
pub co2_thresholds: [u32; 7],    // CO2 levels that trigger each rise
```

**On `WorldTile`:**
```rust
pub coastal_lowland: Option<u8>, // 1, 2, or 3 (meters above sea level)
pub submerged: bool,             // true if underwater from sea level rise
```

## Environmental Disasters

### Disaster Types (from XML)

| Type | Subtypes | Affected Terrain | Effect |
|---|---|---|---|
| Volcanic Eruption | Gentle, Catastrophic, Megacolossal | Volcanic tiles | Damage + VolcanicSoil fertility |
| Flood | Moderate, Major, 1000-year | Floodplain tiles | Damage + food bonus |
| Blizzard | Significant, Crippling | Tundra/Snow | Unit damage, movement penalty |
| Dust Storm | Gradient, Haboob | Desert | Yield penalty, unit damage |
| Tornado | Family, Outbreak | Plains/Grassland | Improvement destruction |
| Hurricane | Cat 4, Cat 5 | Coast/adjacent land | Wide-area damage |
| Drought | Major, Extreme | Any land | Food yield penalty |
| Nuclear Accident | Minor, Major, Catastrophic | Nuclear plant tile | Fallout contamination |

### Resolution

Each turn in `advance_turn` (new phase after loyalty):
1. Roll for each eligible disaster type (probability scales with climate level)
2. Select affected tiles based on terrain/feature matching
3. Apply damage: destroy improvements, damage units, reduce population
4. Apply benefits: volcanic eruptions add VolcanicSoil, floods add food
5. Emit `StateDelta::DisasterOccurred { type, tiles, damage, benefits }`

### Data Model

```rust
pub struct DisasterEvent {
    pub kind: DisasterKind,
    pub severity: u8,           // 1–3
    pub affected_tiles: Vec<HexCoord>,
    pub turn: u32,
}

pub enum DisasterKind {
    VolcanicEruption,
    Flood,
    Blizzard,
    DustStorm,
    Tornado,
    Hurricane,
    Drought,
    NuclearAccident,
}
```

## Implementation Steps

1. Add `coastal_lowland` and `submerged` to `WorldTile`
2. Tag coastal lowlands during mapgen (based on distance from coast + elevation)
3. Add `climate_level` and disaster tracking to `GameState`
4. Create `libciv/src/world/climate.rs` — CO2 thresholds, sea level logic
5. Create `libciv/src/world/disaster.rs` — disaster types, probability, resolution
6. Add disaster phase to `advance_turn` (after loyalty, before era score)
7. Add `StateDelta::DisasterOccurred`, `SeaLevelRose`, `TileSubmerged`
8. Update mapgen to tag coastal lowlands
9. Add Flood Barrier building to building_defs
10. Add 8+ integration tests (one per disaster type + climate progression)

## Files

| File | Changes |
|---|---|
| `libciv/src/world/climate.rs` | **NEW** — climate tracking |
| `libciv/src/world/disaster.rs` | **NEW** — disaster system |
| `libciv/src/world/tile.rs` | Add coastal_lowland, submerged |
| `libciv/src/world/mapgen/` | Tag coastal lowlands |
| `libciv/src/game/state.rs` | Add climate fields |
| `libciv/src/game/rules/turn_phase.rs` | Disaster phase |
| `libciv/src/game/diff.rs` | New delta variants |

## Dependencies

- **Blocked by**: GS-1 (CO2 accumulation from power plants)
- **Blocks**: Nothing directly (but climate affects all late-game play)
