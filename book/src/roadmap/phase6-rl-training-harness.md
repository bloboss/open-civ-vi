# RL Training Harness

Phase 6, item 18. Structured observation space and reward shaping.

## Current State

- Deterministic AI agent exists (`ai/deterministic.rs`): `Agent` trait + `HeuristicAgent` with exploration/production heuristics. 9 integration tests.
- `GameStateDiff` provides structured per-turn observations (50+ delta variants).
- `compute_score()` in `game/score.rs` provides a scalar game evaluation.
- Deterministic `IdGenerator` with seeded RNG ensures reproducibility.
- No gym-like API, no tensor observation space, no reward function.

## Design

The RL harness wraps the game engine in a **step-based interface** compatible with standard RL frameworks (Gymnasium/PettingZoo conventions).

### Interface

```rust
pub struct CivEnv {
    state: GameState,
    rules: DefaultRulesEngine,
    agent_civ: CivId,
    opponent_policy: Box<dyn Agent>,  // heuristic or self-play
}

impl CivEnv {
    pub fn reset(&mut self, seed: u64) -> Observation;
    pub fn step(&mut self, action: Action) -> (Observation, f64, bool, Info);
    pub fn available_actions(&self) -> Vec<Action>;
}
```

### Observation Space

Structured observation extracted from `GameState`:

```rust
pub struct Observation {
    // Global
    pub turn: u32,
    pub era: u8,

    // Per-civ (self)
    pub gold: i32,
    pub faith: u32,
    pub science_per_turn: i32,
    pub culture_per_turn: i32,
    pub num_cities: u32,
    pub num_units: u32,
    pub score: u32,
    pub techs_researched: u32,
    pub great_person_points: Vec<(GreatPersonType, u32)>,

    // Per-city (owned cities)
    pub cities: Vec<CityObs>,

    // Visible map (sparse)
    pub visible_tiles: Vec<TileObs>,

    // Diplomacy
    pub relations: Vec<(CivId, DiplomaticStatus)>,

    // Victory progress
    pub victory_progress: Vec<VictoryProgress>,
}
```

### Action Space

Discrete actions with parameters:

```rust
pub enum Action {
    EndTurn,
    MoveUnit { unit: UnitId, to: HexCoord },
    Attack { attacker: UnitId, target: UnitId },
    FoundCity { settler: UnitId },
    StartProduction { city: CityId, item: ProductionItem },
    ResearchTech { tech: TechId },
    ResearchCivic { civic: CivicId },
    AssignPolicy { policy: PolicyId },
    SendEnvoy { city_state: CivId },
    // ... other actions
}
```

### Reward Shaping

```rust
pub fn compute_reward(prev: &GameState, curr: &GameState, civ: CivId) -> f64 {
    let mut reward = 0.0;

    // Score delta
    let score_delta = compute_score(curr, civ) as f64 - compute_score(prev, civ) as f64;
    reward += score_delta * 0.01;

    // City founded
    let prev_cities = prev.cities.iter().filter(|c| c.owner == civ).count();
    let curr_cities = curr.cities.iter().filter(|c| c.owner == civ).count();
    reward += (curr_cities as f64 - prev_cities as f64) * 1.0;

    // Tech researched
    // Victory achieved: large positive
    // Defeat: large negative

    reward
}
```

## Implementation Plan

### Step 1: Define `Observation` and `Action` types

New module `libciv/src/rl/` with `observation.rs`, `action.rs`, `env.rs`, `reward.rs`.

### Step 2: Implement observation extraction

`fn observe(state: &GameState, civ_id: CivId) -> Observation` -- extract structured data from game state, respecting fog-of-war (only include `visible_tiles`).

### Step 3: Implement action dispatch

`fn dispatch(env: &mut CivEnv, action: Action) -> Result<GameStateDiff, RulesError>` -- map each `Action` variant to the corresponding `RulesEngine` method.

### Step 4: Implement `available_actions()`

Enumerate valid actions for the current state. This is computationally expensive for large action spaces -- consider hierarchical action selection (choose unit, then choose action for unit).

### Step 5: Implement reward function

Configurable reward shaping with per-component weights. Default: score delta + victory progress bonuses.

### Step 6: Python bindings (optional)

Use PyO3 to expose `CivEnv` as a Python class compatible with Gymnasium:

```python
env = CivEnv(seed=42, num_civs=4)
obs = env.reset()
obs, reward, done, info = env.step(action)
```

### Step 7: Tests

1. Reset environment, verify initial observation matches game state.
2. Step with valid action, verify observation updates.
3. Step with invalid action, verify error handling.
4. Play a full game to completion, verify terminal observation and reward.
5. Verify determinism: same seed + same action sequence = same observations.

## Complexity

High. New module, observation/action space design, reward engineering. The game engine does the heavy lifting, but the RL interface requires careful design for training efficiency.

## Dependencies

- **Soft**: Diff consolidation (Phase 1) -- diff-based observations are more efficient than full-state extraction.
- **Soft**: Save/load (Phase 6, item 16) -- for checkpointing during training.
- **Soft**: All victory conditions implemented -- for meaningful reward signals.
