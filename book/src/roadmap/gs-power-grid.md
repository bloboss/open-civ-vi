# GS-1: Power & Energy Grid

> **Reference**: `original-xml/DLC/Expansion2/Data/Expansion2_Buildings.xml`,
> `Expansion2_GlobalParameters.xml`

## Overview

Buildings from the Industrial era onward consume power. Cities must generate
sufficient power from power plants (or renewable improvements) or suffer yield
penalties. Fossil fuel plants emit CO2, feeding the climate system (GS-2).

## Data Model

### New fields

**On `City`:**
```rust
pub power_consumed: u32,    // sum of all building power costs
pub power_generated: u32,   // sum of all power plant output
```

**On `BuildingDef`:**
```rust
pub power_cost: u32,        // power consumed per turn (0 for most buildings)
pub power_generated: u32,   // power output (only power plants)
pub co2_per_turn: u32,      // CO2 emitted per turn (fossil fuel plants only)
```

**On `GameState`:**
```rust
pub global_co2: u32,        // cumulative CO2 (feeds climate system)
```

### Power balance

Each turn during `compute_yields()`:
1. Sum `power_generated` from all power-generating buildings in the city
2. Sum `power_cost` from all power-consuming buildings
3. If `generated >= consumed`: city gets bonus yields (+3 Production from
   Factory, +4 from Power Plant, etc. — the "powered" bonus)
4. If `generated < consumed`: city loses the powered bonus on all buildings
   that require power (brownout)

### Power-generating buildings (from XML)

| Building | District | Cost | Power | CO2/turn | Prereq Tech |
|---|---|---|---|---|---|
| Coal Power Plant | Industrial Zone | 580 | 4 | 1 | Industrialization |
| Oil Power Plant | Industrial Zone | 580 | 4 | 1 | Refining |
| Nuclear Power Plant | Industrial Zone | 580 | 16 | 0 | Nuclear Fission |
| Hydroelectric Dam | Dam | 580 | 6 | 0 | Electricity |
| Solar Farm | (improvement) | — | 2 | 0 | — |
| Wind Farm | (improvement) | — | 2 | 0 | — |
| Geothermal Plant | (improvement) | — | 4 | 0 | — |

### Power-consuming buildings

Most tier-3 district buildings (Research Lab, Broadcast Center, Military
Academy, Seaport, Stadium, etc.) and some tier-2 buildings consume 1–2 power.

## Implementation Steps

1. Add `power_cost`, `power_generated`, `co2_per_turn` to `BuildingDef`
2. Add `global_co2` to `GameState`
3. Add power plant buildings to `building_defs.rs`
4. Add `StateDelta::PowerBalanceChanged { city, generated, consumed }`
5. In `compute_yields()`, check city power balance; apply/withhold powered bonuses
6. In `advance_turn`, accumulate `global_co2` from all fossil fuel plants
7. Add 5+ integration tests

## Files Modified

| File | Changes |
|---|---|
| `libciv/src/game/state.rs` | Add fields to `BuildingDef` and `GameState` |
| `libciv/src/civ/city.rs` | Add `power_consumed`/`power_generated` |
| `libciv/src/rules/building_defs.rs` | Add power plant buildings, set power values |
| `libciv/src/game/rules/turn_phase.rs` | CO2 accumulation phase |
| `libciv/src/game/rules/city.rs` | Power balance in yield computation |
| `libciv/src/game/diff.rs` | New delta variant |

## Dependencies

- **Blocks**: GS-2 (climate needs CO2 data), GS-8 (power plant buildings)
- **Blocked by**: Nothing — can start immediately
