# More Civilizations

Phase 5, item 13. Additional leaders and unique abilities.

## Current State

- 8 built-in civilizations: Rome, Greece, Egypt, Babylon, Germany, Japan, India, Arabia (`civ/civ_identity.rs`).
- Each has a `CivAbilityBundle` in `game/rules.rs` with: unique units, unique buildings, unique districts, unique improvements, rule overrides, and leader/civ ability modifiers.
- Civ abilities are tested in `tests/civ_abilities.rs` (13 tests).
- `BuiltinCiv` and `BuiltinLeader` enums gate unique content.
- `has_rule_override()` and `lookup_bundle()` dispatch civ-specific behavior.

## Design

Each new civilization requires:

1. **Enum variant** in `BuiltinCiv` and `BuiltinLeader`.
2. **`CivAbilityBundle`** match arm in `lookup_bundle()`.
3. **Unique ability modifiers** -- 1-2 `Modifier` structs for the civ ability and 1-2 for the leader ability.
4. **Unique unit** (optional) -- a `UnitTypeDef` with `exclusive_to` and `replaces` fields.
5. **Unique district/building/improvement** (optional) -- similarly gated.
6. **Tests** -- verify unique abilities apply correctly.

### Suggested next civilizations

| Civ | Leader | Unique Ability | Unique Unit |
|-----|--------|---------------|-------------|
| China | Qin Shi Huang | Builders get +1 charge | Crouching Tiger (ranged, no resource) |
| Aztec | Montezuma | +1 amenity per luxury type | Eagle Warrior (melee, converts kills to builders) |
| France | Catherine de Medici | +1 diplomatic visibility | Garde Impériale (melee, +10 near capital) |
| England | Victoria | +1 trade route per city on foreign continent | Redcoat (melee, +10 outside home continent) |
| Persia | Cyrus | +2 movement on declaring war | Immortal (melee with ranged defense) |
| Sumeria | Gilgamesh | Shared war experience with allies | War-Cart (heavy chariot, no resource) |

## Implementation Plan

### Per-civilization checklist

1. Add variant to `BuiltinCiv` and `BuiltinLeader` enums.
2. Add `CivAbilityBundle` match arm with:
   - `civ_ability_modifiers`: modifiers always active for this civ
   - `leader_ability_modifiers`: modifiers from leader ability
   - `unique_units`: `&[(&'static str, &'static str)]` (name, replaces)
   - `unique_buildings`, `unique_districts`, `unique_improvements`
   - `rule_overrides`: `&[RuleOverride]` for non-modifier abilities
3. Register unique `UnitTypeDef` entries in game initialization with `exclusive_to: Some(BuiltinCiv::X)`.
4. Add integration tests in `tests/civ_abilities.rs`.

### Step-by-step for one civ (e.g., China)

1. `BuiltinCiv::China`, `BuiltinLeader::QinShiHuang`
2. Civ ability "Dynastic Cycle": `Modifier { effect: YieldFlat(Culture, 1), target: AllTiles, source: CivAbility, condition: Some(AdjacentToRiver) }` (or similar flavor)
3. Leader ability: builders get +1 charge → `RuleOverride::ExtraBuilderCharge`
4. Unique unit "Crouching Tiger": ranged unit, no strategic resource cost, replaces Crossbowman
5. Tests: verify extra builder charge, verify Crouching Tiger stats

## Complexity

Medium per civ (~50-100 lines each). Mostly data definition work following established patterns. The ability system and modifier pipeline handle the heavy lifting.

## Dependencies

- Later-era units for unique unit replacements (e.g., Crouching Tiger replaces a Medieval unit that needs to exist first).
- For civs with non-modifier abilities, new `RuleOverride` variants may be needed.
