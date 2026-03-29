# Diplomatic Victory

Phase 3, item 8. Requires a diplomatic favor system.

## Current State

- Diplomacy module exists (`civ/diplomacy.rs`): `DiplomaticRelation`, `DiplomaticStatus` (Neutral/Friendly/Unfriendly/Hostile/War), `GrievanceRecord`.
- Grievance decay and status recomputation run in `advance_turn` Phase 5.
- City-state data (`civ/city_state.rs`) has `CityStateData::influence: HashMap<CivId, i32>` and suzerain tracking.
- No `diplomatic_favor` field on `Civilization`.
- No diplomatic favor accumulation logic.
- `PerCivMetWithReligionNotAtWar` condition in modifier system returns `Scale(0)` as a stub.

## Design

Simplified diplomatic victory: accumulate **diplomatic favor** each turn from city-state suzerainty, alliances, and world congress votes (simplified to just suzerainty for initial implementation). Win when favor reaches a threshold.

### Diplomatic Favor Sources (per turn)

| Source | Favor/Turn |
|--------|-----------|
| Base (all civs) | +1 |
| Per city-state suzerainty | +1 |
| Not at war with anyone | +1 |

Threshold: **100 diplomatic favor** to win.

## Implementation Plan

### Step 1: Add `diplomatic_favor` to `Civilization`

```rust
pub diplomatic_favor: u32,  // initialized to 0
```

### Step 2: Add favor accumulation in `advance_turn`

New phase between existing Phase 5 (diplomacy) and Phase 5a (great people):

```rust
// ── Phase 5-favor: Diplomatic favor accumulation ──
for civ in &mut state.civilizations {
    let mut gain = 1u32; // base
    // +1 per city-state suzerainty
    for city in &state.cities {
        if let CityKind::CityState(ref cs) = city.kind {
            if cs.is_suzerain(civ.id) { gain += 1; }
        }
    }
    // +1 if not at war with anyone
    let at_war = state.diplomatic_relations.iter().any(|r|
        (r.civ_a == civ.id || r.civ_b == civ.id)
        && r.status == DiplomaticStatus::War
    );
    if !at_war { gain += 1; }
    civ.diplomatic_favor += gain;
    diff.push(StateDelta::DiplomaticFavorChanged { civ: civ.id, amount: gain as i32, total: civ.diplomatic_favor });
}
```

Note: the loop over cities for suzerainty requires borrowing `state.cities` while iterating `state.civilizations`. Collect suzerainty counts first in an immutable pass, then apply in a mutable pass (same pattern as GP point accumulation).

### Step 3: Add `StateDelta::DiplomaticFavorChanged`

```rust
DiplomaticFavorChanged { civ: CivId, amount: i32, total: u32 },
```

### Step 4: Add `DiplomaticVictory` struct

```rust
#[derive(Debug)]
pub struct DiplomaticVictory {
    pub id: VictoryId,
    pub threshold: u32,  // default 100
}

impl VictoryCondition for DiplomaticVictory {
    fn kind(&self) -> VictoryKind { VictoryKind::ImmediateWin }
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        let favor = state.civ(civ_id)
            .map(|c| c.diplomatic_favor)
            .unwrap_or(0);
        VictoryProgress {
            victory_id: self.id, civ_id,
            current: favor, target: self.threshold,
        }
    }
}
```

### Step 5: Register and test

1. Register `DiplomaticVictory { id, threshold: 100 }` in game initialization.
2. Test: set `diplomatic_favor = 100`, verify `is_won()`.
3. Test: civ with 2 city-state suzerainties and no wars gains 4 favor/turn.
4. Test: declaring war reduces favor gain (loses the +1 peace bonus).
5. End-to-end: accumulate favor to threshold, verify `VictoryAchieved` on `advance_turn`.

## Dependencies

- City-state suzerainty (`CityStateData::suzerain`) exists but `recalculate_suzerain()` is never called during gameplay. Need to wire envoy/influence changes (Phase 5, item 15) for full functionality. For now, tests can set suzerainty directly.

## Complexity

Medium. New field, new delta, new turn phase, new victory struct. ~100 lines of new code plus tests.
