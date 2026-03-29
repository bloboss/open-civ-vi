# Skill: Add a New RulesEngine Method

Use this guide when implementing a new game action on the `RulesEngine` trait.

## Pattern

Every `RulesEngine` method follows the same structure:

1. **Trait signature** in `libciv/src/game/rules.rs` on the `RulesEngine` trait
2. **Implementation** on `DefaultRulesEngine` in the same file
3. **New `RulesError` variant(s)** if new failure modes exist (same file, `enum RulesError`)
4. **New `StateDelta` variant(s)** in `libciv/src/game/diff.rs`
5. **Integration test(s)** in `libciv/tests/`

## Step-by-step

### 1. Add the trait method

```rust
// In the `RulesEngine` trait block:
fn your_action(
    &self,
    state: &mut GameState,
    // ... parameters with typed IDs (CivId, UnitId, CityId, etc.)
) -> Result<GameStateDiff, RulesError>;
```

Key conventions:
- First param is always `&self`
- Second param is always `state: &mut GameState` (or `&GameState` for read-only queries like `move_unit` and `compute_yields`)
- Use typed ID newtypes, not raw integers
- Return `Result<GameStateDiff, RulesError>`

### 2. Add StateDelta variants

In `libciv/src/game/diff.rs`, add variants to the `StateDelta` enum:

```rust
YourActionPerformed { actor: CivId, target: SomeId, amount: u32 },
```

Use specific, descriptive variant names. Each field should use typed IDs.

### 3. Add RulesError variants

In `libciv/src/game/rules.rs`, add to `enum RulesError`:

```rust
/// Clear doc comment explaining when this error occurs.
YourSpecificError,
```

### 4. Implement on DefaultRulesEngine

Follow the validate-mutate-emit pattern:

```rust
fn your_action(
    &self,
    state: &mut GameState,
    some_id: SomeId,
) -> Result<GameStateDiff, RulesError> {
    // ── Phase 1: Validation ──
    let entity = state.entities.iter()
        .find(|e| e.id == some_id)
        .ok_or(RulesError::EntityNotFound)?;

    if !entity.can_do_thing() {
        return Err(RulesError::CannotDoThing);
    }

    // ── Phase 2: Mutation ──
    let mut diff = GameStateDiff::new();

    // Mutate state directly
    let entity = state.entities.iter_mut()
        .find(|e| e.id == some_id)
        .unwrap(); // safe: validated above
    entity.field = new_value;

    // ── Phase 3: Emit deltas ──
    diff.push(StateDelta::YourActionPerformed {
        actor: entity.owner,
        target: some_id,
        amount: new_value,
    });

    Ok(diff)
}
```

### 5. Write integration tests

Create tests in `libciv/tests/` using the `Scenario` pattern:

```rust
mod common;
use common::build_scenario;

#[test]
fn test_your_action_succeeds() {
    let mut s = build_scenario();
    let rules = libciv::game::rules::DefaultRulesEngine;

    let diff = rules.your_action(&mut s.state, s.rome_civ)
        .expect("should succeed");

    assert!(diff.deltas.iter().any(|d| matches!(d,
        StateDelta::YourActionPerformed { .. }
    )));
}

#[test]
fn test_your_action_fails_on_invalid_input() {
    let mut s = build_scenario();
    let rules = libciv::game::rules::DefaultRulesEngine;

    let err = rules.your_action(&mut s.state, invalid_id)
        .expect_err("should fail");
    assert!(matches!(err, RulesError::EntityNotFound));
}
```

### 6. Re-export if needed

If the new method introduces public types, re-export them through `libciv/src/game/mod.rs` and `libciv/src/lib.rs`.

## Checklist

- [ ] Trait method added to `RulesEngine`
- [ ] `StateDelta` variant(s) added to `diff.rs`
- [ ] `RulesError` variant(s) added if needed
- [ ] `DefaultRulesEngine` implementation follows validate-mutate-emit
- [ ] Integration tests cover success and all error paths
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
