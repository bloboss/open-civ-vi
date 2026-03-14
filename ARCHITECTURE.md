# Open Civ VI — Architecture

## Design Philosophy

1. **Invalid states unrepresentable** — encode constraints in types, not runtime checks
2. **Plain enums over trait objects for built-in content** — terrain, features, resources, and improvements are plain enums with direct `match` dispatch; traits remain as the extension point for user content
3. **Separated concerns** — geometry knows nothing of game concepts; world state knows nothing of geometry
4. **Single GameState** — one struct passed by reference to all systems; no global state
5. **Typed modifier pipeline** — every yield change is a `Modifier`; no opaque callbacks
6. **Semantic diffs** — all `RulesEngine` operations return `GameStateDiff` to support replay and RL observation

`&'static str` is used for all built-in names (compile-time content). Structs with `Box<dyn Trait>` fields (`Leader`, `Civilization`) do not derive `Clone`. `libhexgrid` has zero knowledge of terrain, civilizations, or rules.

---

## Library Structure

```
libhexgrid  (no deps)       — pure hex geometry: coords, traits, topology
libciv      (→ libhexgrid)  — all game state: ids, yields, enums, world, civ, rules, game, ai
civsim      (→ libciv)      — CLI binary (`new`, `run`, `demo`, `ai-demo`, `play`)
open4x-web  (→ libciv)      — Leptos/WASM frontend
```

---

## 1. libhexgrid

### Coordinates

```rust
struct HexCoord { pub q: i32, pub r: i32, pub s: i32 }
```

Invariant `q + r + s = 0` enforced at construction. `HexCoord::new(q, r, s)` returns `Result<Self, HexCoordError>`. `HexCoord::from_qr(q, r)` computes `s` automatically. Derives `PartialOrd, Ord` for use in `BinaryHeap`.

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
    Level(u8),  // Level(0)=flat, Level(1)=hills
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
    fn coord(&self) -> HexCoord;   // canonical forward-half endpoint
    fn dir(&self) -> HexDir;       // always forward-half: E, NE, or NW
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

Edges are stored canonically: forward-half directions only (`{E, NE, NW}`). Backward-half edge lookups (`{W, SW, SE}`) normalize to the adjacent tile with the opposite direction. Use `WorldBoard::set_edge()` for automatic canonicalization.

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

`YieldBundle` is a flat struct with named `i32` fields (not a HashMap).

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
// impl Add, AddAssign; with(), get(), add_yield(), merge()
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

`BuiltinTerrain` is a **plain enum** with direct `match`-arm dispatch — no inner structs, no `TerrainDef` trait:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum BuiltinTerrain {
    #[default] Grassland,
    Plains, Desert, Tundra, Snow, Coast, Ocean, Mountain,
}

impl BuiltinTerrain {
    fn name(self) -> &'static str;
    fn base_yields(self) -> YieldBundle;
    fn movement_cost(self) -> MovementCost;
    fn elevation(self) -> Elevation;
    fn is_land(self) -> bool;
    fn is_water(self) -> bool;
}
```

`Mountain` terrain is `Elevation::High` (impassable). Hills are encoded as `WorldTile::hills: bool` separate from terrain.

### Features

`BuiltinFeature` is also a **plain enum**:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BuiltinFeature {
    Forest, Rainforest, Marsh, Floodplain, Reef, Ice, VolcanicSoil, Oasis,
}

impl BuiltinFeature {
    fn name(self) -> &'static str;
    fn yield_modifier(self) -> YieldBundle;
    fn movement_cost_modifier(self) -> MovementCost;
    fn conceals_resources(self) -> bool;
}
```

`Ice` has `movement_cost_modifier() = Impassable`. Feature movement cost is additive with terrain cost; Impassable wins.

### Resources

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BuiltinResource {
    // Bonus: Wheat, Rice, Cattle, Sheep, Fish, Stone, Copper, Deer
    // Luxury: Wine, Silk, Spices, Incense, Cotton, Ivory, Sugar, Salt
    // Strategic: Horses, Iron, Coal, Oil, Aluminum, Niter, Uranium
}

impl BuiltinResource {
    fn name(self) -> &'static str;
    fn category(self) -> ResourceCategory;
    fn base_yields(self) -> YieldBundle;
    fn reveal_tech(self) -> Option<&'static str>;  // tech name required to see this resource
}
```

### Edge Features

```rust
trait EdgeFeatureDef: Debug {
    fn name(&self) -> &'static str;
    fn crossing_cost(&self) -> MovementCost;
}

enum BuiltinEdgeFeature {
    River(River),             // crossing_cost = THREE (300)
    Canal(Canal),             // crossing_cost = ONE (100)
    MountainPass(MountainPass), // crossing_cost = TWO (200)
}
// BuiltinEdgeFeature::as_def() -> &dyn EdgeFeatureDef
```

### WorldTile and WorldEdge

```rust
struct WorldTile {
    pub coord: HexCoord,
    pub terrain: BuiltinTerrain,
    pub hills: bool,                        // elevation modifier; separate from terrain
    pub feature: Option<BuiltinFeature>,
    pub resource: Option<BuiltinResource>,
    pub improvement: Option<BuiltinImprovement>,
    pub improvement_pillaged: bool,
    pub road: Option<BuiltinRoad>,
    pub rivers: Vec<BuiltinEdgeFeature>,    // river crossings adjacent to this tile
    pub natural_wonder: Option<BuiltinNaturalWonder>,
    pub owner: Option<CivId>,
}
// impl HexTile; total_yields() = terrain + feature + resource(gated) + improvement(if !pillaged)
// terrain_defense_bonus() -> i32
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
struct ImprovementRequirements {
    pub requires_land: bool,
    pub requires_water: bool,
    pub elevation: ElevationReq,            // Any | Flat | HillsOrMore | NotMountain
    pub blocked_terrains: &'static [BuiltinTerrain],
    pub required_feature: Option<BuiltinFeature>,
    pub conditional_features: &'static [(BuiltinTerrain, &'static [BuiltinFeature])],
    pub required_resource: Option<BuiltinResource>,
    pub required_tech: Option<&'static str>,
    pub required_civic: Option<&'static str>,
    pub proximity: Option<ProximityReq>,    // AdjacentTerrain | AdjacentFeature | AdjacentResource
}

enum BuiltinImprovement { Farm, Mine, LumberMill, TradingPost, Fort, Airstrip, MissileSilo }

impl BuiltinImprovement {
    fn name(self) -> &'static str;
    fn yield_bonus(self) -> YieldBundle;
    fn build_turns(self) -> u32;
    fn requirements(self) -> ImprovementRequirements;
}
```

Full validation (terrain, elevation, feature, resource, proximity, tech, civic) is enforced by `RulesEngine::place_improvement()`.

### Roads

```rust
trait RoadDef: Debug {
    fn name(&self) -> &'static str;
    fn movement_cost(&self) -> MovementCost;
    fn maintenance(&self) -> u32;
}

enum BuiltinRoad {
    Ancient(AncientRoad),       // Cost(50), maintenance 0
    Medieval(MedievalRoad),     // Cost(50), maintenance 1
    Industrial(IndustrialRoad), // Cost(25), maintenance 2
    Railroad(Railroad),         // Cost(10), maintenance 3
}
// BuiltinRoad::as_def() -> &dyn RoadDef
```

Road cost overrides tile cost in Dijkstra when `tile.road.is_some()`.

### Natural Wonders

```rust
trait NaturalWonder: Debug + Send + Sync {
    fn id(&self) -> NaturalWonderId;
    fn name(&self) -> &'static str;
    fn appeal_bonus(&self) -> i32;
    fn yield_bonus(&self) -> YieldBundle;
    fn movement_cost(&self) -> MovementCost;
    fn impassable(&self) -> bool;
}

enum BuiltinNaturalWonder {
    Krakatoa(Krakatoa),
    GrandMesa(GrandMesa),
    CliffsOfDover(CliffsOfDover),
    UluruAyersRock(UluruAyersRock),
    GalapagosIslands(GalapagosIslands),
}
// BuiltinNaturalWonder::as_def() -> &dyn NaturalWonder
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
    YieldPercent(YieldType, i32),
    CombatStrengthFlat(i32),
    CombatStrengthPercent(i32),
    MovementBonus(u32),
}

enum TargetSelector { AllTiles, AllUnits, UnitDomain(UnitDomain), Civilization(CivId), Global }
enum StackingRule { Additive, Max, Replace }
enum ModifierSource {
    Tech(&'static str), Civic(&'static str), Policy(&'static str),
    Building(&'static str), Wonder(&'static str), Leader(&'static str),
    Religion(&'static str), Era(&'static str), Custom(&'static str),
}
```

`resolve_modifiers(modifiers: &[Modifier]) -> Vec<EffectType>` groups by `(effect discriminant, StackingRule)`: `Additive` sums, `Max` keeps highest, `Replace` keeps last.

### One-Shot Effects

Discrete irreversible state mutations triggered by tech/civic/wonder completion. Processed in a dedicated drain phase of `advance_turn`. Each variant carries its own cascade class and idempotency guard.

```rust
enum OneShotEffect {
    RevealResource(BuiltinResource),
    UnlockUnit(&'static str),
    UnlockBuilding(&'static str),
    UnlockImprovement(&'static str),
    TriggerEureka { tech: &'static str },
    TriggerInspiration { civic: &'static str },
    FreeUnit { unit_type: &'static str, city: Option<CityId> },
    FreeBuilding { building: &'static str, city: Option<CityId> },
    UnlockGovernment(&'static str),
    AdoptGovernment(&'static str),
    UnlockPolicy(&'static str),
    GrantModifier(Modifier),
}
// guard(&Civilization) -> bool  — idempotency check before applying
// cascade_class() -> CascadeClass  — NonCascading | Idempotent
```

### Technology and Civics

```rust
struct TechNode {
    pub id: TechId,
    pub name: &'static str,
    pub cost: u32,
    pub prerequisites: Vec<TechId>,
    pub effects: Vec<OneShotEffect>,        // applied on completion
    pub eureka_description: &'static str,
    pub eureka_effects: Vec<OneShotEffect>, // applied if Eureka triggered
}

struct CivicNode {
    pub id: CivicId,
    pub name: &'static str,
    pub cost: u32,
    pub prerequisites: Vec<CivicId>,
    pub effects: Vec<OneShotEffect>,
    pub inspiration_description: &'static str,
    pub inspiration_effects: Vec<OneShotEffect>,
}

struct TechTree  { pub nodes: HashMap<TechId, TechNode>  }
struct CivicTree { pub nodes: HashMap<CivicId, CivicNode> }
// add_node(), get(), prerequisites_met()
```

Tech trees are built at game init via `build_tech_tree(ids)` / `build_civic_tree(ids)`. Tech names are matched by `&'static str` for tech-gating and improvement requirements.

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
struct VictoryProgress { pub victory_id: VictoryId, pub civ_id: CivId, pub current: u32, pub target: u32 }
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
    fn combat_strength(&self) -> Option<u32>;
    fn promotions(&self) -> &[PromotionId];
    fn health(&self) -> u32;
    fn max_health(&self) -> u32 { 100 }
    fn is_alive(&self) -> bool { self.health() > 0 }
    fn range(&self) -> u8;
    fn vision_range(&self) -> u8;
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
    pub production_queue: VecDeque<ProductionItem>,
    pub walls: WallLevel,
    pub wall_hp: u32,
    pub buildings: Vec<BuildingId>,
    pub districts: Vec<DistrictTypeId>,
    pub worked_tiles: Vec<HexCoord>,
    pub locked_tiles: HashSet<HexCoord>,  // tiles pinned by the player
}
```

Yields, amenities, housing are **not stored** on City — computed by `RulesEngine::compute_yields()` at query time so modifiers always apply correctly. The base rule: every city contributes 1 science/turn before modifiers.

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

City states are stored as `City` with `kind = CityKind::CityState(_)`. Access via `GameState::city_state_by_civ(CivId)`.

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
    pub current_era: AgeType,
    // Research:
    pub researched_techs: Vec<TechId>,
    pub research_queue: VecDeque<TechProgress>,  // front is active tech
    pub completed_civics: Vec<CivicId>,
    pub civic_in_progress: Option<CivicProgress>,
    // Government:
    pub current_government: Option<GovernmentId>,
    pub current_government_name: Option<&'static str>,
    pub active_policies: Vec<PolicyId>,
    // Economy:
    pub gold: i32,
    pub strategic_resources: HashMap<ResourceId, u32>,
    // One-shot tracking (idempotency guards):
    pub revealed_resources: HashSet<BuiltinResource>,
    pub eureka_triggered: HashSet<&'static str>,
    pub inspiration_triggered: HashSet<&'static str>,
    pub unlocked_governments: Vec<&'static str>,
    pub unlocked_policies: Vec<&'static str>,
    pub unlocked_units: Vec<&'static str>,
    pub unlocked_buildings: Vec<&'static str>,
    pub unlocked_improvements: Vec<&'static str>,
    // Fog of war:
    pub visible_tiles: HashSet<HexCoord>,
    pub explored_tiles: HashSet<HexCoord>,
}

struct TechProgress  { pub tech_id: TechId,  pub progress: u32, pub boosted: bool }
struct CivicProgress { pub civic_id: CivicId, pub progress: u32, pub inspired: bool }

struct Leader {
    pub name: &'static str,
    pub civ_id: CivId,
    pub abilities: Vec<Box<dyn LeaderAbility>>,
    pub agenda: Box<dyn Agenda>,
}

trait LeaderAbility: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn modifiers(&self) -> Vec<Modifier>;
}

trait Agenda: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn attitude(&self, toward: CivId) -> i32;
}
```

`Civilization` does not derive `Clone` (contains `Leader` with `Box<dyn>` fields).

### Diplomacy

```rust
enum DiplomaticStatus { War, Denounced, Neutral, Friendly, Alliance }

struct DiplomaticRelation {
    pub civ_a: CivId, pub civ_b: CivId,
    pub status: DiplomaticStatus,
    pub grievances_a_against_b: Vec<GrievanceRecord>,
    pub grievances_b_against_a: Vec<GrievanceRecord>,
    pub active_agreements: Vec<AgreementId>,
    pub turns_at_war: u32,
}
// is_at_war(), add_grievance(), opinion_score_a_toward_b(), opinion_score_b_toward_a()

struct GrievanceRecord {
    pub grievance_id: GrievanceId,
    pub description: &'static str,
    pub amount: i32,
    pub visibility: GrievanceVisibility,  // Public | RequiresSpy | RequiresAlliance
    pub recorded_turn: u32,
}
```

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

No spread, conversion, or majority-religion mechanics are implemented.

### Trade Routes

```rust
struct TradeRoute {
    pub id: TradeRouteId,
    pub origin: CityId,
    pub destination: CityId,
    pub owner: CivId,
    pub origin_yields: YieldBundle,
    pub destination_yields: YieldBundle,
    pub turns_remaining: Option<u32>,
}
// is_international() -> bool  (stub: always false)
```

No trader unit, route creation, or yield delivery is wired into `advance_turn`.

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

Pathfinding uses Dijkstra with a min-heap. Road cost substitution is active: when `tile.road.is_some()`, the road's `movement_cost()` is used instead of terrain cost. LOS uses hex ray interpolation; `Elevation::High` blocks LOS.

### GameState

```rust
struct GameState {
    pub turn: u32,
    pub seed: u64,
    pub board: WorldBoard,
    pub id_gen: IdGenerator,
    pub civilizations: Vec<Civilization>,
    pub cities: Vec<City>,
    pub units: Vec<BasicUnit>,
    pub diplomatic_relations: Vec<DiplomaticRelation>,
    pub religions: Vec<Religion>,
    pub trade_routes: Vec<TradeRoute>,
    pub great_people: Vec<GreatPerson>,
    pub tech_tree: TechTree,
    pub civic_tree: CivicTree,
    pub governments: Vec<Government>,
    pub policies: Vec<Policy>,
    pub current_era: EraId,
    pub unit_type_defs: Vec<UnitTypeDef>,   // production registry
    pub building_defs: Vec<BuildingDef>,     // production registry
    pub effect_queue: VecDeque<(CivId, OneShotEffect)>,  // drained each turn
}
// civ(), city(), unit(), unit_mut(), city_state_by_civ() helpers
```

### IdGenerator

Deterministic ULID generation from a seeded `SmallRng`. Same seed always produces the same ID sequence. Used for reproducible game state.

### Rules Engine

```rust
trait RulesEngine: Debug {
    fn move_unit(&self, state: &GameState, unit: UnitId, to: HexCoord)
        -> Result<GameStateDiff, RulesError>;
    fn compute_yields(&self, state: &GameState, civ: CivId) -> YieldBundle;
    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff;
    fn assign_citizen(&self, state: &mut GameState, city: CityId, tile: HexCoord, lock: bool)
        -> Result<GameStateDiff, RulesError>;
    fn assign_policy(&self, state: &mut GameState, civ: CivId, policy: PolicyId)
        -> Result<GameStateDiff, RulesError>;
    fn declare_war(&self, state: &mut GameState, aggressor: CivId, target: CivId)
        -> Result<GameStateDiff, RulesError>;
    fn make_peace(&self, state: &mut GameState, civ_a: CivId, civ_b: CivId)
        -> Result<GameStateDiff, RulesError>;
    fn attack(&self, state: &mut GameState, attacker: UnitId, defender: UnitId)
        -> Result<GameStateDiff, RulesError>;
    fn found_city(&self, state: &mut GameState, settler: UnitId, name: String)
        -> Result<GameStateDiff, RulesError>;
    fn place_improvement(&self, state: &mut GameState, civ_id: CivId, coord: HexCoord,
                         improvement: BuiltinImprovement) -> Result<GameStateDiff, RulesError>;
}

struct DefaultRulesEngine;  // zero-sized; implements all RulesEngine methods
```

`advance_turn` phases: (1) food accumulation + population growth + auto citizen assign, (2) production accumulation, (3) gold/science/culture collection → tech and civic progress + completion, (4) effect queue drain, (5) diplomacy decay + war timer, (6) turn counter increment.

### Semantic Diffs

```rust
enum StateDelta {
    TurnAdvanced { from: u32, to: u32 },
    UnitMoved { unit: UnitId, from: HexCoord, to: HexCoord, cost: u32 },
    UnitCreated { unit: UnitId, coord: HexCoord, owner: CivId },
    UnitDestroyed { unit: UnitId },
    CityFounded { city: CityId, coord: HexCoord, owner: CivId },
    CityCaptured { city: CityId, new_owner: CivId, old_owner: CivId },
    PopulationGrew { city: CityId, new_population: u32 },
    GoldChanged { civ: CivId, delta: i32 },
    TechResearched { civ: CivId, tech: &'static str },
    CivicCompleted { civ: CivId, civic: &'static str },
    DiplomacyChanged { civ_a: CivId, civ_b: CivId, new_status: DiplomaticStatus },
    ResourceRevealed { civ: CivId, resource: BuiltinResource },
    EurekaTriggered { civ: CivId, tech: &'static str },
    InspirationTriggered { civ: CivId, civic: &'static str },
    UnitUnlocked { civ: CivId, unit_type: &'static str },
    BuildingUnlocked { civ: CivId, building: &'static str },
    ImprovementUnlocked { civ: CivId, improvement: &'static str },
    GovernmentUnlocked { civ: CivId, government: &'static str },
    GovernmentAdopted { civ: CivId, government: &'static str },
    PolicyUnlocked { civ: CivId, policy: &'static str },
    PolicyUnslotted { civ: CivId, policy: PolicyId },
    PolicyAssigned { civ: CivId, policy: PolicyId },
    FreeUnitGranted { civ: CivId, unit_type: &'static str, coord: HexCoord },
    FreeBuildingGranted { civ: CivId, building: &'static str, city: CityId },
    BuildingCompleted { city: CityId, building: &'static str },
    DistrictBuilt { city: CityId, district: &'static str, coord: HexCoord },
    WonderBuilt { civ: CivId, wonder: &'static str, city: CityId },
    ProductionStarted { city: CityId, item: &'static str },
    CitizenAssigned { city: CityId, tile: HexCoord },
    UnitAttacked { attacker: UnitId, defender: UnitId, attack_type: AttackType,
                   attacker_damage: u32, defender_damage: u32 },
    TilesRevealed { civ: CivId, coords: Vec<HexCoord> },
    ImprovementPlaced { coord: HexCoord, improvement: BuiltinImprovement },
}

struct GameStateDiff { pub deltas: Vec<StateDelta> }
// push(), is_empty(), len()
```

All rules operations return `GameStateDiff` to support replay and RL observation.

---

## 7. libciv — ai/

```rust
trait Agent: Debug {
    fn take_turn(&self, state: &mut GameState, rules: &dyn RulesEngine) -> GameStateDiff;
}

struct HeuristicAgent { civ_id: CivId }
// Deterministic. On each turn: moves units toward unexplored tiles (highest
// unexplored-score neighbor), queues a combat unit when production queue is empty.
```

---

## 8. Remaining Work

The following systems have data structures defined but no gameplay logic implemented. They are ordered roughly by dependency.

### 8.1 — Coherent Map Generation

**Status:** `randomize_terrain()` in `civsim` is pure noise. No continent shapes, resource distribution, or starting-position logic.

**Needed:**
- Continent/island generation (e.g. Perlin noise + flood-fill)
- Guaranteed habitable starting regions per civ
- Resource scatter with strategic/luxury quotas
- Natural wonder placement (exactly one tile, specific terrain requirements)

### 8.3 — City Defenses and Ranged Attacks

**Status:** `WallLevel` has `defense_bonus()` and `max_hp()`. `city.wall_hp` tracks damage. Combat resolution applies terrain bonuses. No city-initiated attacks exist.

**Needed:**
- City ranged attack action in `RulesEngine` (fires at nearest enemy unit each turn)
- `WallLevel` defense bonus applied in melee/ranged damage formula
- `city.wall_hp` reduced when city takes damage; wall destruction events
- Siege unit type with bonus vs. cities

### 8.4 — Trade Routes and Trader Units

**Status:** `TradeRoute` struct and `GameState.trade_routes` exist. `is_international()` is a stub returning `false`. No trader unit type or route creation logic exists.

**Needed:**
- Trader unit (`UnitCategory::Trader`) with establish-route action
- `RulesEngine::establish_trade_route()` validating path between cities
- Yield calculation based on origin/destination city attributes and international status
- Route delivery wired into `advance_turn` (add `origin_yields` to civ each turn)
- Route expiry / cancellation when cities are captured

### 8.5 — Religion System

**Status:** `Religion`, `Belief` trait, `BeliefContext` defined. `GameState.religions` exists. `religion_founder_yields()` is a stub returning zero. No founding, spread, or majority-religion mechanics exist.

**Needed:**
- `RulesEngine::found_religion()` (requires Great Prophet unit; sets holy city, initial beliefs)
- Missionary and Apostle unit types with spread actions
- Per-city religion pressure and majority-religion tracking
- `Belief` modifier integration in `compute_yields`
- Religion spread wired into `advance_turn`

### 8.6 — Culture Borders, Loyalty, and Tourism

**Status:** Culture is computed as a yield and accumulates civic progress. No territorial expansion, loyalty pressure, or tourism comparison is implemented.

**Needed:**
- Cultural border expansion: tile acquisition triggered by culture accumulation
- Loyalty system: per-city loyalty score influenced by adjacent city culture output, amenities, governors; cities with 0 loyalty revolt
- Tourism generation from wonders, national parks, great works
- Culture victory: check if a civ's tourism exceeds every other civ's home culture

### 8.7 — Strategic Resource Consumption for Unit Production

**Status:** `Civilization.strategic_resources` tracks stockpile. `UnitTypeDef` has no resource cost field. Production completion in `civsim` deducts production but not resources.

**Needed:**
- `resource_cost: Option<(BuiltinResource, u32)>` field on `UnitTypeDef`
- Deduct resources from `civ.strategic_resources` on unit production completion
- `RulesError::InsufficientStrategicResource` if stockpile is depleted
- Strategic resource yields from improvements wired into `advance_turn` stockpile update

### 8.8 — Road Placement

**Status:** `BuiltinRoad` and `RoadDef` trait defined. Road cost override active in Dijkstra. No action exists to place a road.

**Needed:**
- `RulesEngine::place_road()` callable by builder units
- Road upgrade path: Ancient → Medieval → Industrial → Railroad (tech-gated)
- Road maintenance gold deduction in `advance_turn`

### 8.9 — Builder Charges

**Status:** Builder unit type exists. `place_improvement()` works but consumes no unit resource.

**Needed:**
- `charges: u8` field on `BasicUnit` (or `UnitTypeDef`)
- Decrement charges on improvement placement; destroy unit at 0
- Optionally: track charges in `StateDelta`

### 8.10 — Great People System

**Status:** `GreatPerson` struct and `GreatPersonAbility` trait defined. `GameState.great_people` exists. No point accumulation or recruitment mechanics exist.

**Needed:**
- `YieldType::GreatPersonPoints` accumulation per district (Campus → Scientist points, etc.)
- Great person pool with era-gated candidates
- Recruitment action consuming accumulated points
- Activated ability effects (Great General combat bonus, Great Scientist eureka, etc.)

### 8.11 — Era Score and Age System

**Status:** `AgeType` and `Era` structs defined. `GameState.current_era` and `Civilization.current_era` exist. No score tracking, dark age, golden age, or heroic age logic exists.

**Needed:**
- Historic moment triggers that award era score
- Threshold comparison to determine Normal/Golden/Dark age per civ per era
- Age modifiers applied to yields and combat (dark age penalties, golden age bonuses)

### 8.12 — Governor System

**Status:** `Governor`, `GovernorDef` trait, `GovernorPromotion` trait defined (7 built-in governors). No assignment, establishment timer, or promotion effects wired into gameplay.

**Needed:**
- `GameState.governors: Vec<Governor>` collection
- `RulesEngine::assign_governor()` setting `governor.assigned_city`
- Establishment timer decrement in `advance_turn` (turns_to_establish → 0)
- Governor modifiers applied in `compute_yields` when governor is established

### 8.13 — Victory Condition Evaluation

**Status:** `VictoryCondition` trait and `VictoryProgress` defined. No concrete implementations. `GameState` has no `game_over` field.

**Needed:**
- Concrete types: `DominationVictory`, `ScienceVictory`, `CultureVictory`, `DiplomaticVictory`, `ScoreVictory`
- `check_progress(&GameState) -> VictoryProgress` (signature needs `&GameState`, not just `CivId`)
- `GameState.victory_conditions: Vec<Box<dyn VictoryCondition>>` and `game_over: bool`
- Evaluation pass at end of each turn in `advance_turn`

### 8.14 — Natural Wonder Discovery Events

**Status:** `BuiltinNaturalWonder` variants and appeal bonuses defined. Wonders placeable on tiles. No discovery trigger or event exists.

**Needed:**
- On first visibility of a natural wonder tile, emit a discovery event
- Apply wonder yield bonuses to worked tiles
- Appeal radius effect on adjacent tiles

### 8.15 — TurnEngine Consolidation

**Status:** `TurnEngine::process_turn()` calls `advance_turn()` but discards its diff, returning an empty `GameStateDiff` to callers. AI agent diffs are not composed into the turn diff.

**Needed:**
- `process_turn()` should aggregate and return all diffs from the turn
- AI agent decisions integrated into the turn flow
- Production completion moved out of `civsim` into `TurnEngine` or `advance_turn`

---

## Conventions

- VCS: jj (Jujutsu). Commit style: conventional commits (`infra:`, `impl:`, `fix:`, `tests:`, `docs:`, `plan:`)
- Movement costs scaled by 100 (integer math): `ONE=100`, `TWO=200`, `THREE=300`
- All built-in content names are `&'static str`; no heap allocation for compile-time strings
- `Box<dyn Trait>` fields prevent `Clone`; document this on affected structs
- Edition 2024; workspace resolver 2
- Plain enums preferred over trait objects for built-in variants (no dynamic dispatch overhead)
