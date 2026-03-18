# Open Civ VI — Implementation TODO

> Generated from ARCHITECTURE.md §8. Status assessed 2026-03-18, re-assessed 2026-03-18 after PRs #1–#4.

Legend: ✅ Done · 🔶 Partial · ❌ Missing

---

## Overall Status

| § | Feature | Status |
|---|---------|--------|
| 8.1 | Coherent Map Generation | ✅ Done |
| 8.3 | City Defenses & Ranged Attacks | ✅ Done |
| 8.4 | Trade Routes & Trader Units | ✅ Done (core) |
| 8.5 | Religion System | ❌ Stub only |
| 8.6 | Culture Borders | ✅ Done |
| 8.6 | Loyalty & Tourism | ✅ Done |
| 8.8 | Road Placement | ✅ Done |
| 8.9 | Builder Charges | ✅ Done |
| 8.10 | Great People System | 🔶 Partial |
| 8.11 | Era Score & Age System | ✅ Done |
| 8.12 | Governor System | 🔶 Partial |
| 8.13 | Victory Condition Evaluation | 🔶 Partial (score + culture) |
| 8.14 | Natural Wonder Discovery | ❌ Stub only |
| 8.15 | TurnEngine Consolidation | ❌ Broken |

---

## §8.1 — Coherent Map Generation ✅

Six-phase pipeline fully implemented in `libciv/src/world/mapgen/`:
continents → zones → features → rivers → resources → starting positions.
Tests in `libciv/tests/mapgen.rs`.

**No remaining work.**

---

## §8.3 — City Defenses and Ranged Attacks ✅

All four tasks fully implemented and tested (merged via PRs).

| Component | File | Status |
|-----------|------|--------|
| Wall defense bonus in `attack()` | `game/rules.rs` | ✅ Done |
| `WallDamaged` / `WallDestroyed` deltas | `game/diff.rs` | ✅ Done |
| Wall HP damage on melee attacks | `game/rules.rs` | ✅ Done |
| `City::has_attacked_this_turn` field | `civ/city.rs` | ✅ Done |
| `city_bombard()` trait + impl | `game/rules.rs` | ✅ Done |
| `CityBombard` `AttackType` variant | `game/diff.rs` | ✅ Done |
| `siege_bonus: u32` on `UnitTypeDef` | `game/state.rs` | ✅ Done |
| Siege bonus applied in `attack()` | `game/rules.rs` | ✅ Done |
| `has_attacked_this_turn` reset in `advance_turn` | `game/rules.rs` | ✅ Done |

**Tests in `libciv/tests/gameplay.rs` (all passing):**
- `wall_defense_bonus_reduces_damage_to_defender`
- `melee_attack_damages_city_walls`
- `wall_destruction_when_hp_reaches_zero`
- `ranged_attack_does_not_damage_walls`
- `city_bombard_deals_damage_no_counter`
- `city_bombard_requires_walls`
- `city_bombard_range_check`
- `city_bombard_once_per_turn_resets_after_advance`
- `siege_unit_bonus_applies_on_city_tile`
- `siege_bonus_not_applied_in_open_field`
- `city_capture_transfers_ownership_on_last_defender_killed`
- `city_capture_destroys_garrisoned_units_on_tile`
- `city_bombard_fails_after_walls_are_destroyed`
- `city_bombard_fails_after_walls_breached_by_combat`

**No remaining work.**

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

## §8.6 — Loyalty and Tourism ✅

Both subsystems fully implemented and tested.

### Loyalty ✅

| Component | File | Status |
|-----------|------|--------|
| `City::loyalty: i32` field (range 0–100) | `civ/city.rs` | ✅ Done |
| `compute_city_loyalty_delta()` — pressure from adjacent civs, amenities, governor bonus | `game/rules.rs` | ✅ Done |
| Loyalty phase in `advance_turn` (Phase 3c) | `game/rules.rs` | ✅ Done |
| City revolt/flip at loyalty == 0 | `game/rules.rs` | ✅ Done |
| `LoyaltyChanged`, `CityRevolted` StateDelta variants | `game/diff.rs` | ✅ Done |

**Tests in `libciv/tests/loyalty.rs`:** 16 tests covering pressure, revolt, capital resistance, governor bonus, foreign city influence, and city flip mechanics.

### Tourism ✅

| Component | File | Status |
|-----------|------|--------|
| `Civilization::lifetime_culture` tracking | `civ/civilization.rs` | ✅ Done |
| `Civilization::tourism_accumulated` HashMap | `civ/civilization.rs` | ✅ Done |
| `compute_tourism()` function | `civ/tourism.rs` | ✅ Done |
| Tourism generation in `advance_turn` (Phase 3a) | `game/rules.rs` | ✅ Done |
| `TourismGenerated` StateDelta | `game/diff.rs` | ✅ Done |
| `CultureVictory` — won when tourism exceeds every other civ's domestic culture | `game/victory.rs` | ✅ Done |

**Tests in `libciv/tests/tourism.rs`:** 15 tests covering lifetime culture, wonder contributions, cultural dominance, and culture victory trigger.

**No remaining work.**

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

## §8.10 — Great People System 🔶

Retirement system fully implemented. Points accumulation and recruitment not yet done.

### What exists

| Component | File | Status |
|-----------|------|--------|
| `GreatPersonDef` with `RetireEffect` enum (combat modifier, production burst, gold) | `civ/great_people.rs` | ✅ Done |
| `builtin_great_person_defs()` — Sun Tzu, Themistocles, Imhotep, Marco Polo | `civ/great_people.rs` | ✅ Done |
| `RulesEngine::retire_great_person()` trait + impl | `game/rules.rs` | ✅ Done |
| `GreatPersonRetired`, `ProductionBurst` StateDelta variants | `game/diff.rs` | ✅ Done |
| Combat modifier integration (retired general/admiral bonus persists in `attack()`) | `game/rules.rs` | ✅ Done |

**Tests in `libciv/tests/great_persons.rs`:** `test_retire_great_general_grants_land_combat_bonus`, `test_retire_great_admiral_grants_naval_combat_bonus`, `test_retire_great_engineer_adds_production`, `test_retire_great_merchant_grants_gold`, `test_retire_already_retired_fails`, `test_great_person_combat_modifier_applies_in_battle`.

### Remaining tasks

1. **`great_person_points: HashMap<GreatPersonType, u32>` on `Civilization`** — currently tracked via TODO in `civ/civilization.rs`

2. **Points accumulation in `advance_turn`**
   - Per civ: for each district, add points to the corresponding `GreatPersonType` bucket
     (Campus → Scientist, Theatre Square → Artist/Musician/Writer, etc.)

3. **Great person candidate pool with era gating**
   - Available candidates filtered by `current_era` (era gate already on `GreatPersonDef`)

4. **`RulesEngine::recruit_great_person(state, civ_id, great_person_def_id) -> Result<GameStateDiff, RulesError>`**
   - Deduct accumulated points, spawn unit, emit `StateDelta::GreatPersonRecruited`

---

## §8.11 — Era Score and Age System ✅

Fully implemented and tested (merged via PR #4).

| Component | File | Status |
|-----------|------|--------|
| `Civilization::era_score: u32` | `civ/civilization.rs` | ✅ Done |
| `Civilization::era_age: EraAge` (Dark/Normal/Golden/Heroic) | `civ/civilization.rs` | ✅ Done |
| `Civilization::historic_moments` + `earned_moments` dedup guard | `civ/civilization.rs` | ✅ Done |
| `HistoricMoment` definitions (FirstCity, FirstTech, WonderBuilt, CityCaptured, BattleWon, …) | `civ/era.rs` | ✅ Done |
| `observe_deltas()` — awards era score from StateDelta stream | `civ/era.rs` | ✅ Done |
| Phase 5b-1 in `advance_turn`: era score observer | `game/rules.rs` | ✅ Done |
| Phase 5b-2 in `advance_turn`: era advancement check + `compute_era_age()` | `game/rules.rs` | ✅ Done |
| Age modifiers (Dark penalty, Golden bonus, Heroic stacked) applied via `Modifier` system | `game/rules.rs` | ✅ Done |
| `HistoricMomentEarned`, `EraAdvanced` StateDelta variants | `game/diff.rs` | ✅ Done |

**Tests in `libciv/tests/era_score.rs`:** 9 tests covering historic moment earning, deduplication, era advancement, age assignment (golden/dark/heroic), and delta emission.

**No remaining work.**

---

## §8.12 — Governor System 🔶

`governors: Vec<Governor>` on `GameState`, 7 built-in definitions, and `is_established()` defined. Governors integrated into loyalty computation. Assignment action and full lifecycle not yet implemented.

### What exists

| Component | File | Status |
|-----------|------|--------|
| `Governor` struct with `assigned_city`, `turns_to_establish`, `is_established()` | `civ/governor.rs` | ✅ Done |
| `governors: Vec<Governor>` on `GameState` | `game/state.rs` | ✅ Done |
| 7 built-in definitions (Liang, Magnus, Amani, Victor, Pingala, Reyna, Ibrahim) | `civ/governor.rs` | ✅ Done |
| Governor establishment bonus integrated into loyalty pressure | `game/rules.rs` | ✅ Done |

**Tests:** `governor_stabilizes_loyalty` in `libciv/tests/loyalty.rs` verifies governors affect loyalty.

### Remaining tasks

1. **Fix governor ID generation** — all governors share `Ulid::nil()` (bug in `governor.rs:55`); use `IdGenerator`

2. **`RulesEngine::assign_governor(state, civ_id, governor_name, city_id) -> Result<GameStateDiff, RulesError>`**
   - Set `governor.assigned_city = Some(city_id)`, reset `turns_to_establish`
   - Emit `StateDelta::GovernorAssigned { civ, city, governor_name }`

3. **Establishment timer in `advance_turn`**
   - For each governor with `assigned_city.is_some()` and `turns_to_establish > 0`, decrement
   - Emit `StateDelta::GovernorEstablished { city, governor_name }` when it reaches 0

4. **Governor modifiers in `compute_yields`**
   - When governor `is_established()`, include their `GovernorDef::modifiers()` in modifier collection for that city

---

## §8.13 — Victory Condition Evaluation 🔶

`ScoreVictory` and `CultureVictory` implemented. `game_over: Option<GameOver>` on `GameState` blocks further evaluation after a win. Evaluation loop in `advance_turn` Phase 5c active.

### What exists

| Component | File | Status |
|-----------|------|--------|
| `VictoryCondition` trait + evaluation loop | `game/victory.rs` / `game/rules.rs` | ✅ Done |
| `ScoreVictory` — wins at turn limit | `game/victory.rs` | ✅ Done |
| `CultureVictory` — wins when tourism exceeds all other civs' domestic culture | `game/victory.rs` | ✅ Done |
| `VictoryAchieved` StateDelta | `game/diff.rs` | ✅ Done |
| `game_over: Option<GameOver>` on `GameState` | `game/state.rs` | ✅ Done |

**Tests in `libciv/tests/victory.rs`:** `score_increases_with_cities_and_techs`, `score_victory_fires_at_turn_limit`, `culture_victory_triggers_game_over_in_advance_turn`, `game_over_blocks_further_victory_evaluation`.

### Remaining tasks

1. **`DominationVictory`** (`game/victory.rs`)
   - Won when a civ controls the original capital of every other civ
   - `check_progress`: count captured original capitals / total other civs

2. **`ScienceVictory`**
   - Milestone chain: Launch Earth Satellite → Land on Moon → Establish Mars Colony
   - Each milestone requires specific projects/wonders (simplification: just tech prerequisites)
   - `check_progress`: count completed milestones / 3

3. **`DiplomaticVictory`**
   - Won by accumulating Diplomatic Favor and winning World Congress vote
   - Simplified: win when `diplomatic_favor >= threshold` (new field on `Civilization`)
   - `check_progress`: `diplomatic_favor / threshold`

4. **Register all conditions in `GameState::new()`**
   - `DominationVictory` and `ScienceVictory` not yet in `victory_conditions` vec

---

## §8.14 — Natural Wonder Discovery Events ❌

Definitions, tile placement, yield bonuses, and appeal values are all done. Discovery trigger event is missing.

### What exists

| Component | File | Status |
|-----------|------|--------|
| `NaturalWonder` trait with `yield_bonus()` and `appeal_bonus()` | `world/wonder.rs` | ✅ Done |
| 5 built-in wonders (Krakatoa, Grand Mesa, Cliffs of Dover, Uluru, Galapagos) | `world/wonder.rs` | ✅ Done |
| `natural_wonder: Option<BuiltinNaturalWonder>` on `WorldTile` | `world/tile.rs` | ✅ Done |
| Wonder yield bonus applied in `WorldTile::total_yields()` | `world/tile.rs` | ✅ Done |
| Appeal bonus values defined per wonder | `world/wonder.rs` | ✅ Done |

**Tests in `libciv/tests/natural_wonders.rs`:** `wonder_tile_yields_terrain_plus_wonder_bonus`, `impassable_wonder_has_impassable_movement_cost`, `wonder_appeal_bonus_is_correct`.

### Remaining tasks

1. **Discovery event on first visibility**
   - In fog-of-war reveal logic (`game/visibility.rs` or `advance_turn`): when a tile with `natural_wonder.is_some()` enters `visible_tiles` for the first time and is not yet in `explored_tiles`, emit event
   - New `StateDelta::NaturalWonderDiscovered { civ, wonder_name, coord }`

2. **Appeal radius on adjacent tiles** (deferred — feeds into future housing/amenities work)
   - `NaturalWonder::appeal_bonus()` applied to all tiles within radius 1–2 at query time

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

Dependencies drive ordering. Completed items struck through.

| Priority | Task | Blocks |
|----------|------|--------|
| 1 | §8.15 TurnEngine consolidation | All future testing |
| 2 | ~~§8.9 Builder charges~~ ✅ | ~~§8.8 road placement~~ |
| 3 | ~~§8.3 Wall defense bonus in combat~~ ✅ | ~~§8.3 city ranged attack~~ |
| 4 | ~~§8.8 Road placement action~~ ✅ | — |
| 5 | ~~§8.3 City ranged attack~~ ✅ | — |
| 6 | §8.12 Governor assign + establish timer | §8.12 yield modifiers |
| 7 | §8.14 Natural wonder discovery event | — |
| 8 | ~~§8.11 Era score & age~~ ✅ | ~~§8.10 great people era gating~~ |
| 9 | §8.10 Great people accumulation + recruitment | — |
| 10 | §8.13 Domination + Science victory | — |
| 11 | ~~§8.6 Loyalty system~~ ✅ | ~~§8.13 culture victory~~ |
| 12 | ~~§8.6 Tourism~~ ✅ | ~~§8.13 culture victory~~ |
| 13 | §8.5 Religion system | — |
| 14 | §8.4 Trade deferred items | — |
| 15 | §8.13 Diplomatic victory | — |
