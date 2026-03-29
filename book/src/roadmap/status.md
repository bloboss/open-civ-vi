# Implementation Status & Roadmap

## Current Status

The engine has 200+ passing integration tests across 21 test files. Core gameplay is functional end-to-end: map generation, city founding, unit movement, combat, research, production, trade, cultural borders, loyalty, era scoring, tourism, and victory conditions all work.

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
| Research & civics | via gameplay tests | Tech tree, eureka, civic tree, inspiration |
| Trade routes | 11 tests | Domestic/international yields, autonomous traders, expiry |
| Diplomacy | via gameplay tests | War/peace, grievances, status changes |
| Natural wonders | 3 tests | Yields, movement cost, appeal |
| LOS & elevation | 2 tests | Cliff blocking, gradual transition |
| AI agent | 9 tests | Production, movement, determinism, multi-turn stability |
| Great people | 6 tests | Retirement effects, combat modifier integration |
| Era score | 11 tests | Historic moments, era advancement, age determination |
| Loyalty | 13 tests | Pressure, revolt, occupation penalty, governor bonus |
| Tourism & culture | 15 tests | Accumulation, great works, dominance, cultural victory |
| Victory conditions | 8 tests | Score, culture, domination |
| Civ abilities | 13 tests | Rome, Babylon, Greece, Germany, unique units/districts/improvements |
| Multiplayer server | -- | WebSocket, auth, game rooms, fog-of-war projection |
| WASM frontend | -- | Leptos app, hex renderer, WebSocket client |

## Partially Implemented

### Great People Points & Recruitment
- **Done**: retirement effects (combat bonus, production burst, gold grant), 4 built-in great persons
- **TODO**: points accumulation per turn (based on district ownership), candidate pool with era gating, `recruit_great_person()` action, competition between civs for the same great person

### Governor Assignment
- **Done**: data structures, loyalty integration, built-in governor definitions (Liang, Magnus, Amani)
- **TODO**: `assign_governor()` action in RulesEngine, establishment timer countdown in TurnEngine, promotion tree unlocking, governor-specific modifier application

### Victory Conditions
- **Done**: Score victory (turn-limit), Culture victory (tourism vs culture), Domination victory (capture all capitals)
- **TODO**: Science victory (milestone chain: satellite, moon landing, Mars colony), Diplomatic victory (diplomatic favor), Religious victory (convert all civs)

### TurnEngine Consolidation
- **Done**: basic turn processing
- **TODO**: proper diff aggregation from all turn phases, AI diff composition into the master turn diff

## Not Yet Implemented

### Religion System
- Founding religions via Great Prophets
- Belief selection and effects
- Religious unit spread mechanics
- Passive religious pressure between cities
- Theological combat
- Worship buildings

### Strategic Resource Consumption
- Units requiring strategic resources for production
- Resource stockpile tracking per unit maintained
- Resource depletion on unit creation

### Natural Wonder Discovery Events
- One-time bonus when a civilization first discovers a natural wonder
- Era score award for discovery
- Exploration incentive mechanics

### Additional Content
- Later-era techs and civics (Medieval through Future)
- More civilizations and leaders
- World wonders (built wonders, not natural wonders)
- More unit types per era
- City-state quests and suzerainty mechanics

## Suggested Implementation Order

Based on dependencies and impact, the recommended order for remaining work:

### Phase 1: Engine Consolidation
1. **TurnEngine diff aggregation** -- fix diff composition so all turn phases contribute to a single coherent diff. This is a prerequisite for reliable replay and RL observation.
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
