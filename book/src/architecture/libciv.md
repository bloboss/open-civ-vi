# libciv -- Game Engine

`libciv` contains all game state, rules, and simulation logic. It is organized into five major modules:

```
libciv/src/
+-- ids.rs          # ULID-backed entity IDs
+-- yields.rs       # YieldBundle and YieldType
+-- enums.rs        # Shared game enums
+-- world/          # Terrain, features, tiles, improvements, mapgen
+-- rules/          # Modifiers, tech trees, policies, effects
+-- civ/            # Civilizations, cities, units, diplomacy, religion
+-- game/           # GameState, WorldBoard, RulesEngine, TurnEngine
+-- ai/             # Agent trait and HeuristicAgent
+-- visualize.rs    # ASCII terminal rendering
```

## Core Types

### GameState

The central data structure holding all game state:

```rust
struct GameState {
    // Fundamental
    turn: u32,
    board: WorldBoard,
    ids: IdGenerator,

    // Entities
    civilizations: Vec<Civilization>,
    cities: Vec<City>,
    units: Vec<BasicUnit>,
    great_people: Vec<GreatPerson>,
    governors: Vec<Governor>,

    // Knowledge
    tech_tree: TechTree,
    civic_tree: CivicTree,
    tech_refs: TechRefs,
    civic_refs: CivicRefs,

    // Government
    governments: Vec<Government>,
    policies: Vec<Policy>,

    // Definitions
    unit_type_defs: Vec<UnitTypeDef>,
    building_defs: Vec<BuildingDef>,

    // Systems
    diplomatic_relations: Vec<DiplomaticRelation>,
    religions: Vec<Religion>,
    placed_districts: Vec<PlacedDistrict>,
    great_works: Vec<GreatWork>,
    eras: Vec<Era>,
    victory_conditions: Vec<Box<dyn VictoryCondition>>,
    game_over: Option<GameOver>,
    // ...
}
```

`GameState` is passed by mutable reference to all engine operations. There is no global state.

### WorldBoard

Implements `HexBoard` from `libhexgrid`:

```rust
struct WorldBoard {
    width: usize,
    height: usize,
    topology: BoardTopology,
    tiles: Vec<WorldTile>,
    edges: HashMap<(HexCoord, HexDir), WorldEdge>,
}
```

Key methods:
- `find_path()` -- Dijkstra pathfinding with terrain costs, road overrides, and edge crossing penalties
- `has_los()` -- Line-of-sight check using elevation-based blocking
- `set_edge()` -- Edge insertion with automatic canonicalization

### RulesEngine

The trait defining all game actions:

```rust
trait RulesEngine {
    fn move_unit(&self, state: &mut GameState, unit_id: UnitId, path: &[HexCoord])
        -> Result<GameStateDiff, RulesError>;
    fn attack(&self, state: &mut GameState, attacker: UnitId, defender: UnitId)
        -> Result<GameStateDiff, RulesError>;
    fn found_city(&self, state: &mut GameState, settler: UnitId, name: &str)
        -> Result<GameStateDiff, RulesError>;
    fn compute_yields(&self, state: &GameState, city: CityId) -> YieldBundle;
    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff;
    fn place_improvement(&self, ...) -> Result<GameStateDiff, RulesError>;
    fn place_district(&self, ...) -> Result<GameStateDiff, RulesError>;
    fn establish_trade_route(&self, ...) -> Result<GameStateDiff, RulesError>;
    fn declare_war(&self, ...) -> Result<GameStateDiff, RulesError>;
    fn make_peace(&self, ...) -> Result<GameStateDiff, RulesError>;
    // ... and more
}
```

`DefaultRulesEngine` is the concrete implementation. Every method returns a `GameStateDiff` describing the state changes.

### TurnEngine

Stateless turn orchestrator that coordinates the full game turn:

```rust
struct TurnEngine;

impl TurnEngine {
    fn process_turn(state: &mut GameState, rules: &dyn RulesEngine) -> GameStateDiff;
}
```

A single `process_turn()` call advances the game by one turn, processing all phases: production, research, growth, trade, loyalty, culture, era scoring, and victory checks.

## Module Details

### world/

| File | Contents |
|------|----------|
| `terrain.rs` | `BuiltinTerrain` -- 8 terrain types with yields and movement costs |
| `feature.rs` | `BuiltinFeature` -- 8 tile features (forest, marsh, reef, etc.) |
| `resource.rs` | `BuiltinResource` -- 23 resources across bonus/luxury/strategic categories |
| `improvement.rs` | `BuiltinImprovement` -- 9 improvements with tech/terrain/feature requirements |
| `road.rs` | `BuiltinRoad` -- 4 road tiers (Ancient through Railroad) with tech gates |
| `edge.rs` | `BuiltinEdgeFeature` -- rivers, canals, mountain passes |
| `tile.rs` | `WorldTile` -- combines terrain + hills + feature + resource + improvement + road |
| `wonder.rs` | `BuiltinNaturalWonder` -- 5 natural wonders with yields and appeal |
| `mapgen/` | Procedural map generation pipeline (continents, climate, rivers, resources) |

### rules/

| File | Contents |
|------|----------|
| `modifier.rs` | `Modifier`, `EffectType`, `TargetSelector`, `Condition`, `resolve_modifiers()` |
| `effect.rs` | `OneShotEffect` -- tech/civic completion effects (unlock, reveal, free unit) |
| `tech.rs` | `TechTree`, `TechNode`, `build_tech_tree()` -- 12 Ancient Era techs |
| `civic_tree_def.rs` | `CivicTree`, `CivicNode`, `build_civic_tree()` -- 6 Ancient Era civics |
| `policy.rs` | `Government`, `Policy`, `PolicySlots` |
| `victory.rs` | `VictoryCondition` trait, `VictoryProgress` |

### civ/

| File | Contents |
|------|----------|
| `civilization.rs` | `Civilization`, `Leader`, `TechProgress`, `CivicProgress` |
| `city.rs` | `City`, `CityKind`, `CityOwnership`, `WallLevel`, `ProductionItem` |
| `unit.rs` | `Unit` trait, `BasicUnit` |
| `district.rs` | `BuiltinDistrict` -- 12 district types with requirements |
| `diplomacy.rs` | `DiplomaticRelation`, `DiplomaticStatus`, `GrievanceRecord` |
| `religion.rs` | `Religion`, `Belief` trait |
| `trade.rs` | `TradeRoute`, `compute_route_yields()` |
| `great_people.rs` | `GreatPerson`, `GreatPersonDef`, `RetireEffect` |
| `governor.rs` | `Governor`, `GovernorDef` trait, built-in governors |
| `era.rs` | `Era`, `EraAge`, `HistoricMomentKind`, thresholds |
| `tourism.rs` | `compute_tourism()`, `domestic_tourists()`, `has_cultural_dominance()` |
| `great_works.rs` | `GreatWork`, `GreatWorkSlot`, `GreatWorkType` |
| `grievance.rs` | Concrete grievance triggers (war, pillage, capture) |

### game/

| File | Contents |
|------|----------|
| `state.rs` | `GameState`, `IdGenerator`, `UnitTypeDef`, `BuildingDef` |
| `board.rs` | `WorldBoard` -- pathfinding, LOS, edge canonicalization |
| `rules.rs` | `DefaultRulesEngine` implementing `RulesEngine` |
| `diff.rs` | `GameStateDiff`, `StateDelta` (40+ variants) |
| `turn.rs` | `TurnEngine` -- full turn processing |
| `visibility.rs` | `recalculate_visibility()` -- fog of war |
| `score.rs` | `compute_score()`, `all_scores()` |
| `victory.rs` | `ScoreVictory`, `CultureVictory`, `DominationVictory`, `GameOver` |
