# Science Victory

Phase 3, item 6. Define milestone techs and a progress tracker.

## Current State

- `VictoryCondition` trait in `game/victory.rs` with `check_progress(civ_id, state) -> VictoryProgress`.
- `VictoryKind::ImmediateWin` for conditions checked every turn.
- Three victories implemented: `ScoreVictory`, `CultureVictory`, `DominationVictory`.
- Victory evaluation runs in `advance_turn` Phase 5c.
- Tech tree currently only has Ancient era techs (12 nodes). Science victory milestones require later-era techs that don't exist yet.

## Design

Science victory uses a **milestone chain**: three project completions, each gated by a late-game tech. The first civ to complete all three wins.

### Milestones

| # | Milestone | Required Tech | Era |
|---|-----------|--------------|-----|
| 1 | Launch Earth Satellite | Rocketry | Modern |
| 2 | Launch Moon Landing | Satellites | Atomic |
| 3 | Launch Mars Colony | Nuclear Fusion | Information |

### Data model

Add to `Civilization`:

```rust
pub science_milestones_completed: u32,  // 0..3
```

Each milestone is a `ProductionItem::Project(&'static str)` queued in a city with a Spaceport district (or the existing `IndustrialZone` as a stand-in until Spaceport is added). On completion, increment `science_milestones_completed` and emit a new `StateDelta::ScienceMilestoneCompleted`.

### Implementation Plan

#### Step 1: Add `ScienceVictory` struct

In `game/victory.rs`:

```rust
#[derive(Debug)]
pub struct ScienceVictory {
    pub id: VictoryId,
}

impl VictoryCondition for ScienceVictory {
    fn kind(&self) -> VictoryKind { VictoryKind::ImmediateWin }
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        let completed = state.civ(civ_id)
            .map(|c| c.science_milestones_completed)
            .unwrap_or(0);
        VictoryProgress {
            victory_id: self.id, civ_id,
            current: completed, target: 3,
        }
    }
}
```

#### Step 2: Add `science_milestones_completed` field

Add `pub science_milestones_completed: u32` to `Civilization`, initialized to 0.

#### Step 3: Add `StateDelta::ScienceMilestoneCompleted`

```rust
ScienceMilestoneCompleted { civ: CivId, milestone: u32 },
```

#### Step 4: Milestone completion logic

Option A (simple, no later-era tech dependency): Add a `RulesEngine::complete_science_milestone(state, civ_id)` action that validates `science_milestones_completed < 3`, increments, emits delta. Gate each milestone on a specific researched tech.

Option B (production-based): Add `ProductionItem::Project(&'static str)` variant and treat milestones as city production items with high cost. This better mirrors Civ VI but requires more plumbing.

**Recommendation**: Start with Option A to unblock the victory type, then upgrade to Option B when later-era techs exist.

#### Step 5: Register in `GameState`

In game initialization, push `ScienceVictory { id }` into `victory_conditions`.

#### Step 6: Tests

1. Set `science_milestones_completed = 3` on a civ, verify `check_progress().is_won()`.
2. Set to 2, verify `!is_won()` and `current == 2, target == 3`.
3. End-to-end: call `complete_science_milestone` three times, verify `VictoryAchieved` delta emitted on `advance_turn`.

## Dependencies

- **Hard blocker for production-based milestones**: Later-era tech tree (Phase 5, item 12) for Rocketry, Satellites, Nuclear Fusion.
- **No blocker for the victory check itself**: can be implemented now with `complete_science_milestone()` action.
