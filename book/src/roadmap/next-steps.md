# Next Steps

> **Last updated**: 2026-04-04

The engine has reached **full Civ VI base-game content parity**. All game content
categories (units, buildings, techs, civics, wonders, civilizations, promotions,
great people, governments, policies, city-states, resources, improvements,
districts, victory types) match or exceed the base game XML counts.

What remains is infrastructure work to fulfill the project's primary goal as an
RL training environment.

## Completed Content (verified against XML)

| Category | Count | Verified |
|---|---|---|
| Units | 89 | UnitPromotions.xml |
| Buildings | 45 | Buildings.xml (with prereq chains) |
| Great People | 177 | GreatPeople_*.xml |
| Promotions | 118 | UnitPromotions.xml |
| Technologies | 69 | Technologies.xml |
| Civics | 52 | Civics.xml |
| Governments | 10 | Governments.xml |
| Policies | 113 | Policies.xml |
| World Wonders | 29 | Buildings.xml (wonders) |
| Civilizations | 19 | Civilizations.xml |
| City-States | 25 | Civilizations.xml |
| Resources | 41 | Resources.xml |
| Improvements | 24 | Improvements.xml |
| Districts | 20 | Districts.xml |
| Natural Wonders | 15 | Features.xml |
| Victory Types | 6 | All six Civ VI types |

## Remaining Infrastructure

### 1. RL Training Harness (High Priority)

The project's primary goal is an RL environment. All prerequisites are met:
- Deterministic game engine with seeded RNG
- `apply_delta()` / `apply_diff()` for state reconstruction
- `save_game()` / `load_game()` for checkpointing
- `HeuristicAgent` for baseline/opponent play
- Full combat modifier pipeline with XP/promotions

**Implementation**:
1. `Observation` struct — fog-aware state snapshot for a single civ
2. `Action` enum — discrete actions (move, attack, build, research, etc.)
3. `CivEnv` wrapper — gym-like API: `reset()`, `step(action)`, `available_actions()`
4. Reward function — score delta, military kills, city founding, tech completion
5. Optional: PyO3 Python bindings for training frameworks

**Scope**: ~500 lines new module. See [phase6-rl-training-harness.md](phase6-rl-training-harness.md).

### 2. Replay Viewer (Medium Priority)

Foundation complete (`apply_delta`, save/load). Remaining:
1. `ReplayRecorder` — wraps `TurnEngine`, saves diffs per turn
2. `ReplayViewer` — loads saved game + diff log, steps forward/backward
3. UI integration (civsim replay subcommand or open4x-web)

**Scope**: ~200 lines.

### ~~3. Natural Wonder Discovery Event~~ — DONE

Fully wired: `recalculate_visibility()` emits `NaturalWonderDiscovered`,
`advance_turn` refreshes visibility before era score observer, discovery
awards 3 era score. 3 integration tests.

### 4. Performance Optimization (Deferred)

Profiling-first approach. Known likely hot paths: unit/city lookup (linear scan),
modifier resolution, pathfinding. Deferred until profiling data available.

See [phase6-performance.md](phase6-performance.md).

## Reference Documents

| Document | Purpose |
|---|---|
| [parity.md](parity.md) | Master Civ VI comparison (all categories at parity) |
| [parity-content.md](parity-content.md) | Detailed content reference |
| [parity-systems.md](parity-systems.md) | System-level reference |
| [parity-values.md](parity-values.md) | Yield/cost data |
| [parity-trees.md](parity-trees.md) | Tech/civic tree data |
| [diff-consolidation.md](diff-consolidation.md) | Diff gap tracking (mostly resolved) |
| [phase6-rl-training-harness.md](phase6-rl-training-harness.md) | RL API design |
| [phase6-performance.md](phase6-performance.md) | Performance notes |
