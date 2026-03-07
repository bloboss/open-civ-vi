# Open Civ VI — Architecture

## Design Philosophy

1. **Invalid states unrepresentable** — encode constraints in types, not runtime checks
2. **Extensibility via traits** — game content implements traits; built-in variants are enums wrapping concrete structs, not special-cased
3. **Separated concerns** — geometry knows nothing of game concepts; world state knows nothing of geometry
4. **Single GameState** — one struct passed by reference to all systems; no global state
5. **Typed modifier pipeline** — every yield change is a `Modifier`; no opaque callbacks

`&'static str` is used for all built-in names (compile-time content). Structs with `Box<dyn Trait>` fields (`Leader`, `City`) do not derive `Clone`. `libhexgrid` has zero knowledge of terrain, civilizations, or rules.

---

## Library Structure

```
libhexgrid  (no deps)       — pure hex geometry: coords, traits, topology
libciv      (→ libhexgrid)  — all game state: ids, yields, enums, world/, rules/, civ/, game/
civsim      (→ libciv)      — CLI binary
```

---

## 1. libhexgrid

### Coordinates

```rust
struct HexCoord { pub q: i32, pub r: i32, pub s: i32 }
```

Invariant `q + r + s = 0` enforced at construction. `HexCoord::new(q, r, s)` returns `Result<Self, HexCoordError>`. `HexCoord::from_qr(q, r)` computes `s` automatically.

Arithmetic: `Add`, `Sub`, `Neg`, `Mul<i32>` — all preserve the invariant.

Distance: `(|dq| + |dr| + |ds|) / 2`.

```rust
impl HexCoord {
    fn new(q: i32, r: i32, s: i32) -> Result<Self, HexCoordError>;
    fn from_qr(q: i32, r: i32) -> Self;
    fn zero() -> Self;
    fn distance(&self, other: &HexCoord) -> u32;
    fn neighbors(&self) -> [HexCoord; 6];
    fn ring(&self, radius: u32) -> Vec<HexCoord>;
}
```

### Supporting Types

```rust
enum HexDir { E, NE, NW, W, SW, SE }
// ALL: [E, NE, NW, W, SW, SE]; unit_vec(); opposite()

enum MovementCost {
    Impassable,
    Cost(u32),  // scaled by 100; ONE=100, TWO=200, THREE=300
}

enum Elevation {
    Low,        // below sea level; never blocks LOS
    Level(u8),  // Initial Sea Level: Level(0),
                // Coastal, non-cliff tiles: Level(1)
                // ... a gradient of floodable tiles
                // ... a gradient of non-floodable tiles
    High,       // impassable mountain peak; always blocks LOS
}
// Implements Ord: Low < Level(0) < Level(1) < ... < High

enum Vision { Blind, Radius(u8), Omniscient }

enum MovementProfile { Ground, Naval, Air, Embarked, Amphibious }

enum BoardTopology { Flat, CylindricalEW, Toroidal }
```

### Traits

```rust
trait HexTile {
    fn coord(&self) -> HexCoord;
    fn elevation(&self) -> Elevation;
    fn movement_cost(&self) -> MovementCost;
    fn vision_bonus(&self) -> Vision;
}

trait HexEdge {
    fn coord(&self) -> HexCoord;          // canonical forward-half endpoint
    fn dir(&self) -> HexDir;              // always forward-half: E, NE, or NW
    fn endpoints(&self) -> (HexCoord, HexCoord);  // default impl
    fn crossing_cost(&self) -> MovementCost;
}

trait HexBoard {
    type Tile: HexTile;
    type Edge: HexEdge;
    fn topology(&self) -> BoardTopology;
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn tile(&self, coord: HexCoord) -> Option<&Self::Tile>;
    fn tile_mut(&mut self, coord: HexCoord) -> Option<&mut Self::Tile>;
    fn edge(&self, coord: HexCoord, dir: HexDir) -> Option<&Self::Edge>;
    fn neighbors(&self, coord: HexCoord) -> Vec<HexCoord>;
    fn normalize(&self, coord: HexCoord) -> Option<HexCoord>;
    fn all_coords(&self) -> Vec<HexCoord>;
}
```

Edges are stored canonically: forward-half directions only (`{E, NE, NW}`). Looking up a backward-half edge (`{W, SW, SE}`) normalizes to the adjacent tile with the opposite direction.

---

## 2. libciv — ids/yields/enums

### IDs

```rust
// define_id! macro generates ULID-backed newtypes:
// CityId, UnitId, CivId, TechId, CivicId, GovernmentId, PolicyId, ReligionId,
// WonderId, GreatPersonId, PromotionId, ImprovementId, ResourceId, RoadId,
// AgreementId, GrievanceId, GovernorId, BeliefId, VictoryId, UnitTypeId,
// DistrictTypeId, BuildingId, TradeRouteId, EraId, TerrainId, FeatureId,
// EdgeFeatureId, NaturalWonderId
```

Each ID implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`, `Display`.

### Yields

`YieldBundle` is a flat struct (not a HashMap) with named fields. The `YieldType` enum is used only for the `with()`, `get()`, and `add_yield()` dispatch methods.

```rust
enum YieldType {
    Food, Production, Gold, Science, Culture, Faith,
    Housing, Amenities, Tourism, GreatPersonPoints,
}

struct YieldBundle {
    pub food: i32, pub production: i32, pub gold: i32,
    pub science: i32, pub culture: i32, pub faith: i32,
    pub housing: i32, pub amenities: i32, pub tourism: i32,
    pub great_person_points: i32,
}
// impl Add, AddAssign; merge(), with(), get(), add_yield()
```

### Shared Enums

```rust
enum ResourceCategory { Bonus, Luxury, Strategic }
enum UnitDomain { Land, Sea, Air }
enum UnitCategory { Civilian, Combat, Support, Religious, GreatPerson, Trader }
enum GreatPersonType {
    General, Admiral, Engineer, Merchant, Musician, Artist, Writer, Prophet, Scientist,
}
enum AgeType {
    Ancient, Classical, Medieval, Renaissance, Industrial, Modern, Atomic, Information, Future,
}
enum PolicyType { Military, Economic, Diplomatic, Wildcard }
```

---

## 3. libciv — world/

### Terrain

```rust
trait TerrainDef: Debug {
    fn name(&self) -> &'static str;
    fn base_yields(&self) -> YieldBundle;
    fn movement_cost(&self) -> MovementCost;
    fn elevation(&self) -> Elevation;
    fn is_land(&self) -> bool;
    fn is_water(&self) -> bool { !self.is_land() }  // default
}

enum BuiltinTerrain {
    Grassland(Grassland), Plains(Plains), Desert(Desert),
    Tundra(Tundra), Snow(Snow), Coast(Coast), Ocean(Ocean),
}
// BuiltinTerrain::as_def() -> &dyn TerrainDef
```

Concrete types (`Grassland`, `Plains`, etc.) are zero-sized structs implementing `TerrainDef`.

### Features

```rust
trait FeatureDef: Debug {
    fn name(&self) -> &'static str;
    fn yield_modifier(&self) -> YieldBundle;
    fn movement_cost_modifier(&self) -> MovementCost;
    fn conceals_resources(&self) -> bool { false }  // default
}

enum BuiltinFeature {
    Forest(Forest), Rainforest(Rainforest), Marsh(Marsh), Floodplain(Floodplain),
    Reef(Reef), Ice(Ice), VolcanicSoil(VolcanicSoil), Oasis(Oasis),
}
```

`Ice` has `movement_cost_modifier() = Impassable`. Feature movement cost is additive with terrain cost; Impassable wins.

### Edge Features

```rust
trait EdgeFeatureDef: Debug {
    fn name(&self) -> &'static str;
    fn crossing_cost(&self) -> MovementCost;
    fn blocks_los(&self) -> bool { false }  // default
}

enum BuiltinEdgeFeature {
    River(River),             // crossing_cost = THREE (300)
    Cliff(Cliff),             // crossing_cost = Impassable, blocks_los = true
    Canal(Canal),             // crossing_cost = ONE
    MountainPass(MountainPass), // crossing_cost = TWO
}
```

### WorldTile and WorldEdge

```rust
struct WorldTile {
    pub coord: HexCoord,
    pub terrain: BuiltinTerrain,
    pub feature: Option<BuiltinFeature>,
    pub resource: Option<ResourceId>,
    pub improvement: ImprovementContext,  // { improvement_id, is_pillaged }
    pub road: Option<BuiltinRoad>,
    pub rivers: Vec<BuiltinEdgeFeature>,
    pub owner: Option<CivId>,
}
// impl HexTile; total_yields() = terrain base + feature modifier
```

```rust
struct WorldEdge {
    pub coord: HexCoord,   // canonical forward-half endpoint
    pub dir: HexDir,       // always E, NE, or NW
    pub feature: Option<BuiltinEdgeFeature>,
}
// impl HexEdge; crossing_cost() from feature or zero
```

`WorldEdge::new(coord, dir)` panics in debug if `dir` is not forward-half. Use `WorldBoard::set_edge()` for automatic canonicalization.

### Tile Improvements

```rust
trait TileImprovement: Debug {
    fn name(&self) -> &'static str;
    fn yield_bonus(&self) -> YieldBundle;
    fn build_turns(&self) -> u32;
    fn pillaged(&self) -> bool { false }
}
// Built-in: Farm (+1 Food), Mine (+1 Prod), LumberMill (+2 Prod),
//           TradingPost (+1 Gold), Fort, Airstrip, MissileSilo
```

### Roads

```rust
trait RoadDef: Debug {
    fn name(&self) -> &'static str;
    fn movement_cost(&self) -> MovementCost;  // cost when travelling along road
    fn maintenance(&self) -> u32;             // gold per turn
}

enum BuiltinRoad {
    Ancient(AncientRoad),         // Cost(50), maintenance 0
    Medieval(MedievalRoad),       // Cost(50), maintenance 1
    Industrial(IndustrialRoad),   // Cost(25), maintenance 2
    Railroad(Railroad),           // Cost(10), maintenance 3
}
```

---

## 4. libciv — rules/

### Modifier System

Every numeric effect (policy, building, leader ability, wonder, belief, tech, civic) is a `Modifier`. Modifiers are collected and applied at query time — stored state is never mutated directly.

```rust
struct Modifier {
    pub source: ModifierSource,
    pub target: TargetSelector,
    pub effect: EffectType,
    pub stacking: StackingRule,
}

enum EffectType {
    YieldFlat(YieldType, i32),
    YieldPercent(YieldType, i32),  // 50 = +50%, scaled by 100
    CombatStrengthFlat(i32),
    CombatStrengthPercent(i32),
    MovementBonus(u32),
}

enum TargetSelector {
    AllTiles,
    AllUnits,
    UnitDomain(UnitDomain),
    Civilization(CivId),
    Global,
}

enum StackingRule { Additive, Max, Replace }

enum ModifierSource {
    Tech(&'static str), Civic(&'static str), Policy(&'static str),
    Building(&'static str), Wonder(&'static str), Leader(&'static str),
    Religion(&'static str), Era(&'static str), Custom(&'static str),
}
```

`resolve_modifiers(modifiers: &[Modifier]) -> Vec<EffectType>` currently returns effects unprocessed (Phase 1 stub). Phase 2 implements proper stacking.

### Technology and Civics

```rust
struct TechNode {
    pub id: TechId,
    pub name: &'static str,
    pub cost: u32,
    pub prerequisites: Vec<TechId>,
    pub unlocks: Vec<Unlock>,
    pub eureka_description: &'static str,
}

struct CivicNode {
    pub id: CivicId,
    pub name: &'static str,
    pub cost: u32,
    pub prerequisites: Vec<CivicId>,
    pub unlocks: Vec<Unlock>,
    pub inspiration_description: &'static str,
}

enum Unlock {
    Unit(&'static str), Building(&'static str), Improvement(&'static str),
    District(&'static str), Policy(&'static str), Government(&'static str),
    Resource(&'static str), Ability(&'static str),
}

trait EurekaCondition: Debug {
    fn description(&self) -> &'static str;
    fn is_met(&self) -> bool;
}

struct TechTree  { pub nodes: HashMap<TechId,  TechNode>  }
struct CivicTree { pub nodes: HashMap<CivicId, CivicNode> }
// add_node(), get(), prerequisites_met()
```

`TechTree::prerequisites_met()` is implemented. `EurekaCondition::is_met()` is a trait stub with no concrete implementations yet.

### Policies and Governments

```rust
struct PolicySlots { pub military: u8, pub economic: u8, pub diplomatic: u8, pub wildcard: u8 }

struct Policy {
    pub id: PolicyId,
    pub name: &'static str,
    pub policy_type: PolicyType,
    pub modifiers: Vec<Modifier>,
    pub maintenance: u32,
}

struct Government {
    pub id: GovernmentId,
    pub name: &'static str,
    pub slots: PolicySlots,
    pub inherent_modifiers: Vec<Modifier>,
    pub legacy_bonus: Option<&'static str>,
}
// can_slot_policy()
```

### Victory Conditions

```rust
struct VictoryProgress {
    pub victory_id: VictoryId,
    pub civ_id: CivId,
    pub current: u32,
    pub target: u32,
}
// is_won(), percentage()

trait VictoryCondition: Debug {
    fn id(&self) -> VictoryId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check_progress(&self, civ_id: CivId) -> VictoryProgress;
}
```

No concrete `VictoryCondition` implementations exist yet.

---

## 5. libciv — civ/

### Units

```rust
trait Unit: Debug {
    fn id(&self) -> UnitId;
    fn unit_type(&self) -> UnitTypeId;
    fn owner(&self) -> CivId;
    fn coord(&self) -> HexCoord;
    fn domain(&self) -> UnitDomain;
    fn category(&self) -> UnitCategory;
    fn movement_left(&self) -> u32;
    fn max_movement(&self) -> u32;
    fn combat_strength(&self) -> Option<u32>;  // None = civilian/non-combat
    fn promotions(&self) -> &[PromotionId];
    fn health(&self) -> u32;
    fn max_health(&self) -> u32 { 100 }
    fn is_alive(&self) -> bool { self.health() > 0 }
}

struct BasicUnit { /* all Unit fields as pub */ }
// impl Unit for BasicUnit
```

### Cities

```rust
enum CityKind { Regular, CityState(CityStateData) }

enum CityOwnership { Normal, Occupied, Puppet, Razed }

enum WallLevel { None, Ancient, Medieval, Renaissance }
// defense_bonus() -> i32; max_hp() -> u32

enum ProductionItem {
    Unit(UnitTypeId), Building(BuildingId),
    District(DistrictTypeId), Wonder(WonderId),
}

struct City {
    pub id: CityId,
    pub name: String,
    pub owner: CivId,
    pub founded_by: CivId,
    pub coord: HexCoord,
    pub kind: CityKind,
    pub ownership: CityOwnership,
    pub is_capital: bool,
    pub population: u32,
    pub food_stored: u32,
    pub food_to_grow: u32,
    pub production_stored: u32,
    pub current_production: Option<ProductionItem>,
    pub walls: WallLevel,
    pub wall_hp: u32,
    pub buildings: Vec<BuildingId>,
    pub districts: Vec<DistrictTypeId>,
    pub yields: YieldBundle,
}
```

`City` does not derive `Clone` (contains `CityKind` which holds `CityStateData`). Yields, housing, amenities are computed at query time by the rules engine — the `yields` field is a cache only.

### City States

```rust
enum CityStateType { Cultural, Industrial, Militaristic, Religious, Scientific, Trade }

trait CityStateBonus: Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn yields_for_suzerain(&self) -> YieldBundle;
}

struct CityStateData {
    pub state_type: CityStateType,
    pub suzerain: Option<CivId>,
    pub influence: HashMap<CivId, i32>,
}
// is_suzerain(), get_influence(), recalculate_suzerain()
```

City states are stored as `City` with `kind = CityKind::CityState(_)`.

### Districts and Buildings

```rust
trait DistrictDef: Debug {
    fn id(&self) -> DistrictTypeId;
    fn name(&self) -> &'static str;
    fn base_cost(&self) -> u32;
    fn max_per_city(&self) -> u32 { 1 }
}

trait BuildingDef: Debug {
    fn id(&self) -> BuildingId;
    fn name(&self) -> &'static str;
    fn cost(&self) -> u32;
    fn maintenance(&self) -> u32;
    fn yields(&self) -> YieldBundle;
    fn requires_district(&self) -> Option<DistrictTypeId>;
}

struct AdjacencyContext {
    pub adjacent_districts: Vec<DistrictTypeId>,
    pub adjacent_natural_wonders: Vec<NaturalWonderId>,
    pub adjacent_mountains: u32,
    pub adjacent_rivers: u32,
    pub adjacent_rainforest: u32,
}

struct PlacedDistrict {
    pub district_type: DistrictTypeId,
    pub city_id: CityId,
    pub coord: HexCoord,
    pub buildings: Vec<BuildingId>,
    pub is_pillaged: bool,
}
```

### Civilizations and Leaders

```rust
struct Civilization {
    pub id: CivId,
    pub name: &'static str,
    pub adjective: &'static str,
    pub leader: Leader,
    pub cities: Vec<CityId>,
    pub capital: Option<CityId>,
    pub current_era: AgeType,
    pub researched_techs: Vec<TechId>,
    pub tech_in_progress: Option<TechProgress>,
    pub completed_civics: Vec<CivicId>,
    pub civic_in_progress: Option<CivicProgress>,
    pub current_government: Option<GovernmentId>,
    pub active_policies: Vec<PolicyId>,
    pub gold: i32,
    pub treasury_per_turn: i32,
    pub yields: YieldBundle,
    pub strategic_resources: HashMap<ResourceId, u32>,
}

struct Leader {
    pub name: &'static str,
    pub civ_id: CivId,
    pub abilities: Vec<Box<dyn LeaderAbility>>,
    pub agenda: Box<dyn Agenda>,
}

trait LeaderAbility: Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn modifiers(&self) -> Vec<Modifier>;
}

trait Agenda: Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn attitude(&self, toward: CivId) -> i32;
}

trait StartBias: Debug {
    fn terrain_preference(&self) -> Option<TerrainId>;
    fn feature_preference(&self) -> Option<FeatureId>;
    fn resource_preference(&self) -> Option<ResourceCategory>;
}

struct TechProgress  { pub tech_id: TechId,  pub progress: u32, pub boosted: bool }
struct CivicProgress { pub civic_id: CivicId, pub progress: u32, pub inspired: bool }
```

`Civilization` does not derive `Clone` (contains `Leader` with `Box<dyn>` fields).

### Diplomacy

```rust
enum DiplomaticStatus { War, Denounced, Neutral, Friendly, Alliance }

struct DiplomaticRelation {
    pub civ_a: CivId,
    pub civ_b: CivId,
    pub status: DiplomaticStatus,
    pub grievances_a_against_b: Vec<GrievanceRecord>,
    pub grievances_b_against_a: Vec<GrievanceRecord>,
    pub active_agreements: Vec<AgreementId>,
    pub turns_at_war: u32,
}
// is_at_war(), add_grievance(), opinion_score_a_toward_b(), opinion_score_b_toward_a()

trait Agreement: Debug {
    fn id(&self) -> AgreementId;
    fn name(&self) -> &'static str;
    fn duration_turns(&self) -> Option<u32>;
    fn is_expired(&self, current_turn: u32, signed_turn: u32) -> bool;
}

enum GrievanceVisibility { Public, RequiresSpy, RequiresAlliance }

struct GrievanceRecord {
    pub grievance_id: GrievanceId,
    pub description: &'static str,
    pub amount: i32,
    pub visibility: GrievanceVisibility,
    pub recorded_turn: u32,
}

trait GrievanceTrigger: Debug {
    fn description(&self) -> &'static str;
    fn grievance_amount(&self) -> i32;
    fn visibility(&self) -> GrievanceVisibility { GrievanceVisibility::Public }
}
// Built-in: DeclaredWarGrievance (+30), PillageGrievance (+5), CapturedCityGrievance (+20)
```

### Governors

```rust
trait GovernorDef: Debug {
    fn id(&self) -> GovernorId;
    fn name(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn base_ability_description(&self) -> &'static str;
}

trait GovernorPromotion: Debug {
    fn id(&self) -> PromotionId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn requires(&self) -> Vec<PromotionId>;
}

struct Governor {
    pub id: GovernorId,
    pub def_name: &'static str,
    pub owner: CivId,
    pub assigned_city: Option<CityId>,
    pub promotions: Vec<PromotionId>,
    pub turns_to_establish: u32,
}
// is_established() -> turns_to_establish == 0
```

Built-in governors defined via macro: `Liang`, `Magnus`, `Amani`, `Victor`, `Pingala`, `Reyna`, `Ibrahim`.

### Religion

```rust
trait Belief: Debug {
    fn id(&self) -> BeliefId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
}

struct BeliefContext { pub followers: u32, pub holy_cities: u32 }

struct Religion {
    pub id: ReligionId,
    pub name: String,
    pub founded_by: CivId,
    pub holy_city: CityId,
    pub beliefs: Vec<BeliefId>,
    pub followers: HashMap<CityId, u32>,
}
// total_followers()
```

### Great People

```rust
trait GreatPersonAbility: Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn uses(&self) -> Option<u32>;  // None = unlimited / passive
}

struct GreatPerson {
    pub id: GreatPersonId,
    pub name: &'static str,
    pub person_type: GreatPersonType,
    pub era: &'static str,
    pub owner: Option<CivId>,
    pub coord: Option<HexCoord>,
    pub ability_names: Vec<&'static str>,
    pub is_retired: bool,
}
```

### Eras and Trade Routes

```rust
trait EraTrigger: Debug {
    fn description(&self) -> &'static str;
    fn is_triggered(&self) -> bool;
}

struct Era {
    pub id: EraId,
    pub name: &'static str,
    pub age: AgeType,
    pub tech_count: u32,
    pub civic_count: u32,
}

struct TradeRoute {
    pub id: TradeRouteId,
    pub origin: CityId,
    pub destination: CityId,
    pub owner: CivId,
    pub origin_yields: YieldBundle,
    pub destination_yields: YieldBundle,
    pub turns_remaining: Option<u32>,
}
```

---

## 6. libciv — game/

### WorldBoard

`WorldBoard` implements `HexBoard<Tile=WorldTile, Edge=WorldEdge>`. Tiles are stored in row-major order (`index = r * width + q`) in a flat `Vec<WorldTile>`. Edges are stored in a `HashMap<(HexCoord, HexDir), WorldEdge>` using canonical forward-half keys only.

Default topology is `CylindricalEW` (east-west wrapping). The `coord_to_index` and `normalize_coord` methods apply `q.rem_euclid(width)` for cylindrical/toroidal topologies.

```rust
impl WorldBoard {
    fn new(width: u32, height: u32) -> Self;
    fn set_edge(&mut self, coord: HexCoord, dir: HexDir, edge: WorldEdge);
    fn find_path(&self, start: HexCoord, goal: HexCoord, movement_budget: u32) -> Option<Vec<HexCoord>>;
    fn has_los(&self, from: HexCoord, to: HexCoord) -> bool;
}
```

Pathfinding uses Dijkstra with a min-heap. Movement cost = tile cost + edge crossing cost; impassable tiles/edges are skipped. Road cost override is not yet applied (Phase 2).

LOS uses hex ray interpolation (floating-point lerp, then `hex_round`). A tile blocks LOS if its elevation is strictly above `min(from_elev, to_elev)`. The `Elevation::High` check is currently incomplete (see Phase 2).

### GameState

```rust
struct GameState {
    pub turn: u32,
    pub seed: u64,
    pub board: WorldBoard,
    pub id_gen: IdGenerator,
    pub civilizations: Vec<Civilization>,
    pub cities: Vec<City>,
    pub diplomatic_relations: Vec<DiplomaticRelation>,
    pub religions: Vec<Religion>,
    pub trade_routes: Vec<TradeRoute>,
    pub great_people: Vec<GreatPerson>,
    pub tech_tree: TechTree,
    pub civic_tree: CivicTree,
    pub governments: Vec<Government>,
    pub policies: Vec<Policy>,
    pub current_era: EraId,
}
// civ(CivId), city(CityId), city_state_by_civ(CivId) helpers
```

Collections are `Vec` rather than `HashMap` at this stage; lookups are linear scans. Phase 2 may introduce indexed maps if profiling warrants it.

### IdGenerator

Deterministic ULID generation from a seeded `SmallRng`. Each call to `next_ulid()` advances a monotonically increasing fake timestamp and draws random bits from the RNG. Same seed always produces the same ID sequence.

### Rules Engine

```rust
trait RulesEngine: Debug {
    fn move_unit(&self, state: &GameState, unit: UnitId, to: HexCoord)
        -> Result<GameStateDiff, RulesError>;
    fn compute_yields(&self, state: &GameState, civ: CivId) -> YieldBundle;
    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff;
}

enum RulesError {
    UnitNotFound, DestinationImpassable, InsufficientMovement, InvalidCoord, NotYourTurn,
}

struct DefaultRulesEngine;  // all methods are todo!() stubs
```

### Semantic Diffs

```rust
enum StateDelta {
    TurnAdvanced      { from: u32, to: u32 },
    UnitMoved         { unit: UnitId, from: HexCoord, to: HexCoord },
    UnitCreated       { unit: UnitId, coord: HexCoord, owner: CivId },
    UnitDestroyed     { unit: UnitId },
    CityFounded       { city: CityId, coord: HexCoord, owner: CivId },
    CityCaptured      { city: CityId, new_owner: CivId, old_owner: CivId },
    GoldChanged       { civ: CivId, delta: i32 },
    TechResearched    { civ: CivId, tech: &'static str },
    CivicCompleted    { civ: CivId, civic: &'static str },
    DiplomacyChanged  { civ_a: CivId, civ_b: CivId, new_status: String },
}

struct GameStateDiff { pub deltas: Vec<StateDelta> }
// push(), is_empty(), len()
```

All rules operations return `GameStateDiff` to support replay and RL observation.

---

## Phase 2 — Failing Tests to Fix

The following tests are `#[ignore]`d and define the Phase 2 acceptance criteria:

### 1. Modifier stacking — `rules/modifier.rs`

**Test:** `test_modifier_stacking_additive`
Fix `resolve_modifiers()` to group modifiers by `(effect discriminant, StackingRule)`. For `Additive`, sum all `YieldFlat` amounts for the same `YieldType` and return a single `EffectType` with the total. Input: two `YieldFlat(Production, 2)` and `YieldFlat(Production, 3)`, both `Additive`. Expected: one effect with value 5.

**Test:** `test_modifier_stacking_max`
For `Max`, return only the largest value per `(YieldType, effect kind)` group. Input: `YieldFlat(Production, 2)`, `YieldFlat(Production, 5)`, `YieldFlat(Production, 3)`, all `Max`. Expected: one effect with value 5.

**Test:** `test_modifier_stacking_replace`
For `Replace`, return only the last modifier in the slice. Input: `YieldFlat(Production, 2)` then `YieldFlat(Production, 7)`, both `Replace`. Expected: exactly one effect, value 7.

### 4. LOS blocked by high elevation — `game/board.rs`

**Test:** `test_los_blocked_by_high`
`has_los(from, to)` must return `false` when an intermediate tile has `Elevation::High` and both endpoints are at lower elevation. Currently the stub sets the blocker tile to `Grassland` (flat) rather than a mountain, and the LOS ray check using `elevation() > min_elev` must correctly block when a tile is `Elevation::High`. Fix the test setup to assign a mountain tile at the midpoint, and verify the ray check blocks it.

### 5. Dijkstra prefers roads — `game/board.rs`

**Test:** `test_dijkstra_prefers_roads`
`find_path()` must apply `road.movement_cost()` as an override when a `WorldTile` has a `road: Some(...)`. A path through road tiles should be preferred over an equivalent path through unroaded tiles. Implement road cost substitution in the Dijkstra loop: when `tile.road.is_some()`, use `road.as_def().movement_cost()` instead of `tile.terrain.as_def().movement_cost()`.

### Stubbed rules engine methods

- `DefaultRulesEngine::move_unit()` — validate that the unit exists, find a path to `to` within `unit.movement_left` budget, check destination is not impassable, update `unit.coord` and deduct movement, return `StateDelta::UnitMoved`.
- `DefaultRulesEngine::compute_yields()` — sum `tile.total_yields()` for all tiles owned by the civ; add building yields from city buildings; apply resolved `Modifier`s.
- `DefaultRulesEngine::advance_turn()` — accumulate food/production/science/gold per city, trigger growth when `food_stored >= food_to_grow`, complete production when `production_stored >= item_cost`, advance research progress.

---

## Phase 3 — Gameplay Systems (ordered)

1. Tile yield calculation and worked tile assignment per city
2. City food accumulation and population growth/decline
3. City production queue and build completion
4. Unit movement validation and combat resolution
5. District placement with adjacency bonus calculation
6. Tech and civic research with eureka/inspiration trigger evaluation
7. Trade route creation, path resolution, yield delivery
8. Policy card slot enforcement and government switching
9. Diplomacy state machine: opinion modifiers, grievance decay, war/peace transitions
10. Religion founding, missionary spread, majority religion per city
11. Great person point accumulation and recruitment
12. Governor assignment, establishment timer, promotion effects
13. Era transition evaluation via `EraTrigger`
14. Victory condition evaluation

---

## Conventions

- VCS: jj (Jujutsu)
- Commit style: conventional commits (`infra:`, `impl:`, `tests:`, `fix:`, `docs:`)
- Movement costs scaled by 100 (integer math throughout): `ONE=100`, `TWO=200`, `THREE=300`
- All built-in content names are `&'static str`; no heap allocation for known-at-compile-time strings
- `Box<dyn Trait>` fields prevent `Clone`; document this on affected structs
