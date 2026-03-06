# Open Civ VI — Architecture Specification

## Design Philosophy

1. **Invalid states unrepresentable** — encode constraints in types, not runtime checks
2. **Extensibility via traits** — game objects implement traits; mods add new impls
3. **Separated concerns** — geometry ≠ world state ≠ rules ≠ civilization state
4. **Single GameState** — eliminates threading HexBoard through every function
5. **Typed modifier pipeline** — no opaque callbacks; every yield change is a `Modifier`

Rust-style notation throughout. `trait` = interface, `struct` = concrete data, `enum` = sum type.

---

## Library Structure

```
libhexgrid  — coordinate geometry, pathfinding, board abstraction
libworld    — terrain, features, resources, tile improvements  (implements libhexgrid traits)
librules    — yields, modifiers, tech/civics, victory conditions
libcivcore  — civilizations, cities, units, diplomacy
libgame     — GameState, turn engine, semantic diff, CLI
civsim      — CLI binary
```

Workspace layout mirrors serde: each library is an independent crate with its own `Cargo.toml`.
`libworld` depends on `libhexgrid`; `librules` and `libcivcore` depend on both; `libgame` depends
on all four.

---

## 1. libhexgrid — Spatial Engine

### Coordinates

Cube coordinates: `HexCoord { q: i32, r: i32, s: i32 }` with the invariant `q + r + s = 0`
enforced at construction (via `HexCoord::new` returning `Result` or panicking in debug).
`HexCoord` implements `Add`, `Sub`, `Neg`, and scalar `Mul` so arithmetic composes naturally.

Distance: `max(|q₁−q₂|, |r₁−r₂|, |s₁−s₂|)`.

### Board Topology

```rust
enum BoardTopology {
    Hexagon   { radius: u32 },
    Rectangle { width: u32, height: u32, wrap_x: bool, wrap_y: bool },
}
```

Wrapping is implemented entirely inside `WorldBoard` via coordinate normalization — invisible to
all other layers. New topologies are a future concern; this is the only non-extensible enum in
the geometry layer and is intentionally isolated.

### Traits

```rust
trait HexTile {
    fn base_movement_cost(&self) -> MovementCost;
    fn elevation(&self) -> Elevation;
}

trait HexEdge {
    fn movement_cost(&self) -> MovementCost;
}

trait HexBoard<Tile: HexTile, Edge: HexEdge> {
    fn topology(&self) -> &BoardTopology;
    fn get_tile(&self, coord: HexCoord) -> Option<&Tile>;
    fn get_tile_mut(&mut self, coord: HexCoord) -> Option<&mut Tile>;
    fn get_edge(&self, coord: HexCoord, dir: HexDir) -> Option<&Edge>;
    fn neighbors(&self, coord: HexCoord) -> Vec<HexCoord>;
    fn distance(&self, a: HexCoord, b: HexCoord) -> u32;
    fn tiles_within_radius(&self, center: HexCoord, r: u32) -> Vec<HexCoord>;
    fn shortest_path(&self, a: HexCoord, b: HexCoord, profile: &MovementProfile)
        -> Option<Vec<HexCoord>>;
    fn line_of_sight(&self, a: HexCoord, b: HexCoord) -> bool;
    fn visible_tiles(&self, origin: HexCoord, vision: Vision) -> Vec<HexCoord>;
}
```

`libhexgrid` has zero knowledge of terrain, rivers, or civilizations. Coupling is prevented
because `libworld` provides the concrete `Tile`/`Edge` types — `libhexgrid` only knows the
traits. Pathfinding is implemented with simple Dijkstra; performance improvements are deferred.

### Supporting Types

```rust
enum HexDir       { NE, E, SE, SW, W, NW }
enum MovementCost { Impassable, Cost(u8) }

/// Low and High are discrete sentinels for LOS and flood logic.
/// Level(u8) encodes intermediate elevation for sea-level-rise simulation;
/// higher u8 = higher ground.
enum Elevation    { Low, Level(u8), High }

enum Vision       { Blind, Radius(u8), Omniscient }

struct MovementProfile {
    ignore_terrain: bool,  // air units bypass terrain cost
    ignore_rivers:  bool,  // embarked/naval units unaffected by river crossings
    can_embark:     bool,  // land unit may cross coast/ocean tiles
}
```

`Elevation::High` (and `Level` values above a flood threshold) blocks line-of-sight unless
the observer is also at an equal or greater level, or is `Omniscient`.

---

## 2. libworld — Map State

libworld defines `WorldTile` and `WorldEdge`, which implement the libhexgrid traits.
It has no knowledge of civilizations or rules.

### Terrain, Features, and Edge Features — Extensible Traits

All three are **traits**, not enums, so mods can define new terrain types, features, and edge
features. Built-in variants are structs implementing the trait.

```rust
trait TerrainDef: Send + Sync {
    fn id(&self) -> TerrainId;
    fn base_movement_cost(&self) -> MovementCost;
    fn base_yields(&self) -> YieldBundle;
    fn is_water(&self) -> bool;
    fn can_have_feature(&self, feature: &dyn FeatureDef) -> bool;
}

// Built-in: Grassland, Plains, Desert, Tundra, Snow, Coast, Ocean (all impl TerrainDef)

trait FeatureDef: Send + Sync {
    fn id(&self) -> FeatureId;
    fn movement_cost_override(&self) -> Option<MovementCost>;
    fn yield_delta(&self) -> YieldBundle;
    fn appeal_delta(&self) -> i32;
}

// Built-in: Forest, Rainforest, Marsh, Floodplain, Reef, Ice, VolcanicSoil, Oasis

trait EdgeFeatureDef: Send + Sync {
    fn id(&self) -> EdgeFeatureId;
    /// Extra MP cost to cross this edge (land units unless overridden by MovementProfile)
    fn crossing_cost(&self) -> MovementCost;
}

// Built-in: River, Cliff, Canal, MountainPass
```

### WorldTile

```rust
struct WorldTile {
    terrain:        Box<dyn TerrainDef>,
    features:       Vec<Box<dyn FeatureDef>>,
    natural_wonder: Option<Box<dyn NaturalWonder>>,
    resource:       Option<Box<dyn Resource>>,
    improvement:    Option<Box<dyn TileImprovement>>,
    road:           Option<Box<dyn RoadDef>>,
    appeal:         i32,  // recomputed by rules engine each query
}

impl HexTile for WorldTile { ... }
```

### WorldEdge

`WorldEdge` implements `HexEdge` from libhexgrid, keeping libhexgrid ignorant of rivers.
River movement penalty is applied by the rules engine during pathfinding via `MovementProfile`.

```rust
struct WorldEdge { feature: Option<Box<dyn EdgeFeatureDef>> }

impl HexEdge for WorldEdge {
    fn movement_cost(&self) -> MovementCost {
        self.feature.as_ref()
            .map(|f| f.crossing_cost())
            .unwrap_or(MovementCost::Cost(0))
    }
}
```

### Natural Wonders

```rust
trait NaturalWonder: Send + Sync {
    fn id(&self) -> &'static str;
    fn yields(&self) -> YieldBundle;
    fn modifiers(&self) -> Vec<Modifier>;
}
```

### Resources

```rust
trait Resource: Send + Sync {
    fn id(&self) -> ResourceId;
    fn category(&self) -> ResourceCategory;
    fn base_yields(&self) -> YieldBundle;
}

enum ResourceCategory { Strategic, Luxury, Bonus, Artifact, Power }
```

### Tile Improvements

Improvements are a trait so mods can define new ones without touching core code.

```rust
struct ImprovementContext<'a> {
    tile:           &'a WorldTile,
    adjacent_tiles: &'a [&'a WorldTile],
    civ:            &'a Civilization,
}

trait TileImprovement: Send + Sync {
    fn id(&self) -> ImprovementId;
    fn required_tech(&self) -> Option<TechId>;
    fn builder_charges(&self) -> u8;
    fn tile_yields(&self, ctx: &ImprovementContext) -> YieldBundle;
    /// Modifiers applied to the city that owns this tile
    fn city_modifiers(&self, ctx: &ImprovementContext) -> Vec<Modifier>;
}
```

Built-in implementations: `Farm`, `Mine`, `LumberMill`, `TradingPost`, `Fort`, `Airstrip`,
`MissileSilo`. All implement `TileImprovement`; none are special-cased.

### Roads

Roads are a trait, not an enum, so mods can define new road types (e.g. canal network, maglev).

```rust
trait RoadDef: Send + Sync {
    fn id(&self) -> RoadId;
    fn movement_cost_override(&self) -> MovementCost;
    fn required_tech(&self) -> Option<TechId>;
    /// True if this road type is automatically created by trade routes
    fn created_by_trade(&self) -> bool;
}

// Built-in: AncientRoad, MedievalRoad, IndustrialRoad, Railroad (all impl RoadDef)
```

---

## 3. librules — Rule Engine

### Yields

The core yield types are a fixed `enum` for now; extending yields to a trait is architecturally
possible but deferred — new yield types would require the mod author to also define rules for
how they are accumulated, spent, and displayed, which is a significant authoring burden.

```rust
enum YieldType { Food, Production, Gold, Science, Culture, Faith, Tourism, Power }

/// Sparse bundle — unset keys default to zero
struct YieldBundle(HashMap<YieldType, i32>);

impl YieldBundle {
    fn add(&mut self, y: YieldType, amount: i32);
    fn get(&self, y: YieldType) -> i32;
    fn merge_additive(&self, other: &Self) -> Self;
}
```

When trait-based yields become necessary, `YieldType` can be replaced with `Box<dyn YieldKind>`
and `YieldBundle` becomes `HashMap<TypeId, i32>` keyed on the concrete type's `TypeId`.

### Modifier System

Every mechanic that changes a numeric value (policy, building, leader ability, wonder, belief,
governor, technology, civic) is a `Modifier`. Modifiers are collected by target and applied at
query time — never mutating stored state directly.

```rust
struct Modifier {
    target:   TargetSelector,
    effect:   EffectType,
    stacking: StackingRule,
    source:   ModifierSource,  // for UI attribution and debugging
}

enum TargetSelector {
    City(CityId),
    AllCities(CivId),
    District { city: CityId, dtype: DistrictTypeId },
    AllDistricts { civ: CivId, dtype: DistrictTypeId },
    Unit(UnitId),
    AllUnits { civ: CivId, domain: Option<UnitDomain> },
    Tile(HexCoord),
    TradeRoute(TradeRouteId),
    Civilization(CivId),
    Global,
}

enum EffectType {
    YieldFlat(YieldType, i32),
    YieldPercent(YieldType, i32),  // applied after flat bonuses
    MovementBonus(i32),
    CombatBonus(i32),
    RangedCombatBonus(i32),
    LoyaltyPerTurn(i32),
    HousingBonus(f32),
    AmenitiesBonus(i32),
    GreatPersonPoints(GreatPersonType, i32),
    SightBonus(i32),
    TradeRouteCapacity(i32),
    AppealBonus(i32),
}

enum StackingRule { Additive, Multiplicative, Max, Replace }

enum ModifierSource {
    Policy(PolicyId),
    Building(BuildingId),
    Wonder(WonderId),
    LeaderAbility,
    Pantheon,
    Belief,
    Governor(GovernorId),
    GreatPerson(GreatPersonId),
    CityState(CivId),
    Technology(TechId),   // e.g. tech that boosts farm yields
    Civic(CivicId),       // e.g. civic that boosts trade yields
}
```

### Query System

The rules engine exposes typed queries. Callers never manually collect modifiers.

```rust
trait RulesEngine: Send + Sync {
    fn query_city_yield        (&self, city: CityId,            state: &GameState) -> YieldBundle;
    fn query_district_adjacency(&self, district: &PlacedDistrict, state: &GameState) -> YieldBundle;
    fn query_trade_yields      (&self, route: TradeRouteId,     state: &GameState) -> YieldBundle;
    fn query_unit_combat       (&self, unit: UnitId,            state: &GameState) -> i32;
    fn query_unit_movement     (&self, unit: UnitId,            state: &GameState) -> u8;
    fn query_tile_appeal       (&self, coord: HexCoord,         state: &GameState) -> i32;
    fn query_loyalty           (&self, city: CityId,            state: &GameState) -> f32;
    fn query_housing           (&self, city: CityId,            state: &GameState) -> f32;
    fn query_amenities         (&self, city: CityId,            state: &GameState) -> i32;

    /// Low-level — prefer typed queries above
    fn collect_modifiers<'a>(&'a self, target: &TargetSelector, state: &'a GameState)
        -> Vec<&'a Modifier>;
}
```

### Technology and Civics

Two independent research trees with the same internal structure.

```rust
struct TechTree  { nodes: HashMap<TechId,  TechNode>  }
struct CivicTree { nodes: HashMap<CivicId, CivicNode> }

struct TechNode {
    prerequisites: Vec<TechId>,
    cost:          u32,
    eureka:        Option<Box<dyn EurekaCondition>>,
    unlocks:       Vec<Unlock>,
    modifiers:     Vec<Modifier>,   // active once researched (e.g. +1 prod to farms)
}

// CivicNode is identical, with CivicId and an inspiration condition instead of eureka.

enum Unlock {
    Unit(UnitTypeId),
    District(DistrictTypeId),
    Building(BuildingId),
    Improvement(ImprovementId),
    Policy(PolicyId),
    Wonder(WonderId),
    Government(GovernmentId),
}

/// Predicate evaluated against GameState each turn until satisfied.
trait EurekaCondition: Send + Sync {
    fn is_met(&self, civ: CivId, state: &GameState) -> bool;
}
```

### Governments and Policies

```rust
struct Government {
    id:                 GovernmentId,
    required_civic:     CivicId,
    policy_slots:       PolicySlots,
    inherent_modifiers: Vec<Modifier>,
}

struct PolicySlots { military: u8, economic: u8, diplomatic: u8, wildcard: u8 }

enum PolicyType { Military, Economic, Diplomatic, Wildcard, DarkAge, HeroicAge }

struct Policy {
    id:             PolicyId,
    required_civic: CivicId,
    policy_type:    PolicyType,
    modifiers:      Vec<Modifier>,
}
```

### Victory Conditions

```rust
trait VictoryCondition: Send + Sync {
    fn id(&self) -> VictoryId;
    fn check   (&self, civ: CivId, state: &GameState) -> bool;
    fn progress(&self, civ: CivId, state: &GameState) -> VictoryProgress;
}

struct VictoryProgress { current: u32, required: u32, label: String }
```

Built-in implementations: `ScienceVictory`, `CultureVictory`, `DominationVictory`,
`ReligiousVictory`, `DiplomaticVictory`, `ScoreVictory`.

---

## 4. libcivcore — Civilization State

### Eras

Eras are user-defined structs, not a static enum, so mods can define new eras or alter
transition triggers. The global ordered list of `Era`s is stored in `GameState`.

```rust
struct Era {
    id:      EraId,
    name:    String,
    trigger: Box<dyn EraTrigger>,
}

/// Returns true when this era should become the active era for a given civ.
/// Default implementation: triggered when the civ researches the last tech
/// of the previous era (standard Civ VI behavior).
trait EraTrigger: Send + Sync {
    fn is_triggered(&self, civ: CivId, state: &GameState) -> bool;
}
```

### Units

Air units are **not** a separate struct. All units implement `Unit`. `UnitDomain::Air` units
use `base_tile` + `range` instead of standard pathfinding; movement validation is
`distance(base_tile, target) <= range`. This keeps the unit system extensible.

```rust
trait Unit: Send + Sync {
    fn id(&self)               -> UnitId;
    fn unit_type(&self)        -> UnitTypeId;
    fn owner(&self)            -> CivId;
    fn position(&self)         -> HexCoord;
    fn domain(&self)           -> UnitDomain;
    fn movement_profile(&self) -> MovementProfile;
    fn movement_points(&self)  -> u8;
    fn remaining_movement(&self) -> u8;
    fn combat_strength(&self)  -> Option<i32>;       // None = civilian / non-combat
    fn ranged_strength(&self)  -> Option<i32>;
    fn range(&self)            -> Option<u8>;         // ranged attack or air operational radius
    fn base_tile(&self)        -> Option<HexCoord>;   // Some for air units (home city/airstrip)
    fn health(&self)           -> u8;                 // 0–100
    fn promotions(&self)       -> &[PromotionId];
    fn category(&self)         -> UnitCategory;
}

enum UnitDomain   { Land, Naval, Air }
enum UnitCategory { Combat, Civilian, Support, GreatPerson, Religious }
```

### Cities

Yields, housing, amenities, and loyalty are **not stored** on `City` — they are computed by
`RulesEngine` queries so modifiers are always applied correctly. The `loyalty` field is stored
only for persistence between turns (it accumulates/decays, it is not purely derived).

```rust
struct City {
    id:                     CityId,
    owner:                  CivId,       // changes on loyalty flip or capture
    location:               HexCoord,
    name:                   String,
    population:             u32,
    status:                 CityStatus,
    worked_tiles:           Vec<HexCoord>,
    districts:              Vec<PlacedDistrict>,
    production_queue:       Vec<ProductionItem>,
    accumulated_food:       i32,
    accumulated_production: i32,
    governor:               Option<GovernorId>,
    walls:                  WallLevel,
    religion:               Option<ReligionId>,  // majority religion
    loyalty:                f32,  // 0–100; stored, recomputed via pressure each turn
}

enum CityStatus { Normal, Occupied, Puppet, Razed }
enum WallLevel  { None, Ancient, Medieval, Renaissance }
```

`owner` changes when loyalty reaches 0 (flip, status stays `Normal`) or on capture in war
(status becomes `Occupied`).

### Districts

Districts are extensible via `DistrictDef`. `PlacedDistrict` holds a reference to its
definition via trait object.

```rust
struct PlacedDistrict {
    location:  HexCoord,
    def:       Box<dyn DistrictDef>,
    buildings: Vec<Box<dyn BuildingDef>>,
    damage:    u8,  // 0–100; damaged districts produce no yields
}

struct AdjacencyContext<'a> {
    location:           HexCoord,
    adjacent_tiles:     &'a [&'a WorldTile],
    adjacent_districts: &'a [&'a PlacedDistrict],
    river_adjacent:     bool,
}

trait DistrictDef: Send + Sync {
    fn id(&self)                          -> DistrictTypeId;
    fn required_tech(&self)               -> Option<TechId>;
    fn required_civic(&self)              -> Option<CivicId>;
    fn production_cost(&self, pop: u32)   -> u32;  // scales with city size
    fn adjacency_bonus(&self, ctx: &AdjacencyContext) -> YieldBundle;
    fn modifiers(&self)                   -> Vec<Modifier>;
    fn great_person_type(&self)           -> Option<GreatPersonType>;
    fn max_buildings(&self)               -> u8;
}

trait BuildingDef: Send + Sync {
    fn id(&self)                  -> BuildingId;
    fn required_tech(&self)       -> Option<TechId>;
    fn required_district(&self)   -> DistrictTypeId;
    fn production_cost(&self)     -> u32;
    fn maintenance_gold(&self)    -> i32;
    fn yields(&self)              -> YieldBundle;
    fn modifiers(&self)           -> Vec<Modifier>;
    fn housing(&self)             -> f32;
    fn amenities(&self)           -> i32;
    fn great_person_points(&self) -> Vec<(GreatPersonType, i32)>;
}
```

Built-in districts: `Campus`, `CommercialHub`, `Harbor`, `Encampment`, `TheaterSquare`,
`IndustrialZone`, `HolySite`, `Aqueduct`, `Entertainment`, `Spaceport`.

### Governors

Governors are extensible: mods define new governor types by implementing `GovernorDef`.
The `GovernorTitle` enum is removed; governors are identified by `GovernorId` (a newtype
over a string or integer key).

```rust
trait GovernorDef: Send + Sync {
    fn id(&self)   -> GovernorId;
    fn name(&self) -> &str;
    fn available_promotions(&self) -> &[Box<dyn GovernorPromotion>];
}

trait GovernorPromotion: Send + Sync {
    fn id(&self) -> PromotionId;
    fn modifiers(&self, city: &City, state: &GameState) -> Vec<Modifier>;
}

struct Governor {
    def:                Arc<dyn GovernorDef>,
    active_promotions:  Vec<Box<dyn GovernorPromotion>>,
    assigned_city:      Option<CityId>,
    turns_to_establish: u8,  // > 0 = en route; promotions inactive until 0
}
```

Built-in governors (`Magnus`, `Amani`, `Victor`, `Reyna`, `Liang`, `Pingala`, `Moksha`)
each implement `GovernorDef`.

### Leaders and Civilizations

```rust
struct Civilization {
    id:              CivId,
    name:            String,
    leader:          Leader,
    cities:          Vec<CityId>,
    units:           Vec<UnitId>,
    trade_routes:    Vec<TradeRouteId>,  // index into GameState; avoids duplication
    tech_progress:   TechProgress,
    civic_progress:  CivicProgress,
    government:      GovernmentId,
    active_policies: Vec<PolicyId>,
    treasury:        i32,
    envoys:          u32,
    governors:       Vec<Governor>,
    diplomacy:       HashMap<CivId, DiplomaticRelation>,
    religion:        Option<ReligionId>,
    great_person_points: HashMap<GreatPersonType, i32>,
}

struct Leader {
    name:       String,
    abilities:  Vec<Box<dyn LeaderAbility>>,
    agenda:     Box<dyn Agenda>,
    start_bias: StartBias,
}

trait LeaderAbility: Send + Sync {
    fn modifiers(&self, state: &GameState) -> Vec<Modifier>;
}

/// Encodes what this leader's AI values — drives opinion scores and grievances.
trait Agenda: Send + Sync {
    fn evaluate(&self, other: CivId, state: &GameState) -> i32;
    fn grievance_triggers(&self) -> &[Box<dyn GrievanceTrigger>];
}

struct StartBias {
    terrain:             Vec<TerrainId>,
    features:            Vec<FeatureId>,
    resource_categories: Vec<ResourceCategory>,
    coastal:             bool,
    priority:            u8,  // 1–5; higher = stronger preference in start picker
}
```

### Diplomacy

Both `Agreement` and `GrievanceTrigger` are traits so mods can add new deal types and AI
triggers without modifying core enums.

```rust
struct DiplomaticRelation {
    status:            DiplomaticStatus,
    opinion_score:     i32,
    turns_at_war:      Option<u32>,
    active_agreements: Vec<Box<dyn Agreement>>,
}

enum DiplomaticStatus { War, Neutral, Friendly, Denounced, Alliance }

trait Agreement: Send + Sync {
    fn id(&self) -> AgreementId;
    fn duration_turns(&self) -> Option<u32>;  // None = permanent until cancelled
    fn on_active_modifiers(&self, self_civ: CivId, other_civ: CivId) -> Vec<Modifier>;
    fn on_break_opinion_penalty(&self) -> i32;
}

// Built-in: OpenBorders, ResearchAgreement, EmergencyAlliance, TradeAgreement

trait GrievanceTrigger: Send + Sync {
    fn id(&self) -> GrievanceId;
    fn opinion_penalty(&self) -> i32;
    fn check(&self, actor: CivId, victim: CivId, state: &GameState) -> bool;
}

// Built-in: SettledTooClose, ConvertedCities, AttackedCityState,
//           BrokenAgreement, DeclaredWarOnAlly
```

### City States

```rust
struct CityState {
    id:            CivId,
    state_type:    CityStateType,
    location:      HexCoord,
    suzerain:      Option<CivId>,
    envoy_counts:  HashMap<CivId, u32>,
    bonus:         Box<dyn CityStateBonus>,
}

enum CityStateType { Military, Cultural, Industrial, Economic, Religious, Scientific, Trade }

trait CityStateBonus: Send + Sync {
    fn suzerain_modifiers(&self, civ: &Civilization) -> Vec<Modifier>;
    fn envoy_thresholds(&self) -> &[(u32, Vec<Modifier>)];
}
```

### Religion

```rust
struct Religion {
    id:               ReligionId,
    founder:          CivId,
    pantheon:         Box<dyn Belief>,
    beliefs:          Vec<Box<dyn Belief>>,
    holy_city:        CityId,
    cities_following: u32,
}

struct BeliefContext<'a> {
    civ:  &'a Civilization,
    city: Option<&'a City>,
    unit: Option<&'a dyn Unit>,
}

trait Belief: Send + Sync {
    fn id(&self) -> BeliefId;
    fn modifiers(&self, ctx: &BeliefContext) -> Vec<Modifier>;
}
```

### Trade Routes

> **Design note:** `TradeRoute`s and `Unit`s are indexed globally in `GameState`
> (not exclusively per-civilization) to simplify queries that cross civ boundaries —
> e.g. pathfinding around foreign units, combat resolution, trade yield calculation from
> a destination city belonging to another civ. Each `Civilization` holds `Vec<UnitId>`
> and `Vec<TradeRouteId>` as lightweight index references into the global maps, so both
> per-civ and global lookups remain O(1).

```rust
struct TradeRoute {
    id:              TradeRouteId,
    owner:           CivId,
    origin:          CityId,
    destination:     CityId,
    path:            Vec<HexCoord>,  // used for road creation and trading-post checks
    remaining_turns: u32,
    trader_unit:     UnitId,
}
```

Yields computed each turn via `RulesEngine::query_trade_yields()`.

### Great People

```rust
struct GreatPerson {
    id:          GreatPersonId,
    person_type: GreatPersonType,
    era:         EraId,
    modifiers:   Vec<Modifier>,                     // passive if retired to a tile
    abilities:   Vec<Box<dyn GreatPersonAbility>>,
}

enum GreatPersonType {
    General, Admiral, Scientist, Engineer, Merchant,
    Prophet, Artist, Musician, Writer,
}

trait GreatPersonAbility: Send + Sync {
    fn label(&self) -> &str;
    fn activate(&self, person: &GreatPerson, state: &mut GameState);
}
```

---

## 5. libgame — Game Orchestration

### GameState

Single top-level struct passed (by reference) to all systems.

```rust
struct GameState {
    board:              WorldBoard,    // impl HexBoard<WorldTile, WorldEdge>
    civilizations:      HashMap<CivId,        Civilization>,
    city_states:        HashMap<CivId,        CityState>,
    cities:             HashMap<CityId,       City>,
    units:              HashMap<UnitId,       Box<dyn Unit>>,
    trade_routes:       HashMap<TradeRouteId, TradeRoute>,
    great_people:       Vec<GreatPerson>,
    religions:          HashMap<ReligionId,   Religion>,
    tech_tree:          TechTree,
    civic_tree:         CivicTree,
    governments:        HashMap<GovernmentId, Government>,
    policies:           HashMap<PolicyId,     Policy>,
    eras:               Vec<Era>,              // ordered; user-defined
    victory_conditions: Vec<Box<dyn VictoryCondition>>,
    rules:              Box<dyn RulesEngine>,
    turn:               u32,
    rng_seed:           u64,
}
```

### Semantic Diffs

To support replay, undo, AI planning, and Lua event hooks, the game engine produces a
`GameStateDiff` after each logical operation. This is a structured record of what changed,
not a raw binary delta.

```rust
/// A single atomic change to GameState
enum StateDelta {
    CityOwnerChanged  { city: CityId, from: CivId, to: CivId },
    UnitMoved         { unit: UnitId, from: HexCoord, to: HexCoord },
    UnitDestroyed     { unit: UnitId },
    PopulationChanged { city: CityId, delta: i32 },
    TileImproved      { coord: HexCoord, improvement: ImprovementId },
    ResearchCompleted { civ: CivId, tech: TechId },
    CivicCompleted    { civ: CivId, civic: CivicId },
    DiplomacyChanged  { a: CivId, b: CivId, status: DiplomaticStatus },
    ReligionFounded   { religion: ReligionId, founder: CivId },
    VictoryAchieved   { civ: CivId, victory: VictoryId },
    // ... extensible
}

/// Ordered list of deltas produced by one turn or one action.
struct GameStateDiff { deltas: Vec<StateDelta> }
```

All turn-processing functions return `GameStateDiff`. The Lua scripting layer receives diffs
as events, enabling reactive mod logic.

### Era and Age

```rust
enum AgeType { Normal, DarkAge, HeroicAge, GoldenAge }
```

The active `Era` per civilization is determined by evaluating `Era::trigger` each turn.

### Turn Structure

```
1. Start of turn
   - Apply loyalty pressure from adjacent cities/governors
   - Resolve city flips (loyalty ≤ 0)
   - Advance disasters (floods, volcanic eruptions)

2. Per-civilization player actions
   - Move units / conduct combat
   - City production choices
   - Research advance
   - Policy / government changes
   - Diplomacy and envoy placement
   - Trade route assignment

3. End of turn
   - Collect yields (food, gold, science, culture, faith)
   - City food consumption and population growth/decline
   - Production queue advance
   - Great person point accumulation
   - Eureka/inspiration checks
   - Victory condition evaluation
```

### Production Items

```rust
enum ProductionItem {
    Unit(UnitTypeId),
    District(DistrictTypeId),
    Building(BuildingId),
    Wonder(WonderId),
    Project(ProjectId),
}

trait Project: Send + Sync {
    fn id(&self) -> ProjectId;
    fn production_cost(&self) -> u32;
    fn on_complete(&self, city: CityId, state: &mut GameState) -> GameStateDiff;
}
```

### Lua Scripting Interface

A typed Lua API is developed alongside gameplay. The engine exposes:

- **Events** — Lua callbacks fired when `StateDelta`s occur (e.g. `on_city_captured`)
- **Queries** — Lua can call `query_city_yield`, `query_unit_combat`, etc.
- **Commands** — Lua can issue validated actions (move unit, build improvement, etc.)
- **Definitions** — Lua can register new `TerrainDef`, `FeatureDef`, `TileImprovement`,
  `DistrictDef`, `GovernorDef`, `Agreement`, etc. via the trait system

The Lua API surface is documented in a separate `LUA_API.md` and kept in sync with the
trait interfaces. Type safety at the Rust/Lua boundary is enforced via a thin binding layer.

### CLI Interface

```
civsim generate-map --topology rectangle --width 80 --height 52 --wrap-x
civsim spawn-civ --leader "Victoria" --start q,r,s
civsim spawn-unit --civ 0 --type Warrior --at q,r,s
civsim run-turn
civsim query-yields --city "London"
civsim query-combat --unit 7
civsim query-loyalty --city "Paris"
civsim print-diplomacy
```

---

## 6. Implementation Phases

Development uses **test-driven development**: tests are written first and initially fail,
tracking progress toward a concrete implementation. Commits are atomic and tagged:
`infra`, `impl`, `tests`, `fix`, `docs`.

The Lua scripting API is developed alongside gameplay (Phase 3) — not bolted on at the end.

Algorithms (Dijkstra, LOS) are implemented simply first; performance is not a priority
until correctness is confirmed.

### Phase 1 — Scaffolding

- `libhexgrid`: `HexCoord` (with invariant + arithmetic), `HexBoard` trait, `HexDir`,
  `MovementCost`, `Elevation`, `Vision`, `MovementProfile`
- `libworld`: `WorldTile`, `WorldEdge`, `TerrainDef`/`FeatureDef`/`EdgeFeatureDef` traits,
  `Resource`/`TileImprovement`/`RoadDef` traits (stubs)
- `librules`: `YieldBundle`, `Modifier`, `EffectType`, `TargetSelector`,
  `RulesEngine` trait (stub)
- `libcivcore`: all structs and traits declared with stub impls; `Era`, `Unit`, `City`,
  `Civilization`, `Governor`, `DiplomaticRelation`, etc.
- `libgame`: `GameState`, `GameStateDiff`, `StateDelta`, all ID newtypes

**Goal:** project compiles with zero warnings. Failing test suite written covering all
Phase 2 correctness criteria.

### Phase 2 — Geometry and Core Rules

Failing tests from Phase 1 pass:

- Hex distance and neighbor correctness (all 6 directions, wraparound)
- Toroidal coordinate normalization (rectangle with `wrap_x`)
- Pathfinding (Dijkstra) with terrain costs and edge crossing costs
- Line-of-sight through `Elevation::High` tiles
- Yield stacking: flat then percent, multiple sources, `StackingRule` variants
- District adjacency bonus calculation
- Modifier collection and `StackingRule` application
- Determinism: same `rng_seed` → identical map generation

### Phase 3 — Gameplay (in order, with Lua API)

1. Tile yield calculation and worked tile assignment
2. City food accumulation and population growth
3. City production queue and build completion
4. Unit movement validation and combat resolution
5. District placement, adjacency bonuses, building construction
6. Tech and civic research with eureka triggers; tech `Modifier`s applied
7. Trade route creation, path resolution, yield generation
8. Policy cards and government switching
9. Diplomacy state machine, opinion modifiers, war declarations
10. Religion founding, missionary spread, belief modifiers
11. Great person point accumulation and recruitment
12. Governor assignment and promotion effects
13. Era transitions via `EraTrigger`
14. Victory condition evaluation
