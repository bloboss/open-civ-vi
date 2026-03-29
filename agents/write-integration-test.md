# Skill: Write Integration Tests

Use this guide when writing integration tests for libciv game mechanics.

## Test Infrastructure

All integration tests live in `libciv/tests/`. They share a common test fixture defined in `libciv/tests/common/mod.rs`.

### The Scenario Pattern

```rust
mod common;
use common::build_scenario;

#[test]
fn test_something() {
    let mut s = build_scenario();
    // s.state: GameState -- fully initialized game world
    // s.rome_civ: CivId
    // s.babylon_civ: CivId
    // s.rome_capital: CityId
    // s.babylon_capital: CityId
    // s.rome_warrior: UnitId
    // s.babylon_warrior: UnitId
    // s.warrior_type: UnitTypeId
    // s.settler_type: UnitTypeId
    // s.builder_type: UnitTypeId
}
```

The scenario creates a deterministic 14x8 board (seed 42) with:
- **Rome**: capital at (3,3), warrior at (5,3)
- **Babylon**: capital at (10,5), warrior at (8,5)
- Three registered unit types: warrior, settler, builder

### Using DefaultRulesEngine

```rust
let rules = libciv::game::rules::DefaultRulesEngine;

// Call any RulesEngine method:
let diff = rules.move_unit(&s.state, s.rome_warrior, target_coord)
    .expect("move should succeed");

// Apply move deltas to state:
s.apply_move(&diff);
```

### Advancing Turns

```rust
// Process one full turn (advance_turn + reset movement + recalculate visibility):
s.advance_turn();
```

### Asserting on Diffs

```rust
// Check that a specific delta was emitted:
assert!(diff.deltas.iter().any(|d| matches!(d,
    StateDelta::UnitMoved { unit, .. } if *unit == s.rome_warrior
)));

// Check that a delta was NOT emitted:
assert!(!diff.deltas.iter().any(|d| matches!(d,
    StateDelta::UnitDestroyed { .. }
)));
```

### Asserting on State

```rust
// Check unit position:
let unit = s.state.unit(s.rome_warrior).unwrap();
assert_eq!(unit.coord, target_coord);

// Check city population:
let city = s.state.city(s.rome_capital).unwrap();
assert_eq!(city.population, 2);

// Check civilization gold:
let civ = s.state.civ(s.rome_civ).unwrap();
assert!(civ.gold >= 100);
```

## Conventions

- **One assertion focus per test** -- test names should describe the single behavior being verified
- **Test names** use snake_case and describe the behavior: `wall_defense_bonus_reduces_damage_to_defender`
- **Determinism** -- all tests must be deterministic; the scenario uses seed 42
- **No test utilities outside `common/`** -- keep shared setup in `common/mod.rs`
- **Terrain setup** -- if your test needs specific terrain, modify tiles directly:

```rust
use libciv::world::terrain::BuiltinTerrain;
let tile = s.state.board.tile_mut(coord).unwrap();
tile.terrain = BuiltinTerrain::Desert;
tile.hills = true;
```

## Adding a New Test File

1. Create `libciv/tests/your_feature.rs`
2. Add `mod common;` at the top
3. Import what you need from `libciv`
4. Run with `cargo test --test your_feature`

## Checklist

- [ ] Tests use `build_scenario()` from `common/mod.rs`
- [ ] Each test has a clear, descriptive name
- [ ] Both success and error paths are tested
- [ ] Delta emissions are verified (not just state mutations)
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
