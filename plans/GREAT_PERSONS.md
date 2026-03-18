# Great Persons Implementation Plan

## Goal

Implement a set of dummy great persons for four classes -- Great General
(Land Combat), Great Admiral (Naval Combat), Great Engineer (Engineering),
and Great Merchant (Trade) -- with concrete retire-on-use abilities.
Military great persons (General/Admiral) are implemented first because
their abilities are localized combat bonuses and one-time retirement
effects; culture/science/religious special buildings are out of scope.

## Existing infrastructure

| Layer | What exists today |
|-------|-------------------|
| `GreatPerson` struct | `id`, `name`, `person_type`, `era`, `owner`, `coord`, `ability_names`, `is_retired` |
| `GreatPersonAbility` trait | `name()`, `description()`, `uses()` |
| `GreatPersonType` enum | General, Admiral, Engineer, Merchant, Musician, Artist, Writer, Prophet, Scientist |
| `UnitCategory::GreatPerson` | Already in the enum |
| `GameState.great_people` | `Vec<GreatPerson>` -- storage exists, unused |
| `StateDelta` | No great-person-specific deltas yet |
| `RulesEngine` trait | No `retire_great_person` method yet |
| `OneShotEffect` | `GrantModifier`, `FreeUnit` patterns usable for retirement effects |

## Dummy great persons (one per class)

### 1. Great General -- "Sun Tzu" (Ancient era)

**Retire ability:** All land military units owned by the player permanently
gain +5 combat strength. The great person is consumed.

Implementation: on retirement, push a `Modifier` with
`source = Custom("Sun Tzu")`, `target = UnitDomain(Land)`,
`effect = CombatStrengthFlat(5)`, `stacking = Additive` into the
owning civilization's modifier list. Remove the great person unit from
`state.units`, mark `is_retired = true`.

### 2. Great Admiral -- "Themistocles" (Ancient era)

**Retire ability:** All naval military units owned by the player permanently
gain +5 combat strength. The great person is consumed.

Same pattern as Sun Tzu but `target = UnitDomain(Sea)`.

### 3. Great Engineer -- "Imhotep" (Ancient era)

**Retire ability:** Instantly add 200 production to the nearest city's
current production queue item (reduces remaining cost). The great person
is consumed.

Implementation: find the city closest to the great person's coord (must
be owned by the same civ). Apply a one-time production burst:
`city.production_accumulated += 200` (capped at item cost so it does not
overflow). Emit a `GreatPersonRetired` delta and a `ProductionBurst`
delta.

### 4. Great Merchant -- "Marco Polo" (Ancient era)

**Retire ability:** Instantly grant 200 gold to the owning civilization.
The great person is consumed.

Implementation: `civ.gold += 200`. Emit `GreatPersonRetired` delta and
`GoldChanged` delta.

## Implementation steps

### Step 1: Data model extensions

**File: `libciv/src/game/diff.rs`**

Add new `StateDelta` variants:
- `GreatPersonRetired { great_person: GreatPersonId, owner: CivId }`
- `ProductionBurst { city: CityId, amount: u32 }`

**File: `libciv/src/civ/great_people.rs`**

Add a `GreatPersonDef` struct (static template, analogous to
`UnitTypeDef`):
- `person_type: GreatPersonType`
- `name: &'static str`
- `era: &'static str`
- `retire_effect: RetireEffect`

Add `RetireEffect` enum:
- `CombatStrengthBonus { domain: UnitDomain, bonus: i32 }` -- permanent modifier
- `ProductionBurst { amount: u32 }` -- one-time production to nearest city
- `GoldGrant { amount: u32 }` -- one-time gold to owning civ

Add a `fn builtin_great_persons() -> Vec<GreatPersonDef>` returning the
four dummy persons.

**File: `libciv/src/civ/civilization.rs`**

Add `great_person_modifiers: Vec<Modifier>` field to `Civilization`.
This stores permanent modifiers granted by retired great persons.

**File: `libciv/src/game/state.rs`**

Add `great_person_defs: Vec<GreatPersonDef>` to `GameState` (registry,
like `unit_type_defs`).

Add `fn great_person(&self, id: GreatPersonId) -> Option<&GreatPerson>`
and `fn great_person_mut(...)` lookup helpers.

### Step 2: `RulesEngine::retire_great_person`

**File: `libciv/src/game/rules.rs`**

Add to `RulesEngine` trait:
```rust
fn retire_great_person(
    &self,
    state: &mut GameState,
    great_person_id: GreatPersonId,
) -> Result<GameStateDiff, RulesError>;
```

Add `RulesError` variants:
- `GreatPersonNotFound`
- `GreatPersonAlreadyRetired`
- `NotYourGreatPerson` (owner check)

**`DefaultRulesEngine` implementation:**

1. Find the `GreatPerson` by ID; error if missing or already retired.
2. Look up the matching `GreatPersonDef` by name from `state.great_person_defs`.
3. Match on `RetireEffect`:
   - `CombatStrengthBonus { domain, bonus }`:
     Push a `Modifier` to the owning civ's `great_person_modifiers`.
   - `ProductionBurst { amount }`:
     Find nearest owned city to the GP's coord.
     Add `amount` to `city.production_accumulated` (capped at current item cost).
   - `GoldGrant { amount }`:
     Add `amount` to the civ's `gold`.
4. Mark `great_person.is_retired = true`.
5. Remove the corresponding unit from `state.units`.
6. Emit `GreatPersonRetired` delta + effect-specific delta.
7. Return diff.

### Step 3: Wire modifiers into combat

**File: `libciv/src/game/rules.rs` (attack method)**

In the combat strength computation, collect the attacking/defending
civ's `great_person_modifiers` and apply
`CombatStrengthFlat` effects filtered by `UnitDomain` matching the
unit's domain.

### Step 4: Great person spawning helper

**File: `libciv/src/game/state.rs` or `libciv/src/civ/great_people.rs`**

Add `fn spawn_great_person(state: &mut GameState, civ_id: CivId,
def_name: &str, coord: HexCoord) -> GreatPersonId` that:
1. Creates a `GreatPerson` from the matching `GreatPersonDef`.
2. Adds it to `state.great_people` with `owner = Some(civ_id)`.
3. Creates a `BasicUnit` with `category = GreatPerson`, `domain` based
   on person type (Admiral = Sea, others = Land), no combat strength.
4. Adds the unit to `state.units`.
5. Returns the ID.

### Step 5: Integration tests

**File: `libciv/tests/great_persons.rs`**

Tests (each uses `build_scenario()` + spawning helpers):

1. `test_retire_great_general_grants_land_combat_bonus` --
   Spawn Sun Tzu, retire, verify Rome warrior's effective CS increases by 5.
2. `test_retire_great_admiral_grants_naval_combat_bonus` --
   Spawn Themistocles, retire, verify naval units get +5 CS.
3. `test_retire_great_engineer_adds_production` --
   Spawn Imhotep near Rome's capital, retire, verify production_accumulated increased.
4. `test_retire_great_merchant_grants_gold` --
   Spawn Marco Polo, retire, verify civ gold increased by 200.
5. `test_retire_already_retired_fails` --
   Retire Sun Tzu twice; second call returns `GreatPersonAlreadyRetired`.
6. `test_great_person_combat_modifier_applies_in_battle` --
   Retire Sun Tzu, then attack; verify attacker damage reflects +5 CS.

## Out of scope

- Great person point accumulation per turn
- Recruitment/patronage UI flow
- Culture/science/religious great persons (Writer, Artist, Musician,
  Scientist, Prophet) -- these require special buildings not yet implemented
- Era-based great person pools
- AI great person usage
