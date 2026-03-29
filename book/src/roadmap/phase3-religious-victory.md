# Religious Victory

Phase 3, item 7. Win when your religion is the majority in all other civs' cities.

## Current State

Religion is fully implemented:

- `Religion` struct with `holy_city`, `beliefs`, `founded_by` (`civ/religion.rs`).
- 24 integration tests covering founding, beliefs, spread, pressure, theological combat.
- `City::religious_followers: HashMap<ReligionId, u32>` tracks per-city follower counts.
- `City::majority_religion()` returns the religion with the most followers.
- Religious pressure propagation runs in `advance_turn` Phase 3d.
- `RulesEngine::spread_religion()` for Missionary/Apostle spread with charge consumption.
- `RulesEngine::theological_combat()` for religious unit combat.

## Implementation Plan

### Step 1: Add `ReligiousVictory` struct

In `game/victory.rs`:

```rust
#[derive(Debug)]
pub struct ReligiousVictory {
    pub id: VictoryId,
}

impl VictoryCondition for ReligiousVictory {
    fn kind(&self) -> VictoryKind { VictoryKind::ImmediateWin }
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        let civ = match state.civ(civ_id) {
            Some(c) => c,
            None => return VictoryProgress { victory_id: self.id, civ_id, current: 0, target: 1 },
        };
        let religion_id = match civ.founded_religion {
            Some(r) => r,
            None => return VictoryProgress { victory_id: self.id, civ_id, current: 0, target: 1 },
        };

        // Count other civs where our religion is majority in more than half their cities.
        let other_civs: Vec<CivId> = state.civilizations.iter()
            .filter(|c| c.id != civ_id)
            .map(|c| c.id)
            .collect();
        let target = other_civs.len() as u32;

        let converted = other_civs.iter().filter(|&&other_civ| {
            let their_cities: Vec<_> = state.cities.iter()
                .filter(|c| c.owner == other_civ)
                .collect();
            if their_cities.is_empty() { return false; }
            let majority_count = their_cities.iter()
                .filter(|c| c.majority_religion() == Some(religion_id))
                .count();
            majority_count > their_cities.len() / 2
        }).count() as u32;

        VictoryProgress {
            victory_id: self.id, civ_id,
            current: converted,
            target: if target == 0 { 1 } else { target },
        }
    }
}
```

### Step 2: Register in `GameState`

Push `ReligiousVictory { id }` into `victory_conditions` during game initialization.

### Step 3: Tests

1. Found a religion for civ A. Set majority followers in all of civ B's cities. Verify `check_progress().is_won()`.
2. Civ with no founded religion always returns `current: 0`.
3. Partial conversion (majority in some but not all of another civ's cities) returns correct progress.
4. End-to-end: spread religion until majority, run `advance_turn`, verify `VictoryAchieved`.

## Complexity

Low. Religion system is complete. This is a single struct implementing `VictoryCondition` plus registration and tests. ~50 lines of new code.
