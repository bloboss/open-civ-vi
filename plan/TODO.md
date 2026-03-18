# Open Civ VI — Implementation TODO

> Generated from ARCHITECTURE.md §8. Status assessed 2026-03-18.

Legend: ✅ Done · 🔶 Partial · ❌ Missing

---

## Overall Status

| § | Feature | Status |
|---|---------|--------|
| 8.1 | Coherent Map Generation | ✅ Done |
| 8.3 | City Defenses & Ranged Attacks | 🔶 Partial |
| 8.4 | Trade Routes & Trader Units | ✅ Done (core) |
| 8.5 | Religion System | ❌ Stub only |
| 8.6 | Culture Borders | ✅ Done |
| 8.6 | Loyalty & Tourism | ❌ Missing |
| 8.8 | Road Placement | ✅ Done |
| 8.9 | Builder Charges | ✅ Done |
| 8.10 | Great People System | ❌ Stub only |
| 8.11 | Era Score & Age System | ❌ Stub only |
| 8.12 | Governor System | 🔶 Partial |
| 8.13 | Victory Condition Evaluation | 🔶 Partial (score only) |
| 8.14 | Natural Wonder Discovery | ❌ Stub only |
| 8.15 | TurnEngine Consolidation | ❌ Broken |

---

## §8.1 — Coherent Map Generation ✅

Six-phase pipeline fully implemented in `libciv/src/world/mapgen/`:
continents → zones → features → rivers → resources → starting positions.
Tests in `libciv/tests/mapgen.rs`.

**No remaining work.**

---

## §8.3 — City Defenses and Ranged Attacks 🔶

### What exists today

| Component | File | Lines | Notes |
|-----------|------|-------|-------|
| `WallLevel` enum | `civ/city.rs` | 34–40 | None / Ancient / Medieval / Renaissance |
| `WallLevel::defense_bonus()` | `civ/city.rs` | 128–137 | Returns 0 / 3 / 5 / 8 |
| `WallLevel::max_hp()` | `civ/city.rs` | 139–147 | Returns 0 / 50 / 100 / 200 |
| `City.walls` + `City.wall_hp` | `civ/city.rs` | 67–68 | Initialized to `None` / 0 |
| `attack()` | `game/rules.rs` | 1181–1268 | Unit-vs-unit only; no wall awareness |
| `AttackType::CityAssault` | `game/diff.rs` | 14 | TODO stub (commented) |
| `terrain_defense_bonus()` | `world/tile.rs` | 50–78 | Hills +3, Forest +3, Marsh −2 |
| Damage formula | `game/rules.rs` | 1222–1236 | `30 · exp((cs_atk − cs_def) / 25) · rng` |

### What's missing

The wall defense bonus is defined but **never read** — `attack()` only applies `terrain_defense_bonus()` to the defender's tile. There is no concept of "attacking a city" vs attacking a unit in the open. No city-initiated ranged fire exists. No siege bonus exists.

### Implementation plan — 4 tasks

---

#### Task 1: Wall defense bonus in unit-vs-unit combat

**Goal:** When a defending unit is standing on a city tile that has walls, the wall's `defense_bonus()` is added to the defender's effective combat strength.

**Files:** `game/rules.rs`

**Modification in `attack()` (around line 1216–1220):**

```rust
// Current code:
let terrain_def_bonus = state.board
    .tile(def_coord)
    .map(|t| t.terrain_defense_bonus())
    .unwrap_or(0);
let effective_def_cs = (def_cs as i32 + terrain_def_bonus).max(1) as u32;

// New code:
let terrain_def_bonus = state.board
    .tile(def_coord)
    .map(|t| t.terrain_defense_bonus())
    .unwrap_or(0);
let wall_def_bonus = state.cities.iter()
    .find(|c| c.coord == def_coord)
    .map(|c| c.walls.defense_bonus())
    .unwrap_or(0);
let effective_def_cs = (def_cs as i32 + terrain_def_bonus + wall_def_bonus).max(1) as u32;
```

No new types, no new `StateDelta` variants. Pure additive change inside the existing damage formula.

**Tests (`libciv/tests/gameplay.rs`):**
- `wall_defense_bonus_increases_effective_strength` — place a defender on a city tile with `Ancient` walls, attack with equal-strength unit, assert defender takes less damage than without walls.

---

#### Task 2: Wall HP damage and destruction (melee attacks on cities)

**Goal:** When a melee unit attacks a unit standing on a walled city, a portion of the damage also applies to `city.wall_hp`. When wall HP reaches 0, the wall tier downgrades.

**Files:** `game/diff.rs`, `game/rules.rs`

**New `StateDelta` variants (`game/diff.rs`):**
```rust
/// City walls took damage from an attack.
WallDamaged { city: CityId, damage: u32, hp_remaining: u32 },
/// City walls were destroyed (HP reached 0); tier downgraded.
WallDestroyed { city: CityId, previous_level: WallLevel },
```

**Modification in `attack()` (after defender damage applied, ~line 1248–1253):**

When `attack_type == Melee` and the defender's coord matches a city with walls:
1. Compute wall damage = `def_damage / 2` (walls absorb splash from melee)
2. Apply `city.wall_hp = city.wall_hp.saturating_sub(wall_damage)`
3. Emit `WallDamaged { city, damage, hp_remaining }`
4. If `city.wall_hp == 0` and `city.walls != WallLevel::None`:
   - Save `previous_level = city.walls`
   - Set `city.walls = WallLevel::None` (walls breached)
   - Set `city.wall_hp = 0`
   - Emit `WallDestroyed { city, previous_level }`

**Design choice:** Walls don't downgrade tier-by-tier (Ancient → None). Once breached, they're gone. This matches Civ VI where walls are either up or destroyed, not gradually reduced through tiers. Rebuilding requires a new production item.

**Tests:**
- `melee_attack_damages_city_walls` — attack a unit on a walled city, assert `WallDamaged` delta emitted and `wall_hp` decreased.
- `wall_destruction_when_hp_reaches_zero` — set `wall_hp = 1`, attack, assert `WallDestroyed` emitted and `city.walls == WallLevel::None`.

---

#### Task 3: City ranged attack

**Goal:** Cities with walls can fire a ranged attack at one enemy unit per turn. This is a player/AI-triggered action (not automatic in `advance_turn`).

**Files:** `game/rules.rs` (trait + impl), `game/diff.rs`

**New `AttackType` variant (`game/diff.rs:14`):**
```rust
// Uncomment and activate the existing stub:
CityBombard,   // was: CityAssault
```

**New `RulesError` variant:**
```rust
/// City has no walls and cannot perform a ranged attack.
CityCannotAttack,
/// City has already attacked this turn.
CityAlreadyAttacked,
```

**New field on `City` (`civ/city.rs`):**
```rust
pub has_attacked_this_turn: bool,   // reset to false at start of advance_turn
```

**New trait method (`game/rules.rs` RulesEngine trait):**
```rust
/// City with walls fires a ranged attack at an enemy unit within range 2.
/// Requires walls (WallLevel != None). Each city may fire once per turn.
/// City ranged attacks deal damage but never take counter-damage.
fn city_bombard(
    &self,
    state: &mut GameState,
    city_id: CityId,
    target: UnitId,
) -> Result<GameStateDiff, RulesError>;
```

**Implementation (`DefaultRulesEngine`):**

```rust
fn city_bombard(&self, state: &mut GameState, city_id: CityId, target: UnitId)
    -> Result<GameStateDiff, RulesError>
{
    // 1. Validate city exists and has walls
    let city_idx = state.cities.iter().position(|c| c.id == city_id)
        .ok_or(RulesError::CityNotFound)?;
    let city = &state.cities[city_idx];
    if city.walls == WallLevel::None {
        return Err(RulesError::CityCannotAttack);
    }
    if city.has_attacked_this_turn {
        return Err(RulesError::CityAlreadyAttacked);
    }
    let city_coord = city.coord;
    let city_owner = city.owner;

    // 2. City ranged strength = 15 + wall_defense_bonus (Ancient=18, Med=20, Ren=23)
    let city_cs = 15_u32 + city.walls.defense_bonus() as u32;

    // 3. Validate target unit exists, is enemy, and within range 2
    let (def_coord, def_cs) = {
        let u = state.unit(target).ok_or(RulesError::UnitNotFound)?;
        if u.owner == city_owner { return Err(RulesError::SameCivilization); }
        (u.coord, u.combat_strength.unwrap_or(0))
    };
    let dist = city_coord.distance(&def_coord);
    if dist > 2 { return Err(RulesError::NotInRange); }

    // 4. Damage formula (same exponential, no terrain bonus for city offense)
    let rng = 0.75 + state.id_gen.next_f32() * 0.5;
    let damage = (30.0_f32
        * f32::exp((city_cs as f32 - def_cs as f32) / 25.0)
        * rng) as u32;

    // 5. Apply damage to target (no counter-damage to city)
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::UnitAttacked {
        attacker: UnitId::nil(),   // sentinel: city, not a unit
        defender: target,
        attack_type: AttackType::CityBombard,
        attacker_damage: 0,
        defender_damage: damage,
    });
    if let Some(u) = state.unit_mut(target) {
        u.health = u.health.saturating_sub(damage);
        if u.health == 0 {
            diff.push(StateDelta::UnitDestroyed { unit: target });
        }
    }
    state.units.retain(|u| u.health > 0);

    // 6. Mark city as having attacked
    state.cities[city_idx].has_attacked_this_turn = true;

    Ok(diff)
}
```

**Reset in `advance_turn` (beginning of the method):**
```rust
for city in &mut state.cities {
    city.has_attacked_this_turn = false;
}
```

**Note on `UnitId::nil()` as attacker:** The `UnitAttacked` delta currently requires a `UnitId` for the attacker. Since the attacker is a city, not a unit, we use `UnitId::nil()` as a sentinel. An alternative is to add a new `StateDelta::CityBombarded { city, target, damage }` variant — this is cleaner but adds more delta variants. Either approach works; the sentinel is simpler for now.

**Tests:**
- `city_bombard_deals_damage_no_counter` — city with Ancient walls fires at adjacent enemy, assert damage dealt and 0 attacker damage.
- `city_bombard_requires_walls` — city with `WallLevel::None` returns `CityCannotAttack`.
- `city_bombard_range_check` — target at distance 3 returns `NotInRange`.
- `city_bombard_once_per_turn` — second bombard same turn returns `CityAlreadyAttacked`; after `advance_turn`, can fire again.

---

#### Task 4: Siege unit bonus vs cities

**Goal:** Siege units (Catapult, Bombard) get a combat strength bonus when attacking units garrisoned in cities.

**Files:** `game/state.rs` (UnitTypeDef), `game/rules.rs` (attack)

**New field on `UnitTypeDef` (`game/state.rs`):**
```rust
/// Bonus combat strength when attacking a unit on a city tile. 0 for non-siege units.
pub siege_bonus: u32,
```

**Register siege unit types (wherever unit types are built, likely `civsim` or `game/state.rs`):**
```rust
UnitTypeDef {
    name: "Catapult",
    combat_strength: Some(23),
    range: 2,
    siege_bonus: 10,
    ..
}
```

**Modification in `attack()` (around line 1222, before damage formula):**
```rust
// Look up the attacker's UnitTypeDef to check for siege bonus
let siege_bonus = state.unit_type_defs.iter()
    .find(|d| d.id == state.unit(attacker_id).map(|u| u.unit_type).unwrap_or_default())
    .map(|d| d.siege_bonus)
    .unwrap_or(0);
let is_city_tile = state.cities.iter().any(|c| c.coord == def_coord);
let effective_atk_cs = if is_city_tile { atk_cs + siege_bonus } else { atk_cs };

// Then use effective_atk_cs in the damage formula instead of atk_cs
```

**Tests:**
- `siege_unit_bonus_applies_on_city_tile` — Catapult attacks a unit on a city, assert more damage than a non-siege ranged unit with same base strength.
- `siege_bonus_not_applied_in_open_field` — Catapult attacks a unit NOT on a city, assert same damage as equivalent non-siege unit.

---

### Summary — modification surface

| File | Changes |
|------|---------|
| `libciv/src/game/diff.rs` | Add `WallDamaged`, `WallDestroyed` deltas; uncomment `CityBombard` attack type |
| `libciv/src/game/rules.rs` | Modify `attack()` for wall bonus + wall damage; add `city_bombard()` trait method + impl; reset `has_attacked_this_turn` in `advance_turn` |
| `libciv/src/civ/city.rs` | Add `has_attacked_this_turn: bool` field to `City`; initialize in `City::new()` |
| `libciv/src/game/state.rs` | Add `siege_bonus: u32` field to `UnitTypeDef` |
| `libciv/tests/gameplay.rs` | ~8 new tests covering all four tasks |

### Dependencies

- Task 1 (wall defense bonus) has no dependencies — can be done first.
- Task 2 (wall HP damage) depends on Task 1 conceptually but not in code.
- Task 3 (city bombard) depends on Task 2 only for `WallDamaged`/`WallDestroyed` deltas; otherwise independent.
- Task 4 (siege bonus) is fully independent of Tasks 2–3.
- Tasks 1 and 4 can be done in parallel. Tasks 2 and 3 can be done in parallel after delta types are defined.

---

## §8.4 — Trade Routes and Trader Units ✅ (core)

Core lifecycle complete. Deferred edge cases:

1. **Per-city food/production delivery** — `compute_route_yields()` currently gold-only; add food/production contributions for domestic routes
2. **Max trade route capacity** — gate additional slots on Commercial Hub district count per civ; enforce in `establish_trade_route()`
3. **Route cancellation on city capture** — `CityCaptured` delta not yet emitted; when emitted, drain routes whose origin or destination matches the captured city
4. **Trader respawn on route expiry** — when a route expires in phase 2b, spawn a new Trader unit at origin city

---

## §8.5 — Religion System ❌

All data structures defined. No mechanics implemented.

### Remaining tasks (dependency order)

1. **Great Prophet unit type**
   - Add to `UnitTypeDef` registry (`category: UnitCategory::Religious`)

2. **`RulesEngine::found_religion(state, great_prophet_id, name, beliefs) -> Result<GameStateDiff, RulesError>`**
   - Validates Great Prophet unit, sets `Religion { holy_city, beliefs, founded_by }`, removes unit
   - Emit new `StateDelta::ReligionFounded { civ, religion_id, city }`

3. **Missionary and Apostle unit types**
   - Add to `UnitTypeDef` registry
   - Add `RulesEngine::spread_religion(state, unit_id, target_city_id)`

4. **Per-city religion pressure**
   - Add `religion_pressure: HashMap<ReligionId, u32>` to `City`
   - `advance_turn` phase: accumulate pressure from adjacent cities, holy cities, missionaries

5. **Majority-religion tracking**
   - Helper `city_majority_religion(city) -> Option<ReligionId>`
   - Wire into `compute_yields` for `Belief` modifier integration

6. **Belief modifier integration**
   - `Belief` implementations return `Vec<Modifier>`; include in `compute_yields` modifier collection

---

## §8.6 — Loyalty and Tourism ❌

Cultural border expansion is done. Two subsystems remain.

### Loyalty

1. **Add `loyalty: i32` field to `City`** (range 0–100, starting at 50)

2. **Loyalty pressure computation** (`game/rules.rs`)
   - Per-turn pressure from: adjacent city culture output (weighted by distance), amenities surplus, governor establishment bonus
   - Pressure from foreign civs decays loyalty toward them

3. **Wire loyalty into `advance_turn`** (new phase after culture borders)
   - Adjust `city.loyalty` by net pressure
   - At `loyalty == 0`: emit `StateDelta::CityRevolted { city }`, flip `city.ownership` to `Occupied` or transfer to highest-pressure civ

4. **`StateDelta` variants**: `LoyaltyChanged { city, delta, new_value }`, `CityRevolted { city, new_owner }`

### Tourism

1. **Tourism sources**
   - Wonders: add `tourism_output: u32` to wonder definitions
   - National parks: new improvement type with tourism yield
   - Great Works: new `GreatWork` struct attached to buildings/districts

2. **`compute_yields` tourism field** — aggregate sources into `YieldBundle::tourism`

3. **Culture Victory check** (`game/victory.rs`)
   - New `CultureVictory` implementation: won when a civ's cumulative tourism exceeds every other civ's total home culture

---

## §8.8 — Road Placement ✅

Implemented. `RulesEngine::place_road()` validates builder unit, tech requirements,
ownership, land-only, and upgrade path (no downgrades). Road maintenance is deducted
in `advance_turn()` Phase 2c. `StateDelta::RoadPlaced` emitted on success.

---

## §8.9 — Builder Charges ✅

Implemented. `BasicUnit.charges: Option<u8>` tracks remaining charges.
`UnitTypeDef.max_charges: u8` controls initial charge count (3 for builders, 0 for others).
Both `place_improvement()` and `place_road()` decrement charges and destroy the builder
at zero via `decrement_builder_charges()`. `StateDelta::ChargesChanged` and
`StateDelta::UnitDestroyed` emitted as appropriate.

---

## §8.10 — Great People System ❌

Data structures exist (`GreatPerson`, `GreatPersonAbility` trait, `GameState.great_people`). No accumulation or recruitment.

### Remaining tasks

1. **`great_person_points: HashMap<GreatPersonType, u32>` on `Civilization`**

2. **Points accumulation in `advance_turn`**
   - Per civ: for each district, add points to the corresponding `GreatPersonType` bucket
     (Campus → Scientist, Theatre Square → Artist/Musician/Writer, etc.)

3. **Great person candidate pool**
   - Static registry of `GreatPersonDef` structs with era gating
   - Available candidates filtered by `current_era`

4. **`RulesEngine::recruit_great_person(state, civ_id, great_person_def_id) -> Result<GameStateDiff, RulesError>`**
   - Deduct accumulated points, spawn unit, emit `StateDelta::GreatPersonRecruited`

5. **Activated ability effects**
   - Each `GreatPersonAbility` implementation processes a one-shot effect (eureka, combat bonus, etc.)
   - `RulesEngine::activate_great_person_ability(state, unit_id, ability_idx)`

---

## §8.11 — Era Score and Age System ❌

`AgeType`, `Era` struct, and `Civilization::current_era` defined. No scoring or age transitions.

### Remaining tasks

1. **`era_score: u32` field on `Civilization`**

2. **Historic moment triggers**
   - Define `HistoricMoment` enum (e.g. `FirstCity`, `FirstTech`, `WonderBuilt`, `CityCaptured`, …)
   - Each `StateDelta` variant that constitutes a historic moment awards era score
   - Wire into `advance_turn` effect-drain phase

3. **Era transition detection**
   - Track tech/civic completion counts against era thresholds (from `Era::tech_count`, `Era::civic_count`)
   - When threshold crossed, determine Normal/Golden/Dark age by comparing `era_score` to target

4. **Age modifier application**
   - Dark age: yield penalty modifiers applied via `Modifier` system
   - Golden age: yield bonus modifiers
   - Heroic age (dark → golden): stacked bonuses
   - Modifiers collected in `compute_yields`

5. **`StateDelta` variants**: `EraAdvanced { civ, new_era }`, `AgeAssigned { civ, age_type }`

---

## §8.12 — Governor System 🔶

Struct, 7 built-in definitions, and `is_established()` defined. Not wired into gameplay.

### Remaining tasks

1. **Add `governors: Vec<Governor>` to `GameState`** (`game/state.rs`)

2. **Fix governor ID generation** — current code uses `Ulid::nil()` for all governors (bug in `governor.rs:55`); use `IdGenerator`

3. **`RulesEngine::assign_governor(state, civ_id, governor_name, city_id) -> Result<GameStateDiff, RulesError>`**
   - Set `governor.assigned_city = Some(city_id)`, reset `turns_to_establish`
   - Emit `StateDelta::GovernorAssigned { civ, city, governor_name }`

4. **Establishment timer in `advance_turn`**
   - For each governor with `assigned_city.is_some()` and `turns_to_establish > 0`, decrement
   - Emit `StateDelta::GovernorEstablished { city, governor_name }` when it reaches 0

5. **Governor modifiers in `compute_yields`**
   - When governor `is_established()`, include their `GovernorDef::modifiers()` in modifier collection for that city

---

## §8.13 — Victory Condition Evaluation 🔶

`ScoreVictory` and evaluation loop in `advance_turn` phase 5b are implemented. Four victory types missing.

### Remaining tasks

1. **`DominationVictory`** (`game/victory.rs`)
   - Won when a civ controls the original capital of every other civ
   - `check_progress`: count captured original capitals / total other civs

2. **`ScienceVictory`**
   - Milestone chain: Launch Earth Satellite → Land on Moon → Establish Mars Colony
   - Each milestone requires specific projects/wonders (simplification: just tech prerequisites)
   - `check_progress`: count completed milestones / 3

3. **`CultureVictory`** (depends on §8.6 Tourism)
   - Won when civ's cumulative tourism > every other civ's domestic culture
   - `check_progress`: count civs surpassed / total other civs

4. **`DiplomaticVictory`**
   - Won by accumulating Diplomatic Favor and winning World Congress vote
   - Simplified: win when `diplomatic_favor >= threshold` (new field on `Civilization`)
   - `check_progress`: `diplomatic_favor / threshold`

5. **Register all conditions in `GameState::new()`**
   - Add to `GameState.victory_conditions: Vec<Box<dyn VictoryCondition>>`

---

## §8.14 — Natural Wonder Discovery Events ❌

Definitions and tile placement exist. No discovery triggers or yield application.

### Remaining tasks

1. **Discovery event on first visibility**
   - In fog-of-war reveal logic (`game/visibility.rs` or `advance_turn`): when a tile with `natural_wonder.is_some()` enters `visible_tiles` for the first time and is not yet in `explored_tiles`, emit event
   - New `StateDelta::NaturalWonderDiscovered { civ, wonder_name, coord }`

2. **Wonder yield bonus on worked tiles**
   - In `WorldTile::total_yields()` (or `compute_yields`): if tile has `natural_wonder`, add `NaturalWonder::yield_bonus()` to output

3. **Appeal radius on adjacent tiles**
   - `WorldTile` gains `appeal: i32` field (or computed on demand)
   - `NaturalWonder::appeal_bonus()` applied to all tiles within radius 1–2
   - Appeal feeds into housing/amenities calculations (future work)

---

## §8.15 — TurnEngine Consolidation ❌

`TurnEngine::process_turn()` discards the diff from `advance_turn` and returns empty. AI diffs not composed.

### Remaining tasks

1. **Return aggregated diff from `process_turn()`** (`game/turn.rs`)
   ```rust
   pub fn process_turn(&self, state: &mut GameState, rules: &dyn RulesEngine) -> GameStateDiff {
       let mut diff = rules.advance_turn(state);
       for agent in &self.agents {
           let agent_diff = agent.take_turn(state, rules);
           diff.merge(agent_diff);  // or extend deltas
       }
       diff
   }
   ```

2. **`GameStateDiff::merge(other)`** — if not already present, add method to extend one diff's deltas with another's

3. **Move production completion from `civsim` into `advance_turn` or `TurnEngine`**
   - Currently `civsim/src/main.rs` handles production queue pop; move to `game/rules.rs` `advance_turn` phase 2

4. **Wire AI agents into `TurnEngine`**
   - `TurnEngine` should hold `Vec<Box<dyn Agent>>` and call each in order between `advance_turn` phases or after

---

## Suggested Implementation Order

Dependencies drive ordering. Items with no incomplete dependencies first:

| Priority | Task | Blocks |
|----------|------|--------|
| 1 | §8.15 TurnEngine consolidation | All future testing |
| 2 | ~~§8.9 Builder charges~~ ✅ | ~~§8.8 road placement~~ |
| 3 | §8.3 Wall defense bonus in combat | §8.3 city ranged attack |
| 4 | ~~§8.8 Road placement action~~ ✅ | — |
| 5 | §8.3 City ranged attack | — |
| 6 | §8.12 Governor system | §8.6 loyalty |
| 7 | §8.14 Natural wonder discovery | — |
| 8 | §8.11 Era score & age | §8.10 great people |
| 9 | §8.10 Great people accumulation | §8.11 era triggers |
| 10 | §8.13 Domination + Science victory | §8.6 culture for Culture victory |
| 11 | §8.6 Loyalty system | §8.13 culture victory |
| 12 | §8.6 Tourism | §8.13 culture victory |
| 13 | §8.5 Religion system | — |
| 14 | §8.4 Trade deferred items | — |
| 15 | §8.13 Culture + Diplomatic victory | §8.6 tourism |
