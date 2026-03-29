# Natural Wonder Discovery

Phase 2, item 4. One-time yield/era-score bonus when a civ first sights a natural wonder.

## Current State

- 5 built-in natural wonders with yield bonuses and appeal: Krakatoa, Grand Mesa, Cliffs of Dover, Uluru, Galapagos Islands (`world/wonder.rs`).
- Tile placement and yield calculation work (`WorldTile::natural_wonder`).
- Visibility system (`game/visibility.rs`) tracks `visible_tiles` and `explored_tiles` per civ; `recalculate_visibility()` returns `TilesRevealed` deltas for newly explored tiles.
- Era score observer (`civ/historic_moments.rs`) already awards points for certain `StateDelta` variants.
- No `NaturalWonderDiscovered` delta variant exists. No discovery event fires.

## Implementation Plan

### Step 1: Add `StateDelta::NaturalWonderDiscovered`

In `game/diff.rs`, add:

```rust
NaturalWonderDiscovered { civ: CivId, wonder_name: &'static str, coord: HexCoord },
```

### Step 2: Emit discovery in visibility recalculation

In `game/visibility.rs`, inside `recalculate_visibility()`, after computing newly explored tiles:

```rust
for &coord in &newly_explored {
    if let Some(tile) = board.tile(coord)
        && tile.natural_wonder.is_some()
    {
        let wonder_name = tile.natural_wonder.as_ref().unwrap().as_def().name();
        diff.push(StateDelta::NaturalWonderDiscovered {
            civ: civ_id,
            wonder_name,
            coord,
        });
    }
}
```

This requires `recalculate_visibility` to accept a mutable `GameStateDiff` (it already returns one -- just add the new deltas to it).

### Step 3: Wire into era score observer

In `civ/historic_moments.rs`, add a `HistoricMomentKind::NaturalWonderDiscovered` variant and a match arm in `observe_deltas`:

```rust
StateDelta::NaturalWonderDiscovered { civ, .. } => {
    Some((*civ, HistoricMomentKind::NaturalWonderDiscovered))
}
```

Configure with era score value of 1 (non-unique, each wonder discovery counts).

### Step 4: One-time yield bonus (optional)

Grant an immediate bonus (e.g. +1 era score already from Step 3, plus optionally a one-shot science or culture bonus). This would use the existing `effect_queue` or a direct state mutation + delta.

### Step 5: Tests

1. Place a natural wonder on the map. Move a unit into vision range. Verify `NaturalWonderDiscovered` delta is emitted.
2. Verify the delta is only emitted once per civ per wonder (subsequent visibility recalculations for the same tile don't re-emit).
3. Verify era score increases from the historic moment.

## Complexity

Low. Touches 3 files (`diff.rs`, `visibility.rs`, `historic_moments.rs`) with ~20 lines of new code plus tests.
