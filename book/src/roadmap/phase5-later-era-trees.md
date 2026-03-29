# Later-Era Tech and Civic Trees

Phase 5, item 12. Extend the tech and civic trees from Ancient through Future eras.

## Current State

- Tech tree (`rules/tech_tree_def.rs`): 12 Ancient era techs (Pottery through Theology) + 1 sentinel.
- Civic tree (`rules/civic_tree_def.rs`): 5 Ancient era civics (Code of Laws through Mysticism) + 1 sentinel.
- `TechRefs` struct has named handles for all Ancient techs. `CivicRefs` similarly.
- `TechNode` supports prerequisites, cost, `OneShotEffect` effects, and eureka data.
- `CivicNode` supports prerequisites, cost, `OneShotEffect` effects, and inspiration data.
- Era system (`civ/era.rs`) defines all 9 eras (Ancient through Future) with `tech_count` and `civic_count` thresholds for era advancement.
- District requirements already reference `TechRefs` and `CivicRefs` for gating.

## Design

Follow Civ VI's tech/civic tree structure. Each era adds ~8-12 techs and ~6-10 civics. Techs unlock units, buildings, improvements, and strategic resources. Civics unlock policies, governments, and districts.

### Era progression

| Era | Approx Techs | Approx Civics | Key Unlocks |
|-----|-------------|---------------|-------------|
| Classical | 8 | 6 | Horseback Riding, Mathematics, Construction; Political Philosophy, Drama & Poetry |
| Medieval | 8 | 6 | Stirrups, Gunpowder, Military Tactics; Divine Right, Mercenaries |
| Renaissance | 8 | 6 | Cartography, Banking, Astronomy; Exploration, Reformed Church |
| Industrial | 8 | 6 | Industrialization, Steam Power, Rifling; Nationalism, Urbanization |
| Modern | 8 | 6 | Electricity, Radio, Flight; Suffrage, Ideology |
| Atomic | 6 | 4 | Nuclear Fission, Rocketry, Computers; Cold War, Space Race |
| Information | 6 | 4 | Satellites, Nuclear Fusion, Nanotechnology; Globalization |
| Future | 4 | 3 | Speculative techs for endgame |

## Implementation Plan

### Step 1: Extend `TechRefs` and `CivicRefs`

Add named fields for key techs/civics that are referenced by other systems:
- `horseback_riding`, `mathematics`, `construction` (Classical)
- `stirrups`, `gunpowder` (Medieval)
- `cartography`, `astronomy` (Renaissance)
- `industrialization`, `steam_power`, `rifling` (Industrial)
- `electricity`, `flight`, `radio` (Modern)
- `nuclear_fission`, `rocketry`, `computers` (Atomic)
- `satellites`, `nuclear_fusion` (Information)

Similarly for `CivicRefs`: `political_philosophy`, `divine_right`, `nationalism`, etc.

### Step 2: Populate `tech_tree_def.rs`

For each era, add `TechNode` entries with:
- `cost`: scaling by era (Ancient ~25, Classical ~50, Medieval ~100, etc.)
- `prerequisites`: 1-2 techs from the same or previous era
- `effects`: `OneShotEffect` variants for unit unlocks, building unlocks, improvement unlocks, resource reveals
- `eureka_description` and `eureka_effects`: half-cost boost conditions

### Step 3: Populate `civic_tree_def.rs`

Same structure as tech. Key civics unlock:
- Governments (Autocracy, Oligarchy, Classical Republic at Political Philosophy; Monarchy, Theocracy at Medieval civics)
- Policy cards
- District types (TheaterSquare at Drama & Poetry, etc.)

### Step 4: Wire effects

Each tech/civic node's `OneShotEffect`s need corresponding content:
- `UnlockUnit(name)` -- unit type must exist in `unit_type_defs`
- `UnlockBuilding(name)` -- building must exist in `building_defs`
- `RevealResource(resource)` -- strategic resource becomes visible
- `GrantModifier(modifier)` -- permanent modifier for the researching civ
- `UnlockGovernment(name)` -- government must exist

### Step 5: Update era advancement thresholds

Adjust `Era::tech_count` and `Era::civic_count` in `civ/era.rs` to match the new tree sizes. Currently the era definitions exist but thresholds may need tuning.

### Step 6: Tests

1. Research a Classical tech, verify prerequisites enforced.
2. Research all Ancient+Classical techs, verify era advances to Medieval.
3. Eureka conditions grant correct cost reduction.
4. Key unlocks (units, buildings, governments) become available after research.

## Complexity

High (content volume). The framework is solid -- this is primarily data entry with effect wiring. Estimate ~500 lines for tech definitions, ~400 for civic definitions, ~100 for wiring and tests.

## Dependencies

- Unit types for each era need to be defined (Phase 5, item 13 or done inline).
- Building definitions for later-era buildings.
- Government definitions for later-era governments.
- Science victory milestones reference specific late-era techs (Phase 3, item 6).
