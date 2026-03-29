# Replay Viewer

Phase 6, item 17. Reconstruct games from diff sequences.

## Current State

- `GameStateDiff` contains `Vec<StateDelta>` with 50+ delta variants covering all game state changes.
- `advance_turn` returns a `GameStateDiff` (though `TurnEngine::process_turn` currently discards it -- see [diff consolidation plan](./diff-consolidation.md)).
- All `RulesEngine` actions return `GameStateDiff`.
- No delta application function exists (state is mutated directly, deltas are emitted as records).
- No replay storage or playback system exists.

## Design

A replay is a sequence of per-turn diffs plus the initial game state (or just the seed + config to regenerate it). The viewer steps through turns, applying deltas to reconstruct each game state.

### Architecture

```
ReplayRecorder -> Vec<TurnRecord>
TurnRecord { turn: u32, diff: GameStateDiff }

ReplayViewer {
    initial_state: GameState,
    turns: Vec<TurnRecord>,
    current_turn: usize,
}
```

### Core requirement: `apply_delta()`

A function that applies a single `StateDelta` to a `GameState`:

```rust
fn apply_delta(state: &mut GameState, delta: &StateDelta);
```

This is the inverse of the current pattern where state is mutated and a delta is emitted. The diff consolidation plan (Phase 1) notes this as a future improvement.

## Implementation Plan

### Step 1: Complete diff consolidation (prerequisite)

Ensure `TurnEngine::process_turn` returns complete diffs with no gaps. See [diff consolidation plan](./diff-consolidation.md).

### Step 2: Implement `apply_delta()`

Large match statement over all `StateDelta` variants:

```rust
fn apply_delta(state: &mut GameState, delta: &StateDelta) {
    match delta {
        StateDelta::UnitMoved { unit, to, cost, .. } => {
            if let Some(u) = state.units.iter_mut().find(|u| u.id == *unit) {
                u.coord = *to;
                u.movement_left = u.movement_left.saturating_sub(*cost);
            }
        }
        StateDelta::TechResearched { civ, tech, .. } => {
            if let Some(c) = state.civilizations.iter_mut().find(|c| c.id == *civ) {
                c.researched_techs.push(*tech);
            }
        }
        // ... 50+ variants
    }
}
```

### Step 3: `ReplayRecorder`

Collect `GameStateDiff` from each `process_turn` call:

```rust
pub struct ReplayRecorder {
    pub turns: Vec<TurnRecord>,
}

impl ReplayRecorder {
    pub fn record_turn(&mut self, turn: u32, diff: GameStateDiff) {
        self.turns.push(TurnRecord { turn, diff });
    }
}
```

### Step 4: `ReplayViewer`

```rust
pub struct ReplayViewer {
    initial_state: GameState,  // requires save/load (Phase 6, item 16)
    turns: Vec<TurnRecord>,
    current_state: GameState,
    current_turn: usize,
}

impl ReplayViewer {
    pub fn step_forward(&mut self) { ... }
    pub fn step_backward(&mut self) { ... }  // requires re-applying from initial
    pub fn jump_to_turn(&mut self, turn: usize) { ... }
}
```

### Step 5: Serializable replay format

Combine with save/load (Phase 6, item 16) to persist replays:

```rust
pub struct ReplayFile {
    pub seed: u64,
    pub config: GameConfig,
    pub turns: Vec<TurnRecord>,
}
```

### Step 6: Tests

1. Record a 10-turn game, replay from initial state, verify final states match.
2. Step forward and backward, verify state consistency at each turn.
3. Verify replay determinism: same seed + config produces identical replay.

## Complexity

High. `apply_delta()` is the largest single function (~50+ match arms). Testing requires end-to-end state comparison. Backward stepping is expensive (replay from start).

## Dependencies

- **Hard**: Diff consolidation (Phase 1) -- diffs must be complete.
- **Hard**: Save/load (Phase 6, item 16) -- initial state must be serializable for replays.
- **Soft**: All state mutations must emit deltas (ongoing requirement).
