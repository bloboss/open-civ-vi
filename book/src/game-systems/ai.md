# AI Agents

## Agent Trait

```rust
trait Agent {
    fn take_turn(&self, state: &mut GameState, rules: &dyn RulesEngine) -> GameStateDiff;
}
```

All AI agents implement the `Agent` trait. The `take_turn()` method receives full game state and returns a diff of all actions taken.

## HeuristicAgent

The built-in `HeuristicAgent` is a deterministic, rule-based AI:

```rust
struct HeuristicAgent {
    civ_id: CivId,
}
```

### Production Decisions

When a city's production queue is empty, the agent selects a unit to produce:
- Prefers combat units
- Sorts by production cost (cheapest first)
- Ties broken alphabetically by name

The agent does not double-queue -- it checks if the queue is already populated before adding.

### Movement Decisions

Units move using a tile-scoring gradient descent:

| Tile State | Score |
|------------|-------|
| Unexplored | 100 |
| Explored but not visible | 50 |
| Currently visible | 10 |

The agent moves each unit toward the highest-scoring adjacent tile, prioritizing exploration of unknown territory.

### Determinism

Given the same seed and game state, the agent produces identical results. This is verified by integration tests that run the same scenario twice and compare outputs.

### Multi-Turn Behavior

The agent runs stably over many turns without panicking. Integration tests verify:
- Both agents can act on the first turn
- Warriors explore over multiple turns (don't get stuck)
- The agent runs 100+ turns without errors

### Server Integration

In multiplayer games, the server runs `HeuristicAgent` instances for AI civilizations. After all human players submit their turns, the server calls `take_turn()` for each AI agent before resolving the turn.

## Future AI Work

The agent infrastructure is designed to support more sophisticated agents:
- The `Agent` trait allows swapping in learned policies (e.g., RL-trained agents)
- `GameStateDiff` output enables reward shaping for training
- The deterministic simulation ensures reproducible training episodes
- No game state is hidden from the agent (it receives the full `GameState`)
