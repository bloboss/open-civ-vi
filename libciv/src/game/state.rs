use std::collections::VecDeque;
use crate::{
    BarbarianCampId, BuildingId, CivId, CivicRefs, CityId, GrievanceId, ProjectId, TechRefs,
    UnitCategory, UnitDomain, UnitId, UnitTypeId, WonderId, EraId, VictoryId, YieldBundle,
};
use super::victory::BuiltinVictoryCondition;
use crate::civ::{
    BarbarianCamp, BarbarianConfig, BasicUnit, BeliefRefs, BuiltinBelief, Civilization, City,
    CityKind, DiplomaticRelation, GreatPerson, GreatPersonDef, Governor, PlacedDistrict,
    Religion, TradeRoute, WonderTourism, WorldCongress,
};
use crate::civ::era::Era;
use crate::civ::religion::build_beliefs;
use crate::rules::{TechTree, CivicTree, Government, Policy, OneShotEffect,
    register_builtin_governments, register_builtin_policies};
use crate::rules::tech::{build_tech_tree, build_civic_tree};
use crate::rules::building_defs::builtin_building_defs;
use crate::rules::unit_defs::builtin_unit_type_defs;
use crate::rules::project_defs::builtin_project_defs;
use crate::rules::promotion::{RegisteredPromotion, register_builtin_promotions};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use ulid::Ulid;

use super::board::WorldBoard;

/// Static descriptor for a unit type; stored in `GameState.unit_type_defs`.
/// `apply_effect(FreeUnit)` looks up by name to construct a `BasicUnit`.
/// `production_cost` is used by the production queue completion logic.
#[derive(Debug, Clone)]
pub struct UnitTypeDef {
    /// Canonical ID used to match `BasicUnit.unit_type` back to this def.
    pub id:              UnitTypeId,
    pub name:            &'static str,
    pub production_cost: u32,
    pub domain:          UnitDomain,
    pub category:        UnitCategory,
    pub max_movement:    u32,
    pub combat_strength: Option<u32>,
    /// Melee = 0; ranged attack range in tiles.
    pub range:           u8,
    /// Vision radius for spawned units of this type.
    pub vision_range:    u8,
    /// True for settler-class units: `found_city` may be called on them.
    pub can_found_city:  bool,
    /// Strategic resource consumed from the civilization's stockpile when this
    /// unit completes production. `None` means no resource cost.
    pub resource_cost:   Option<(crate::world::resource::BuiltinResource, u32)>,
    /// Extra combat strength added when this unit attacks a unit on a city tile.
    /// 0 for non-siege units.
    pub siege_bonus:     u32,
    /// Maximum build charges for builder-type units. 0 for non-builder units.
    /// When a unit is spawned, `BasicUnit.charges` is set to `Some(max_charges)`
    /// if `max_charges > 0`, or `None` otherwise.
    pub max_charges:     u8,
    /// If set, this unit is exclusive to the given civilization.
    pub exclusive_to:    Option<crate::civ::civ_identity::BuiltinCiv>,
    /// If set, this unit replaces the named base unit for its civilization.
    pub replaces:        Option<&'static str>,
    /// Era this unit belongs to (for production bonus conditions).
    pub era:             Option<crate::AgeType>,
    /// Promotion class for this unit type (determines which promotions are available).
    pub promotion_class: Option<crate::PromotionClass>,
}

/// Static descriptor for a wonder; stored in `GameState.wonder_defs`.
#[derive(Debug, Clone)]
pub struct WonderDef {
    pub id:              WonderId,
    pub name:            &'static str,
    pub production_cost: u32,
    /// Era this wonder belongs to (for production bonus conditions).
    pub era:             Option<crate::AgeType>,
}

/// Static descriptor for a building type; stored in `GameState.building_defs`.
/// `id` is the canonical `BuildingId` used when adding the building to a city.
#[derive(Debug, Clone)]
pub struct BuildingDef {
    pub id:                  BuildingId,
    pub name:                &'static str,
    pub cost:                u32,
    pub maintenance:         u32,
    pub yields:              YieldBundle,
    pub requires_district:   Option<&'static str>,
    /// Another building that must already exist in the city before this one
    /// can be produced (e.g. University requires Library).
    pub prereq_building:     Option<&'static str>,
    /// A building that cannot coexist with this one in the same city
    /// (e.g. Barracks and Stable are mutually exclusive).
    pub mutually_exclusive:  Option<&'static str>,
    /// Great work slots provided when this building is constructed in a city.
    pub great_work_slots:    Vec<crate::civ::great_works::GreatWorkSlotType>,
    /// If set, this building is exclusive to the given civilization.
    pub exclusive_to:        Option<crate::civ::civ_identity::BuiltinCiv>,
    /// If set, this building replaces the named base building for its civilization.
    pub replaces:            Option<&'static str>,
    /// Power consumed per turn (0 for most buildings; late-era buildings need power).
    pub power_cost:          u32,
    /// Power generated per turn (only power plant buildings produce power).
    pub power_generated:     u32,
    /// CO2 emitted per turn (only fossil fuel power plants).
    pub co2_per_turn:        u32,
}

/// Static descriptor for a city project; stored in `GameState.project_defs`.
/// Projects are production queue items that don't create a unit or building —
/// they have special completion effects (e.g. science milestones, CO2 removal).
#[derive(Debug, Clone)]
pub struct ProjectDef {
    pub id: ProjectId,
    pub name: &'static str,
    pub production_cost: u32,
    pub requires_district: Option<&'static str>,
    pub repeatable: bool,
}

/// Deterministic ID generator backed by a seeded RNG.
pub struct IdGenerator {
    rng: SmallRng,
    pub(crate) timestamp_ms: u64,
}

#[cfg(feature = "serde")]
impl serde::Serialize for IdGenerator {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("IdGenerator", 1)?;
        s.serialize_field("timestamp_ms", &self.timestamp_ms)?;
        s.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for IdGenerator {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Helper { timestamp_ms: u64 }
        let h = Helper::deserialize(deserializer)?;
        // The RNG is rebuilt from seed 0; the caller must reseed from GameState.seed.
        let mut id_gen = IdGenerator::new(0);
        id_gen.timestamp_ms = h.timestamp_ms;
        Ok(id_gen)
    }
}

impl std::fmt::Debug for IdGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdGenerator")
            .field("timestamp_ms", &self.timestamp_ms)
            .finish_non_exhaustive()
    }
}

impl IdGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            timestamp_ms: 0,
        }
    }

    pub fn next_ulid(&mut self) -> Ulid {
        use rand::RngCore;
        let hi = self.rng.next_u64() as u128;
        let lo = self.rng.next_u64() as u128;
        let random = (hi << 64) | lo;
        self.timestamp_ms += 1;
        Ulid::from_parts(self.timestamp_ms, random)
    }

    pub fn next_city_id(&mut self) -> CityId {
        CityId::from_ulid(self.next_ulid())
    }

    pub fn next_unit_id(&mut self) -> UnitId {
        UnitId::from_ulid(self.next_ulid())
    }

    pub fn next_civ_id(&mut self) -> CivId {
        CivId::from_ulid(self.next_ulid())
    }

    pub fn next_building_id(&mut self) -> BuildingId {
        BuildingId::from_ulid(self.next_ulid())
    }

    pub fn next_grievance_id(&mut self) -> GrievanceId {
        GrievanceId::from_ulid(self.next_ulid())
    }

    pub fn next_trade_route_id(&mut self) -> crate::TradeRouteId {
        crate::TradeRouteId::from_ulid(self.next_ulid())
    }

    pub fn next_great_person_id(&mut self) -> crate::GreatPersonId {
        crate::GreatPersonId::from_ulid(self.next_ulid())
    }

    pub fn next_victory_id(&mut self) -> VictoryId {
        VictoryId::from_ulid(self.next_ulid())
    }

    pub fn next_great_work_id(&mut self) -> crate::GreatWorkId {
        crate::GreatWorkId::from_ulid(self.next_ulid())
    }

    pub fn next_barbarian_camp_id(&mut self) -> BarbarianCampId {
        BarbarianCampId::from_ulid(self.next_ulid())
    }

    pub fn next_promotion_id(&mut self) -> crate::PromotionId {
        crate::PromotionId::from_ulid(self.next_ulid())
    }

    pub fn next_project_id(&mut self) -> ProjectId {
        ProjectId::from_ulid(self.next_ulid())
    }

    /// Returns a pseudo-random f32 in [0.0, 1.0) drawn from the seeded RNG.
    /// Used for combat randomisation; does not affect the ULID sequence.
    pub fn next_f32(&mut self) -> f32 {
        use rand::RngCore;
        (self.rng.next_u32() as f32) / (u32::MAX as f32 + 1.0)
    }
}

/// The full game state.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GameState {
    pub turn: u32,
    pub seed: u64,
    pub board: WorldBoard,
    pub id_gen: IdGenerator,
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub civilizations: Vec<Civilization>,
    pub cities: Vec<City>,
    pub units: Vec<BasicUnit>,
    pub placed_districts: Vec<PlacedDistrict>,
    pub diplomatic_relations: Vec<DiplomaticRelation>,
    pub religions: Vec<Religion>,
    pub trade_routes: Vec<TradeRoute>,
    pub great_people: Vec<GreatPerson>,
    /// Registry of great person definitions. Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub great_person_defs: Vec<GreatPersonDef>,
    pub great_works: Vec<crate::civ::great_works::GreatWork>,
    /// Rebuilt from seed after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub tech_tree: TechTree,
    /// Rebuilt from seed after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub tech_refs: TechRefs,
    /// Rebuilt from seed after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub civic_tree: CivicTree,
    /// Rebuilt from seed after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub civic_refs: CivicRefs,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub belief_defs: Vec<BuiltinBelief>,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub belief_refs: BeliefRefs,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub governments: Vec<Government>,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub policies: Vec<Policy>,
    pub current_era: EraId,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub unit_type_defs: Vec<UnitTypeDef>,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub promotion_defs: Vec<RegisteredPromotion>,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub building_defs: Vec<BuildingDef>,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub wonder_defs: Vec<WonderDef>,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub project_defs: Vec<ProjectDef>,
    /// Governors owned by civilizations. Loyalty computation checks for
    /// established governors assigned to cities.
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub governors: Vec<Governor>,
    /// Rebuilt from builtins after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub eras: Vec<Era>,
    /// Index into `eras` for the current global era.
    pub current_era_index: usize,
    /// Active victory conditions evaluated each turn by `advance_turn`.
    /// Register before the game loop. Can be empty (no win condition).
    pub victory_conditions: Vec<BuiltinVictoryCondition>,
    /// Set when a civilization has won the game. `advance_turn` no longer
    /// evaluates victory conditions once this is `Some`.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub game_over: Option<super::victory::GameOver>,
    /// Built wonders that generate tourism per turn. Entries are added when a
    /// wonder completes production (or manually for testing).
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub wonder_tourism: Vec<WonderTourism>,
    /// Skipped during serialization; drained each turn.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub effect_queue: VecDeque<(CivId, OneShotEffect)>,
    /// Cumulative global CO2 emitted by fossil fuel power plants across all
    /// civilizations. Feeds the climate system (GS-2).
    #[cfg_attr(feature = "serde", serde(default))]
    pub global_co2: u32,
    /// Current sea level rise stage (0–7), driven by `global_co2` thresholds.
    #[cfg_attr(feature = "serde", serde(default))]
    pub climate_level: u8,
    /// Active barbarian camps on the map.
    pub barbarian_camps: Vec<BarbarianCamp>,
    /// Configuration for the barbarian system.
    pub barbarian_config: BarbarianConfig,
    /// The CivId representing the barbarian faction. All barbarian units and
    /// camps are owned by this civ. `None` when barbarians are disabled.
    pub barbarian_civ: Option<CivId>,
    /// World Congress state: periodic sessions, active resolutions, diplomatic
    /// victory points (GS-3).
    #[cfg_attr(feature = "serde", serde(default))]
    pub world_congress: WorldCongress,
}

impl GameState {
    pub fn new(seed: u64, width: u32, height: u32) -> Self {
        let board = WorldBoard::new(width, height);
        let mut id_gen = IdGenerator::new(seed);
        let era_id = EraId::from_ulid(id_gen.next_ulid());
        let (tech_tree, tech_refs)   = build_tech_tree(&mut id_gen);
        let (civic_tree, civic_refs) = build_civic_tree(&mut id_gen);
        let (belief_defs, belief_refs) = build_beliefs(&mut id_gen);
        let governments = register_builtin_governments(&mut id_gen);
        let policies = register_builtin_policies(&mut id_gen);
        let building_defs = builtin_building_defs(&mut id_gen);
        let unit_type_defs = builtin_unit_type_defs(&mut id_gen);
        let promotion_defs = register_builtin_promotions(&mut id_gen);
        let project_defs = builtin_project_defs(&mut id_gen);

        Self {
            turn: 0,
            seed,
            board,
            id_gen,
            civilizations: Vec::new(),
            cities: Vec::new(),
            units: Vec::new(),
            placed_districts: Vec::new(),
            diplomatic_relations: Vec::new(),
            religions: Vec::new(),
            trade_routes: Vec::new(),
            great_people: Vec::new(),
            great_person_defs: Vec::new(),
            great_works: Vec::new(),
            tech_tree,
            tech_refs,
            civic_tree,
            civic_refs,
            belief_defs,
            belief_refs,
            governments,
            policies,
            current_era: era_id,
            governors: Vec::new(),
            eras: Vec::new(),
            current_era_index: 0,
            unit_type_defs,
            promotion_defs,
            building_defs,
            wonder_defs: Vec::new(),
            project_defs,
            victory_conditions: Vec::new(),
            game_over: None,
            wonder_tourism: Vec::new(),
            effect_queue: VecDeque::new(),
            global_co2: 0,
            climate_level: 0,
            barbarian_camps: Vec::new(),
            barbarian_config: BarbarianConfig::default(),
            barbarian_civ: None,
            world_congress: WorldCongress::default(),
        }
    }

    pub fn civ(&self, id: CivId) -> Option<&Civilization> {
        self.civilizations.iter().find(|c| c.id == id)
    }

    pub fn city(&self, id: CityId) -> Option<&City> {
        self.cities.iter().find(|c| c.id == id)
    }

    pub fn unit(&self, id: UnitId) -> Option<&BasicUnit> {
        self.units.iter().find(|u| u.id == id)
    }

    pub fn unit_mut(&mut self, id: UnitId) -> Option<&mut BasicUnit> {
        self.units.iter_mut().find(|u| u.id == id)
    }

    /// Returns the city that represents the given city-state CivId, if one exists.
    /// City states are stored in the cities vec with owner == their diplomatic CivId.
    pub fn city_state_by_civ(&self, civ_id: CivId) -> Option<&City> {
        self.cities.iter().find(|c| {
            matches!(c.kind, CityKind::CityState(_)) && c.owner == civ_id
        })
    }

    pub fn barbarian_camp(&self, id: BarbarianCampId) -> Option<&BarbarianCamp> {
        self.barbarian_camps.iter().find(|c| c.id == id)
    }

    pub fn barbarian_camp_mut(&mut self, id: BarbarianCampId) -> Option<&mut BarbarianCamp> {
        self.barbarian_camps.iter_mut().find(|c| c.id == id)
    }

    pub fn great_person(&self, id: crate::GreatPersonId) -> Option<&GreatPerson> {
        self.great_people.iter().find(|gp| gp.id == id)
    }

    pub fn great_person_mut(&mut self, id: crate::GreatPersonId) -> Option<&mut GreatPerson> {
        self.great_people.iter_mut().find(|gp| gp.id == id)
    }
}
