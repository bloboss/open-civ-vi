# TurnEngine Diff Consolidation

Phase 1 of the roadmap calls for fixing diff composition so all turn phases contribute to a single coherent diff. This page describes the problem, the specific gaps, and the implementation plan.

## Problem Statement

The semantic diff system (`GameStateDiff` / `StateDelta`) is designed so that every state change in a turn is captured as structured data -- enabling replay, RL observation, and incremental UI updates. Currently, the system falls short in two ways:

1. **The top-level diff is discarded.** `TurnEngine::process_turn` calls `rules.advance_turn(state)`, which builds a diff across all turn phases, but the return value is assigned to `_diff` and replaced with an empty `GameStateDiff::new()`. Any caller of `process_turn` receives no information about what changed.

2. **Several turn phases mutate state without emitting deltas.** Even the diff that `advance_turn` *does* build is incomplete -- some phases modify `GameState` fields directly without pushing a corresponding `StateDelta`. An observer replaying deltas would end up with a state that doesn't match the actual game.

Together, these mean the diff system cannot yet fulfill its stated purpose of supporting replay or RL observation.

## Specific Gaps

### Critical: Diff discarded at TurnEngine level

In `game/turn.rs`, `TurnEngine::process_turn`:

```rust
let _diff = rules.advance_turn(state);
// Phase 2: apply diff to state, collect AI decisions, etc.
GameStateDiff::new()
```

The diff from `advance_turn` is thrown away. The fix is to return it.

### Missing deltas in advance_turn phases

The following state mutations inside `advance_turn` (in `game/rules.rs`) have no corresponding delta:

| Phase | Mutation | Expected delta |
|-------|----------|----------------|
| Population growth | `auto_assign_citizen()` pushes to `city.worked_tiles` | `CitizenAssigned` |
| Trade route movement | Trader `movement_left` reset to max | (new variant or existing movement delta) |
| Trade route failure | `u.trade_origin = None`, `u.trade_destination = None` cleared | (new variant, e.g. `TradeRouteCleared`) |
| Unique unit healing | `unit.health = (unit.health + 10).min(100)` | (new variant, e.g. `UnitHealed`) |
| Tourism output | `civ.tourism_output` and `civ.domestic_culture` updated | (new variant or extend existing tourism deltas) |
| City rebellion aftermath | `city.loyalty` set to 50 or 25 after revolt | Already has `LoyaltyChanged` variant -- just needs to be emitted |

### Fragile manual delta extraction

Trade route autonomous movement uses a pattern where `move_unit()` is called, its deltas are inspected, state is manually mutated to match, and then the deltas are appended:

```rust
for delta in &move_deltas {
    if let StateDelta::UnitMoved { unit, to, cost, .. } = delta {
        u.coord = *to;
        u.movement_left = u.movement_left.saturating_sub(*cost);
    }
}
diff.deltas.extend(move_deltas);
```

This dual-path (mutate state *and* record delta separately) risks divergence if either side is updated without the other.

## Implementation Plan

### Step 1: Forward the diff from TurnEngine

Change `TurnEngine::process_turn` to return the diff from `advance_turn` instead of discarding it. This is a one-line fix that unblocks all downstream consumers.

### Step 2: Add missing StateDelta variants

Add new variants to the `StateDelta` enum where no existing variant covers the gap:

- `UnitHealed { unit: UnitId, old_health: u32, new_health: u32 }` -- for passive healing (unique units, fortification, etc.)
- `TradeRouteCleared { unit: UnitId }` -- for when a trader's route assignment is removed on failure

The remaining gaps (`CitizenAssigned`, `LoyaltyChanged`) already have variants defined -- they just need to be emitted.

### Step 3: Emit deltas for each gap

For each mutation listed in the gaps table above, add a `diff.push(StateDelta::...)` call adjacent to the state mutation. Specifics:

1. **auto_assign_citizen** -- either modify the helper to accept `&mut GameStateDiff` and push `CitizenAssigned`, or have it return the assigned coord so the caller can push the delta.
2. **Trader movement reset** -- emit a `UnitMoved` or similar delta when resetting movement points.
3. **Trade route failure** -- emit `TradeRouteCleared` when clearing origin/destination.
4. **Unique unit healing** -- emit `UnitHealed` with old and new health values.
5. **Tourism output** -- extend the existing `TourismGenerated` usage or add a new delta for domestic culture and tourism output updates.
6. **City rebellion aftermath** -- emit `LoyaltyChanged` when setting loyalty to 50 or 25 post-revolt.

### Step 4: Add integration tests

Add tests that verify the diff returned from `process_turn` contains the expected deltas for each phase:

- A turn with population growth produces `PopulationGrew` *and* `CitizenAssigned`.
- A turn with a trader in transit produces `UnitMoved` deltas for the trader.
- A turn with a unique unit below max health produces `UnitHealed`.
- A turn with a city at zero loyalty produces `CityRevolted` *and* `LoyaltyChanged`.

### Step 5 (future): Apply-from-diff pattern

A longer-term improvement would be to invert the current pattern: instead of mutating state and *also* recording a delta, apply state changes *from* the delta. This eliminates the divergence risk entirely. Sketch:

```rust
let delta = StateDelta::UnitMoved { unit, from, to, cost };
apply_delta(state, &delta);
diff.push(delta);
```

This is a larger refactor and not required for the initial consolidation, but it's the natural end-state for the diff architecture.

## Success Criteria

After this work is complete:

- `TurnEngine::process_turn` returns a non-empty `GameStateDiff` capturing every state change that occurred during the turn.
- Replaying the returned deltas against the pre-turn state produces an identical post-turn state (modulo field ordering).
- The era score observer (`observe_deltas`) sees a complete set of deltas, so no historic moments are missed due to gaps.
- RL agents can use the diff as a complete observation of what happened each turn.
