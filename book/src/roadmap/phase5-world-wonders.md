# World Wonders

Phase 5, item 14. Constructed wonders with unique effects.

## Current State

- `WonderId` type exists in `ids.rs`.
- `ProductionItem::Wonder(WonderId)` variant exists in `civ/city.rs`.
- `StateDelta::WonderBuilt { civ, wonder: &'static str, city }` exists in `game/diff.rs`.
- `WonderTourism` struct in `civ/tourism.rs` tracks tourism from built wonders.
- `GameState::wonder_tourism: Vec<WonderTourism>` holds registered wonders.
- `HistoricMomentKind::WonderBuilt` awards era score on completion.
- No `WonderDef` registry or wonder definitions exist.
- No wonder placement or construction logic exists.

## Design

World wonders are **unique city production items** (only one per game). Each wonder:
- Requires a specific tech/civic.
- Has a production cost.
- May require placement on a specific district or terrain.
- Grants permanent modifiers and/or one-shot effects.
- Generates tourism.

### Data Model

```rust
pub struct WonderDef {
    pub id: WonderId,
    pub name: &'static str,
    pub production_cost: u32,
    pub required_tech: Option<TechId>,
    pub required_civic: Option<CivicId>,
    pub required_district: Option<BuiltinDistrict>,
    pub terrain_requirement: Option<WonderTerrain>,  // e.g., coast, adjacent to river
    pub modifiers: Vec<Modifier>,
    pub one_shot_effects: Vec<OneShotEffect>,
    pub tourism_per_turn: u32,
}
```

Add `wonder_defs: Vec<WonderDef>` and `built_wonders: Vec<BuiltWonder>` to `GameState`.

```rust
pub struct BuiltWonder {
    pub wonder_id: WonderId,
    pub city_id: CityId,
    pub owner: CivId,
}
```

### Initial Wonders

| Wonder | Era | Requires | Effect |
|--------|-----|----------|--------|
| Stonehenge | Ancient | Astrology | Free Great Prophet |
| Pyramids | Ancient | Masonry | Builders get +1 charge |
| Great Library | Ancient | Writing | +2 Science, free Ancient tech |
| Colosseum | Classical | (civic) | +2 Amenities to this city and adjacent cities |
| Oracle | Ancient | Mysticism (civic) | +2 Great Person Points in all categories |

## Implementation Plan

### Step 1: Add `WonderDef` and `BuiltWonder` structs

New file `civ/wonder.rs` (or extend existing `world/wonder.rs` -- note that file is for *natural* wonders). Recommend a new file to avoid confusion.

### Step 2: Add wonder registry to `GameState`

```rust
pub wonder_defs: Vec<WonderDef>,
pub built_wonders: Vec<BuiltWonder>,
```

### Step 3: Add `RulesEngine::build_wonder()`

Validation:
- Wonder not already built globally (`built_wonders` check).
- City has required district (if any).
- Civ has researched required tech/civic.
- City doesn't already have a wonder in production queue.

On completion (in production processing of `advance_turn`):
- Push to `built_wonders`.
- Register `WonderTourism` entry.
- Apply `one_shot_effects`.
- Apply `modifiers` to the owning civ's modifier collection.
- Emit `StateDelta::WonderBuilt`.

### Step 4: Wonder modifiers in `compute_yields`

Collect modifiers from all `BuiltWonder`s owned by the civ and include them in the modifier resolution pipeline.

### Step 5: Production integration

When `ProductionItem::Wonder(wonder_id)` completes in the city production loop of `advance_turn`, execute the wonder completion logic. Add a uniqueness check: if another civ completed the same wonder this turn (or earlier), refund partial production and clear the queue item.

### Step 6: Define initial wonders

Add 5-10 wonder definitions in a `wonder_defs.rs` include file, similar to `tech_tree_def.rs`.

### Step 7: Tests

1. Queue a wonder, accumulate production, verify `WonderBuilt` delta and `BuiltWonder` registration.
2. Attempt to build an already-built wonder, verify error/refund.
3. Wonder modifiers apply to owning civ's yields.
4. Wonder tourism contributes to tourism output.
5. Wonder requires correct tech/district.

## Complexity

Medium-high. New data model, production integration, modifier wiring. ~300 lines framework + ~100 lines per wonder definition.

## Dependencies

- Production queue already supports `Wonder(WonderId)` variant.
- Tourism system already handles `WonderTourism`.
- Later-era wonders need later-era techs (Phase 5, item 12).
