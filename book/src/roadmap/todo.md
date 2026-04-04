# Remaining TODOs

Code-level TODOs extracted from the codebase, plus potential future work.

## Code TODOs

### Gameplay

| Location | Tag | Description | Priority |
|---|---|---|---|
| `rules/tech.rs:15` | GAMEPLAY | Eureka condition triggers — automate eureka checking in advance_turn | Medium |
| `game/rules/city.rs:105` | GAMEPLAY | Rome ability: build road to capital on city founding | Low |
| `game/rules/turn_phase.rs:801` | GAMEPLAY | City-state territory expansion (distinct from player civ) | Low |
| `game/rules/mod.rs:467` | GAMEPLAY | `purchase_tile()` — spend gold to claim a tile immediately | Low |
| `civ/trade.rs:42` | GAMEPLAY | Trade route food/production delivery for domestic routes | Low |

### Architecture

| Location | Tag | Description | Priority |
|---|---|---|---|
| `civ/diplomacy.rs:43` | ARCH | Grievance visibility should be arranged in levels | Low |
| `civ/diplomacy.rs:71` | ARCH | Consider bi-directional graph for diplomatic relations | Low |
| `civ/era.rs:133` | ARCH | Era dedication modifiers (field exists but unpopulated) | Low |

### Cleanup

| Location | Tag | Description | Priority |
|---|---|---|---|
| `rules/victory.rs:26` | CLEANUP | Stale TODO comment about trait-based victory (now enum-based) | Cleanup |

## Future Work

### Performance Optimization
- Profile before optimizing — no changes without data
- Candidate hot paths: unit/city linear scan, modifier resolution, pathfinding
- Consider HashMap indexes for units-by-id, cities-by-owner
- Criterion benchmarks not yet added

### PyO3 Python Bindings
- Expose `CivEnv` as Python class for Gymnasium/PettingZoo
- Enable direct RL training with PyTorch/JAX frameworks
- Requires PyO3 dependency + `#[pyclass]` annotations

### Multiplayer Polish
- Game room matchmaking and lobby
- Spectator mode
- Turn timer enforcement
- Per-player fog-of-war projection

### WASM Frontend
- City management panel
- Unit action overlay with movement preview
- Minimap with territory colors
- Production queue drag-and-drop
