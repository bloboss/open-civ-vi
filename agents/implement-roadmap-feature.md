# Skill: Implement a Roadmap Feature

Use this guide when picking up a feature from the implementation roadmap.

## Before Starting

1. Read the roadmap: `book/src/roadmap/status.md`
2. Identify which phase and priority the feature falls under
3. Check if there are dependencies that need to be implemented first
4. Read the relevant game system page in `book/src/game-systems/` for design context

## Typical Feature Implementation Flow

### 1. Understand existing infrastructure

Most features have partial implementations. Check what already exists:

```
# Search for existing types/structs:
grep -r "YourFeature" libciv/src/

# Check for TODO comments:
grep -r "TODO.*your_feature" libciv/src/

# Check existing tests:
grep -r "your_feature" libciv/tests/
```

### 2. Plan the changes

A typical feature touches these layers (in order):

| Layer | File(s) | What to add |
|-------|---------|-------------|
| Data model | `civ/*.rs` or `world/*.rs` | New fields on existing structs |
| Diff | `game/diff.rs` | New `StateDelta` variants |
| Errors | `game/rules.rs` | New `RulesError` variants |
| Rules | `game/rules.rs` | New `RulesEngine` trait method + implementation |
| Turn phases | `game/rules.rs` (`advance_turn`) | New phase in the turn pipeline |
| Tests | `tests/*.rs` | Integration tests |
| Re-exports | `game/mod.rs`, `lib.rs` | Public API surface |

### 3. Follow the advance_turn phase ordering

If your feature adds per-turn processing, slot it into the existing phase structure in `advance_turn`:

- Phase 1: Effect queue drain (tech/civic completion effects)
- Phase 2: Per-city processing (production, growth, border expansion)
  - 2a: Research/civic progress
  - 2b: Trade route processing
  - 2c: Road maintenance
- Phase 3: Per-civ processing (yields, gold)
  - 3a: Tourism generation
  - 3b: Culture border expansion
  - 3c: Loyalty pressure
- Phase 4: Visibility recalculation
- Phase 5: Global checks
  - 5a: Great person points accumulation
  - 5b: Era score observer + era advancement
  - 5c: Victory condition evaluation

### 4. Write tests alongside implementation

Don't defer tests. Write them as you implement each piece:
- See `agents/write-integration-test.md` for the test pattern
- Cover both success paths and error paths
- Verify delta emissions, not just state mutations

## Common Patterns

### Adding fields to existing structs

Always initialize new fields in the constructor or `build_scenario()`:

```rust
// In the struct:
pub new_field: u32,

// In build_scenario() or constructors:
new_field: 0,
```

### Wiring into advance_turn

```rust
// In advance_turn, at the appropriate phase:
// ── Phase N: Your Feature ──
for civ in &mut state.civilizations {
    let delta_value = compute_something(civ);
    civ.field += delta_value;
    diff.push(StateDelta::YourDelta {
        civ: civ.id,
        amount: delta_value,
    });
}
```

### Registering new content in GameState::new()

If your feature adds registry items (victory conditions, great person defs, etc.):

```rust
// In GameState initialization:
state.your_registry.push(YourItem { ... });
```

## Quality Gates

Before considering a feature complete:

- [ ] All new public types are re-exported through `lib.rs`
- [ ] All state mutations have corresponding `StateDelta` emissions
- [ ] Integration tests cover success and error paths
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] Book page in `book/src/game-systems/` is updated if the feature changes documented behavior
- [ ] Roadmap status in `book/src/roadmap/status.md` is updated
