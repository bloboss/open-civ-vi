# Plan: Victory Condition Infrastructure

## Context

The game currently has a stub `VictoryCondition` trait and `VictoryProgress` struct in
`libciv/src/rules/victory.rs`, but no concrete implementations, no `GameState` integration,
and no scoring logic. `GameState` has a TODO comment (line 145 of `state.rs`) to add
`victory_conditions` and `game_over` fields. `advance_turn` has a corresponding TODO at
line 958 to evaluate victory conditions each turn.

This plan implements generic victory-condition infrastructure and a concrete score victory as
the first use case.

---

## Generic Design

Two orthogonal concerns:

### 1. Termination Trigger
A condition that can fire at any point and signal game-over. Two kinds:
- **ImmediateWin** — the triggering civ wins the instant the condition is met (e.g. Domination,
  Science, Culture). Evaluated every turn.
- **TurnLimit** — evaluated only when `state.turn >= turn_limit`; the winner is determined by
  a separate scoring mechanism (e.g. Score Victory).

### 2. Scoring Mechanism
Computes a `u32` score for a given civ from current `GameState`. Used for Score Victory display
and for breaking ties. Lives in a new `libciv/src/game/score.rs` module.

---

## File Changes

### A. `libciv/src/rules/victory.rs`

**Refactored trait** — add `&GameState` to `check_progress` (breaking change on existing
stub; no downstream code currently calls it):

```rust
pub trait VictoryCondition: std::fmt::Debug + Send + Sync {
    fn id(&self) -> VictoryId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn kind(&self) -> VictoryKind;
    /// Returns progress for the given civ. `VictoryProgress::is_won()` signals a winner.
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VictoryKind {
    /// Civ that first satisfies the condition wins immediately.
    ImmediateWin,
    /// Condition is only evaluated at `turn >= turn_limit`; scoring determines winner.
    TurnLimit { turn_limit: u32 },
}
```

**`ScoreVictory` concrete struct** (first implementation):
```rust
pub struct ScoreVictory {
    pub id: VictoryId,
    pub turn_limit: u32,   // e.g. 500
}

impl VictoryCondition for ScoreVictory {
    fn kind(&self) -> VictoryKind { VictoryKind::TurnLimit { turn_limit: self.turn_limit } }
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        VictoryProgress {
            victory_id: self.id,
            civ_id,
            current: compute_score(state, civ_id),
            target: u32::MAX,  // "target" is relative; is_won() driven by turn limit externally
        }
    }
}
```

### B. `libciv/src/game/score.rs` (new file)

Standalone scoring function; no side effects, pure query over `GameState`:

```rust
pub fn compute_score(state: &GameState, civ_id: CivId) -> u32 {
    let civ = match state.civ(civ_id) { Some(c) => c, None => return 0 };
    let cities_owned: Vec<&City> = state.cities.iter()
        .filter(|c| c.owner == civ_id && !matches!(c.kind, CityKind::CityState(_)))
        .collect();

    let city_score      = cities_owned.len() as u32 * 5;
    let pop_score       = cities_owned.iter().map(|c| c.population).sum::<u32>();
    let tech_score      = civ.researched_techs.len() as u32 * 3;
    let civic_score     = civ.completed_civics.len() as u32 * 2;
    let territory_score = cities_owned.iter()
        .map(|c| c.territory.len() as u32)
        .sum::<u32>() / 5;

    city_score + pop_score + tech_score + civic_score + territory_score
}

/// Returns a sorted Vec of (CivId, score) in descending order for all non-city-state civs.
pub fn all_scores(state: &GameState) -> Vec<(CivId, u32)> {
    let mut scores: Vec<(CivId, u32)> = state.civilizations.iter()
        .filter(|c| state.cities.iter()
            .any(|city| city.owner == c.id && !matches!(city.kind, CityKind::CityState(_))))
        .map(|c| (c.id, compute_score(state, c.id)))
        .collect();
    scores.sort_by(|a, b| b.1.cmp(&a.1));
    scores
}
```

Scoring components (dummy but representative of Civ VI score calculation):
| Component         | Formula                             |
|-------------------|-------------------------------------|
| Cities            | `cities_owned × 5`                  |
| Population        | `Σ city.population`                 |
| Technologies      | `researched_techs.len() × 3`        |
| Civics            | `completed_civics.len() × 2`        |
| Territory         | `Σ city.territory.len() / 5`        |

Wonders and great people can be added later without changing the trait.

### C. `libciv/src/game/state.rs`

Add fields to `GameState`:

```rust
pub victory_conditions: Vec<Box<dyn VictoryCondition>>,
/// Set when the game has ended. Contains the winner and the condition name.
pub game_over: Option<GameOver>,
```

Add `GameOver` struct (in `state.rs` or a new `victory.rs` re-export):
```rust
#[derive(Debug, Clone)]
pub struct GameOver {
    pub winner: CivId,
    pub condition: &'static str,
    pub turn: u32,
}
```

Initialize in `GameState::new()`:
```rust
victory_conditions: Vec::new(),
game_over: None,
```

Add `IdGenerator::next_victory_id()` (pattern follows existing `next_*` methods).

### D. `libciv/src/game/diff.rs`

Uncomment and define the victory delta:

```rust
VictoryAchieved { civ: CivId, condition: &'static str },
```

### E. `libciv/src/game/rules.rs`

Hook into `advance_turn` at the existing TODO site (line 958), before the turn counter
increment. This becomes **Phase 5b**:

```rust
// ── Phase 5b: Victory condition evaluation ────────────────────────────────
if state.game_over.is_none() {
    let civ_ids: Vec<CivId> = state.civilizations.iter()
        .map(|c| c.id)
        .collect();

    // Check ImmediateWin conditions every turn.
    'outer: for vc in &state.victory_conditions {
        if matches!(vc.kind(), VictoryKind::ImmediateWin) {
            for &civ_id in &civ_ids {
                let progress = vc.check_progress(civ_id, state);
                if progress.is_won() {
                    state.game_over = Some(GameOver {
                        winner: civ_id, condition: vc.name(), turn: state.turn,
                    });
                    diff.push(StateDelta::VictoryAchieved {
                        civ: civ_id, condition: vc.name(),
                    });
                    break 'outer;
                }
            }
        }
    }

    // Check TurnLimit conditions when the turn limit is reached.
    if state.game_over.is_none() {
        for vc in &state.victory_conditions {
            if let VictoryKind::TurnLimit { turn_limit } = vc.kind() {
                if state.turn >= turn_limit {
                    // Winner = highest-scoring civ.
                    if let Some((winner, _)) = all_scores(state).into_iter().next() {
                        state.game_over = Some(GameOver {
                            winner, condition: vc.name(), turn: state.turn,
                        });
                        diff.push(StateDelta::VictoryAchieved {
                            civ: winner, condition: vc.name(),
                        });
                    }
                }
            }
        }
    }
}
```

### F. `libciv/src/game/mod.rs`

Add `pub mod score;` and re-export `compute_score`, `all_scores`, `GameOver`.

### G. `libciv/src/lib.rs`

Re-export `compute_score`, `all_scores`, `GameOver` so `civsim` can use them.

### H. `civsim/src/main.rs`

1. Add `scores` command to the interactive `play` loop. Displays a table of all civs sorted
   by score:
   ```
   +-- Scores ----------------------------------------
   | # | Civilization      |  Score |
   |---|-------------------|--------|
   | 1 | Rome              |    142 |
   | 2 | Babylon           |     87 |
   +--------------------------------------------------
   ```
2. After each `advance_turn`, check `state.game_over` and print a winner banner if set.
3. In `print_turn_header`, add the player's current score to the status line.
4. In `run_ai_demo`, print the final score table after simulation ends.

---

## Module Re-export Map

```
libciv/src/rules/victory.rs    ← VictoryCondition trait, VictoryKind, VictoryProgress, ScoreVictory
libciv/src/game/score.rs       ← compute_score(), all_scores()   (new)
libciv/src/game/state.rs       ← GameState + GameOver struct
libciv/src/game/diff.rs        ← StateDelta::VictoryAchieved
libciv/src/game/rules.rs       ← Phase 5b hook in advance_turn
libciv/src/game/mod.rs         ← pub mod score; re-exports
libciv/src/lib.rs              ← top-level re-exports
civsim/src/main.rs             ← scores command + game-over display
```

---

## Testing

1. **Unit tests in `score.rs`** — `test_compute_score_empty_civ` (0), `test_compute_score_cities_and_techs`.
2. **Integration test in `libciv/tests/gameplay.rs`** (or new `victory.rs` test file):
   - Build scenario with 2 civs; attach a `ScoreVictory { turn_limit: 3 }`.
   - Advance 3 turns; assert `state.game_over.is_some()`.
   - Assert `StateDelta::VictoryAchieved` is present in the diff.
   - Assert the winner matches the civ with the higher computed score.
3. **`cargo test --workspace`** must pass with zero warnings (`cargo clippy -- -D warnings`).
4. **Manual**: `cargo run -p open4x -- play`, type `scores`, verify table renders; advance past
   turn limit, verify winner banner appears.

---

## Out of Scope (Future)

- `DominationVictory`, `ScienceVictory`, `CultureVictory`, `DiplomaticVictory` — concrete
  types will follow the same `VictoryCondition` trait; add to `victory.rs` when those systems
  (wonders, tourism, diplomacy score) are implemented.
- Score caching / incremental updates — plain `Vec` scan is fine for the current game size.
- Wonders, great works, great people in score formula — deferred until those systems are live.
