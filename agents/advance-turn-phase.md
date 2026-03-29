# Skill: Add a New advance_turn Phase

Use this guide when adding per-turn processing for a game system.

## Context

`DefaultRulesEngine::advance_turn()` in `libciv/src/game/rules.rs` processes all per-turn game logic. It follows a strict phase ordering where each phase can depend on results from earlier phases.

## Current Phase Structure

```
Phase 1:   Effect queue drain (tech/civic unlocks)
Phase 2:   Per-city processing
  Phase 2a:  Research & civic progress
  Phase 2b:  Trade route autonomous movement & expiry
  Phase 2c:  Road maintenance deduction
  Phase 2d:  Population growth & citizen assignment
  Phase 2e:  Production completion
Phase 3:   Per-civ processing
  Phase 3a:  Tourism generation
  Phase 3b:  Culture border expansion
  Phase 3c:  Loyalty pressure & city revolt
Phase 4:   Visibility recalculation
Phase 5:   Global checks
  Phase 5a:  Great person point accumulation
  Phase 5b:  Era score observer + era advancement
  Phase 5c:  Victory condition evaluation
Phase 6:   Turn counter increment
```

## Adding a New Phase

### 1. Choose the right position

- **Per-city processing** (Phase 2): resource consumption, building effects, district yields
- **Per-civ processing** (Phase 3): civ-wide bonuses, diplomatic effects
- **Global checks** (Phase 5): cross-civ interactions, win conditions

### 2. Implement with delta emission

Every state mutation must have a corresponding `StateDelta`:

```rust
// ── Phase Nx: Your Feature Description ──
{
    // Collect data first (avoid borrow conflicts)
    let updates: Vec<(CivId, u32)> = state.civilizations.iter()
        .map(|civ| {
            let value = compute_something(civ, &state.cities);
            (civ.id, value)
        })
        .collect();

    // Then mutate
    for (civ_id, value) in updates {
        let civ = state.civilizations.iter_mut()
            .find(|c| c.id == civ_id)
            .unwrap();
        civ.field += value;
        diff.push(StateDelta::YourDelta { civ: civ_id, amount: value });
    }
}
```

### 3. Handle borrow checker patterns

The most common challenge is needing to read from `state` while mutating it. Use the collect-then-mutate pattern:

```rust
// BAD: simultaneous borrow
for civ in &mut state.civilizations {
    let info = state.cities.iter().filter(|c| c.owner == civ.id); // borrow conflict!
}

// GOOD: collect first, then mutate
let civ_data: Vec<_> = state.civilizations.iter()
    .map(|civ| (civ.id, /* read from state */))
    .collect();

for (civ_id, data) in civ_data {
    let civ = state.civilizations.iter_mut()
        .find(|c| c.id == civ_id).unwrap();
    // mutate civ using data
}
```

### 4. Add phase comment markers

Follow the existing convention of marking phases with comment banners:

```rust
// ── Phase 3d: Your Feature ─────────────────────────────────────────────
```

## Testing

Test advance_turn phases by:
1. Setting up state that triggers the phase behavior
2. Calling `s.advance_turn()` (from the Scenario helper)
3. Checking the post-turn state
4. Optionally checking delta emissions via `rules.advance_turn(&mut s.state)`

```rust
#[test]
fn your_feature_triggers_on_advance_turn() {
    let mut s = build_scenario();
    // Set up preconditions...

    let rules = DefaultRulesEngine;
    let diff = rules.advance_turn(&mut s.state);

    // Verify state changed
    assert_eq!(s.state.civilizations[0].field, expected);

    // Verify delta emitted
    assert!(diff.deltas.iter().any(|d| matches!(d,
        StateDelta::YourDelta { .. }
    )));
}
```
