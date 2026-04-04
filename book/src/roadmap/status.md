# Implementation Status & Roadmap

## Current Status

The engine has **461 passing integration tests** across 28+ test files. Core gameplay
is functional end-to-end: map generation, city founding, unit movement, combat,
research, production, trade, cultural borders, loyalty, era scoring, tourism,
religion, great people, barbarian clans, promotions, and victory conditions all work.

The civsim TUI exposes **all 36 RulesEngine methods** as interactive commands
(100% API coverage). Full-game integration tests exercise every major subsystem
in combined scenarios.

### Civ VI Parity Progress (2026-04-04)

See [parity.md](parity.md) for full details.

| Phase | Status | Notes |
|---|---|---|
| P0 Value Fixes | **DONE** | Resource yields, tech/civic prereqs, reveal techs |
| P1 Tech/Civic Trees | **CRITICAL PATH** | 12/67 techs, 8/50 civics (Ancient only) |
| P2 Resources | **DONE** | 10 bonus + 24 luxury + 7 strategic = 41 total |
| P3 Natural Wonders | **DONE** | 15 wonders (12 base + 3 DLC) |
| P4 Improvements | **DONE** | 15 standard + 9 unique = 24 total |
| P5 Districts | **Partial** | Districts done (16+4 UQ); ~60 buildings remain |
| P6 Units | **~75%** | 71/98 unit type defs |
| P7 World Wonders | Not started | 0/29 |
| P8 Gov/Policies | Not started | 2/10 governments, 4/113 policies |
| P9 Civilizations | Not started | 8/19 base-game civs |
| P10 Promotions | **~97%** | 118/122 across 16 classes |
| P11 City-States | Not started | 0/24 concrete |
| P12 Great People | Partial | ~72/177 individuals |

## Completed Systems

| System | Tests | Notes |
|--------|-------|-------|
| Hex grid geometry | libhexgrid tests | Coordinates, pathfinding, LOS, edge canonicalization |
| Map generation | 6 tests | Continents, climate zones, features, rivers, resources, starts |
| Terrain & features | 2 tests | Forest/rainforest resource concealment |
| Tile improvements | 7 tests | Farm, mine, lumbermill with tech/terrain validation |
| Builder charges | 16 tests | Charge consumption, road placement, upgrade/downgrade rules |
| Road maintenance | 2 tests | Gold deduction per turn, ancient roads free |
| Districts | 12 tests | Placement validation: tech, civic, terrain, range, uniqueness |
| Cities & territory | 16 tests | Claiming, reassignment, border expansion, founding |
| Units & combat | 3+ tests | Melee combat, terrain bonuses/penalties |
| City capture | 6 tests | Defender elimination, ownership transfer, domination victory |
| City defenses & bombardment | 14 tests | Walls, wall HP, city ranged attack, siege bonus |
| Research & civics | via gameplay tests | Tech tree, eureka, civic tree, inspiration |
| Trade routes | 11 tests | Domestic/international yields, autonomous traders, expiry |
| Diplomacy | via gameplay tests | War/peace, grievances, status changes |
| Natural wonders | 3 tests | Yields, movement cost, appeal |
| LOS & elevation | 2 tests | Cliff blocking, gradual transition |
| AI agent | 14 tests | Production, movement, determinism, multi-turn stability, simulation |
| Great people | 22 tests | Points, recruitment, patronage (gold/faith), retirement |
| Governors | 17 tests | Assignment, establishment timer, promotions, loyalty bonus |
| Era score | 11 tests | Historic moments, era advancement, age determination |
| Loyalty | 13 tests | Pressure, revolt, occupation penalty, governor bonus |
| Tourism & culture | 15 tests | Accumulation, great works, dominance, cultural victory |
| Victory conditions | 8 tests | Score, culture, domination |
| Civ abilities | 13 tests | Rome, Babylon, Greece, Germany, unique units/districts/improvements |
| Religion | 55 tests | Pantheon, founding, spread, theological combat, inquisition |
| Barbarians | 20 tests | Camps, scouts, clans (hire/bribe/incite), conversion |
| Full-game integration | 9 tests | 50-turn games, all-systems combined scenarios |
| Multiplayer server | -- | WebSocket, auth, game rooms, fog-of-war projection |
| WASM frontend | -- | Leptos app, hex renderer, WebSocket client |

## Partially Implemented

### Great People Points & Recruitment

Retirement system is fully implemented. Points accumulation and recruitment are not yet done.

**Done:**
- `GreatPersonDef` with `RetireEffect` enum (combat modifier, production burst, gold)
- 4 built-in great persons: Sun Tzu, Themistocles, Imhotep, Marco Polo
- `RulesEngine::retire_great_person()` with full delta emission
- Combat modifier integration (retired general/admiral bonus persists in `attack()`)
- 6 passing tests (retirement effects, combat modifier, double-retire guard)

**Remaining tasks (dependency order):**

1. **`great_person_points: HashMap<GreatPersonType, u32>` on `Civilization`** -- currently tracked via TODO in `civ/civilization.rs`
2. **Points accumulation in `advance_turn`** -- per civ: for each district, add points to the corresponding `GreatPersonType` bucket (Campus -> Scientist, Theatre Square -> Artist/Musician/Writer, etc.)
3. **Great person candidate pool with era gating** -- available candidates filtered by `current_era` (era gate already on `GreatPersonDef`)
4. **`RulesEngine::recruit_great_person(state, civ_id, great_person_def_id) -> Result<GameStateDiff, RulesError>`** -- deduct accumulated points, spawn unit, emit `StateDelta::GreatPersonRecruited`

### Governor Assignment

Data structures and loyalty integration exist. Assignment action and full lifecycle are not yet implemented.

**Done:**
- `Governor` struct with `assigned_city`, `turns_to_establish`, `is_established()`
- `governors: Vec<Governor>` on `GameState`
- 7 built-in definitions (Liang, Magnus, Amani, Victor, Pingala, Reyna, Ibrahim)
- Governor establishment bonus integrated into loyalty pressure
- Test: `governor_stabilizes_loyalty` verifies governors affect loyalty

**Remaining tasks:**

1. **Fix governor ID generation** -- all governors share `Ulid::nil()` (bug in `governor.rs:55`); use `IdGenerator`
2. **`RulesEngine::assign_governor(state, civ_id, governor_name, city_id) -> Result<GameStateDiff, RulesError>`** -- set `governor.assigned_city = Some(city_id)`, reset `turns_to_establish`, emit `StateDelta::GovernorAssigned { civ, city, governor_name }`
3. **Establishment timer in `advance_turn`** -- for each governor with `assigned_city.is_some()` and `turns_to_establish > 0`, decrement; emit `StateDelta::GovernorEstablished` when it reaches 0
4. **Governor modifiers in `compute_yields`** -- when governor `is_established()`, include their `GovernorDef::modifiers()` in modifier collection for that city

### Victory Conditions

Score, Culture, and Domination victories are implemented. Three additional victory types remain.

**Done:**
- `VictoryCondition` trait + evaluation loop in `advance_turn` Phase 5c
- `ScoreVictory` (turn-limit with scoring)
- `CultureVictory` (tourism exceeds all other civs' domestic culture)
- `DominationVictory` (capture all original capitals)
- `VictoryAchieved` StateDelta + `game_over: Option<GameOver>` on `GameState`
- 8 passing tests

**Remaining tasks:**

1. **`ScienceVictory`** -- milestone chain: Launch Earth Satellite -> Land on Moon -> Establish Mars Colony; each milestone requires specific tech prerequisites; `check_progress`: count completed milestones / 3
2. **`DiplomaticVictory`** -- simplified: win when `diplomatic_favor >= threshold` (new field on `Civilization`); `check_progress`: `diplomatic_favor / threshold`
3. **`ReligiousVictory`** -- win when your religion is the majority religion in every other civ's cities (requires religion system)

### TurnEngine Consolidation

Basic turn processing works but the diff system has gaps. See [TurnEngine Diff Consolidation](./diff-consolidation.md) for the detailed plan.

**Done:** basic turn processing, `advance_turn` produces diffs for most phases

**Remaining:**
- `TurnEngine::process_turn` discards the diff from `advance_turn` (one-line fix)
- Several turn phases mutate state without emitting deltas (citizen assignment, trader movement reset, unique unit healing, tourism output, post-revolt loyalty)
- AI diffs not composed into master turn diff

### Trade Routes (Deferred Items)

Core trade route lifecycle is complete. Four edge cases remain:

1. **Per-city food/production delivery** -- `compute_route_yields()` currently gold-only; add food/production contributions for domestic routes
2. **Max trade route capacity** -- gate additional slots on Commercial Hub district count per civ
3. **Route cancellation on city capture** -- drain routes whose origin or destination matches the captured city
4. **Trader respawn on route expiry** -- spawn a new Trader unit at origin city when a route expires

## Not Yet Implemented

### Religion System

All data structures are defined (`Religion`, `Belief`, `BeliefContext`). No mechanics are implemented.

**Implementation plan (dependency order):**

1. **Great Prophet unit type** -- add to `UnitTypeDef` registry (`category: UnitCategory::Religious`)
2. **`RulesEngine::found_religion(state, great_prophet_id, name, beliefs) -> Result<GameStateDiff, RulesError>`** -- validate Great Prophet unit, set `Religion { holy_city, beliefs, founded_by }`, remove unit, emit `StateDelta::ReligionFounded`
3. **Missionary and Apostle unit types** -- add to registry with `RulesEngine::spread_religion(state, unit_id, target_city_id)`
4. **Per-city religion pressure** -- add `religion_pressure: HashMap<ReligionId, u32>` to `City`; accumulate pressure from adjacent cities, holy cities, missionaries in `advance_turn`
5. **Majority-religion tracking** -- helper `city_majority_religion(city) -> Option<ReligionId>`; wire into `compute_yields` for `Belief` modifier integration
6. **Belief modifier integration** -- `Belief` implementations return `Vec<Modifier>`; include in `compute_yields` modifier collection

### Natural Wonder Discovery Events

Definitions, tile placement, yield bonuses, and appeal values are done. Discovery trigger is missing.

**Remaining tasks:**

1. **Discovery event on first visibility** -- when a tile with `natural_wonder.is_some()` enters `visible_tiles` for the first time and is not yet in `explored_tiles`, emit `StateDelta::NaturalWonderDiscovered { civ, wonder_name, coord }`
2. **Appeal radius on adjacent tiles** (deferred -- feeds into future housing/amenities work)

### Strategic Resource Consumption

- Units requiring strategic resources for production
- Resource stockpile tracking per unit maintained
- Resource depletion on unit creation

### Additional Content

- Later-era techs and civics (Medieval through Future)
- More civilizations and leaders
- World wonders (built wonders, not natural wonders)
- More unit types per era
- City-state quests and suzerainty mechanics

## Implementation Order

Based on dependencies and impact, the recommended order for remaining work:

### Phase 1: Engine Consolidation

1. **TurnEngine diff aggregation** -- fix diff composition so all turn phases contribute to a single coherent diff. This is a prerequisite for reliable replay and RL observation. See [detailed plan](./diff-consolidation.md).
2. **Governor assignment** -- wire up the `assign_governor()` action and establishment timer. The data structures and loyalty integration are already done.

### Phase 2: Missing Core Mechanics

3. **Great people recruitment** -- add points accumulation per district type, candidate pool, and the `recruit_great_person()` action.
4. **Natural wonder discovery** -- one-time yield/era-score bonus on first sight. Low complexity, high gameplay value.
5. **Strategic resource consumption** -- enforce resource costs when producing units.

### Phase 3: Victory Completion

6. **Science victory** -- define milestone techs and a progress tracker.
7. **Religious victory** -- requires religion founding and spread (see Phase 4).
8. **Diplomatic victory** -- requires a diplomatic favor system.

### Phase 4: Religion

9. **Religion founding** -- Great Prophets found religions at Holy Sites.
10. **Belief system** -- belief selection and modifier integration.
11. **Religious spread** -- Missionary/Apostle units and passive pressure.

### Phase 5: Content Expansion

12. **Later-era tech/civic trees** -- Medieval through Future eras.
13. **More civilizations** -- additional leaders and unique abilities.
14. **World wonders** -- constructed wonders with unique effects.
15. **City-state mechanics** -- quests, envoys, suzerainty bonuses.

### Phase 6: Polish

16. **Save/load** -- serialize GameState for persistence.
17. **Replay viewer** -- reconstruct games from diff sequences.
18. **RL training harness** -- structured observation space and reward shaping.
19. **Performance** -- indexed lookups where profiling identifies hot paths.
