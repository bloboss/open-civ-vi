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
| 8.8 | Road Placement | ❌ Stub only |
| 8.9 | Builder Charges | ❌ Missing |
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

### Done
- `WallLevel` enum with `defense_bonus()` and `max_hp()` (`civ/city.rs`)
- `city.wall_hp` field

### Remaining tasks (dependency order)

1. **Integrate wall defense bonus into combat formula**
   - `game/rules.rs` `attack()` — add `WallLevel::defense_bonus()` to defender's effective strength when defending a city tile
   - Emit `StateDelta` when wall HP is reduced (new variant `WallDamaged { city, hp_remaining }` or reuse existing)

2. **Wall destruction event**
   - When `wall_hp` drops to 0, downgrade `WallLevel` and emit a delta

3. **City ranged attack action**
   - Add `RulesEngine::city_ranged_attack(state, city_id, target_unit_id) -> Result<GameStateDiff, RulesError>`
   - Wire into `advance_turn` phase (after movement, before diplomacy): each city with walls fires at nearest enemy unit in range

4. **Siege unit type**
   - Add `Catapult`/`Trebuchet` etc. to `UnitTypeDef` registry with `siege_bonus_vs_cities: u32`
   - Apply bonus in `attack()` when attacker is siege and defender is in a city

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

## §8.8 — Road Placement ❌

Data types and Dijkstra cost override exist. No placement action.

### Remaining tasks

1. **`RulesEngine::place_road(state, unit_id, coord, road: BuiltinRoad) -> Result<GameStateDiff, RulesError>`**
   - Validate builder unit at `coord`, tech requirement per road tier
   - Set `WorldTile::road`, emit `StateDelta::RoadPlaced { coord, road }`

2. **Road upgrade path enforcement**
   - `place_road()` rejects downgrades; validates tech gates:
     - `AncientRoad` — no tech required
     - `MedievalRoad` — requires "Engineering" civic/tech
     - `IndustrialRoad` — requires "Steam Power"
     - `Railroad` — requires "Railroads"

3. **Road maintenance in `advance_turn`**
   - New phase: sum `BuiltinRoad::maintenance()` across all road tiles owned by each civ; deduct from `civ.gold`
   - Emit `StateDelta::GoldChanged` per civ

---

## §8.9 — Builder Charges ❌

### Remaining tasks

1. **Add `charges: u8` field to `BasicUnit`** (default 3 for Builder)

2. **Decrement in `place_improvement()`**
   - After successful placement, call `unit.charges -= 1`
   - If `charges == 0`, remove unit and emit `StateDelta::UnitDestroyed`

3. **Decrement in `place_road()`** (same pattern, once §8.8 is implemented)

4. **Optional: `StateDelta::ChargesChanged { unit, remaining }`** for UI/replay

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
| 2 | §8.9 Builder charges | §8.8 road placement |
| 3 | §8.3 Wall defense bonus in combat | §8.3 city ranged attack |
| 4 | §8.8 Road placement action | — |
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
