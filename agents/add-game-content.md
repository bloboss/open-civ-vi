# Skill: Add New Game Content

Use this guide when adding new civilizations, unit types, buildings, improvements, or other game content.

## General Principles

- Built-in content uses **plain enums** with `match` dispatch (terrain, features, resources, improvements)
- Content that needs extensibility uses **trait objects** with `as_def()` wrappers (roads, edge features, natural wonders)
- All built-in names use `&'static str`, never `String`
- Content is registered in `GameState` registries (e.g., `unit_type_defs`, `building_defs`)

## Adding a New Civilization

### Files to modify

1. `libciv/src/civ/civilization.rs` -- if new leader abilities or agenda traits are needed
2. `libciv/tests/common/mod.rs` -- if the civ should be available in test scenarios
3. Integration test file in `libciv/tests/`

### Pattern

A civilization needs:
- A `Civilization` struct instance with unique `CivId`
- A `Leader` with `Vec<Box<dyn LeaderAbility>>` and `Box<dyn Agenda>`
- Optionally: unique unit types, unique improvements, unique districts

```rust
// Leader ability (implement the trait):
#[derive(Debug)]
struct YourLeaderAbility;

impl LeaderAbility for YourLeaderAbility {
    fn name(&self) -> &'static str { "Ability Name" }
    fn description(&self) -> &'static str { "What it does" }
    fn modifiers(&self) -> Vec<Modifier> {
        vec![Modifier::new(
            ModifierSource::Leader("Leader Name"),
            TargetSelector::AllUnits,
            EffectType::CombatStrengthFlat(5),
            StackingRule::Additive,
        )]
    }
}
```

Important: `Leader` and `Civilization` contain `Box<dyn Trait>` fields and do **not** derive `Clone`.

## Adding a New Unit Type

### Files to modify

1. `libciv/src/game/state.rs` -- register the `UnitTypeDef` in game initialization
2. `libciv/tests/common/mod.rs` -- add to test scenario if needed

### Pattern

```rust
let unit_type_id = state.id_gen.next_unit_type_id();
state.unit_type_defs.push(UnitTypeDef {
    id: unit_type_id,
    name: "Swordsman",
    domain: UnitDomain::Land,
    category: UnitCategory::Combat,
    combat_strength: Some(36),
    ranged_strength: None,
    range: 0,
    max_movement: 200,  // 2 movement points (scaled by 100)
    production_cost: 90,
    can_found_city: false,
    max_charges: 0,     // only builders have charges
    vision_range: 2,
    required_tech: Some("Iron Working"),
    required_resource: None,
    upgrade_from: Some("Warrior"),
    // ... unique unit fields if applicable
});
```

## Adding a New Improvement

### Files to modify

1. `libciv/src/world/improvement.rs` -- add variant to `BuiltinImprovement` enum
2. Implement `name()`, `yield_bonus()`, `build_turns()`, `requirements()` in the match arms

### Pattern

```rust
// In the BuiltinImprovement enum:
YourImprovement,

// In each match arm:
Self::YourImprovement => ImprovementRequirements {
    requires_land: true,
    requires_water: false,
    elevation: ElevationReq::NotMountain,
    blocked_terrains: &[BuiltinTerrain::Snow],
    required_feature: None,
    conditional_features: &[],
    required_resource: None,
    required_tech: Some("Some Tech"),
    required_civic: None,
    proximity: None,
},
```

## Adding a New StateDelta Variant

When your content produces observable state changes:

1. Add the variant to `StateDelta` in `libciv/src/game/diff.rs`
2. Use typed IDs for all fields
3. Emit the delta adjacent to the state mutation

## Modifier Integration

If your content grants numeric bonuses, use the `Modifier` system:

```rust
Modifier::new(
    ModifierSource::Building("Library"),     // source for debugging
    TargetSelector::Civilization(civ_id),    // who it affects
    EffectType::YieldFlat(YieldType::Science, 2), // +2 science
    StackingRule::Additive,                  // stacks with other bonuses
)
```

Modifiers are collected at query time by `compute_yields()` -- never stored directly on cities or tiles.

## Checklist

- [ ] Content uses `&'static str` for names (not `String`)
- [ ] Registered in appropriate `GameState` registry
- [ ] Modifiers use the `Modifier` pipeline (not direct state mutation)
- [ ] Integration tests verify the content works
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
