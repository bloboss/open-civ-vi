use std::collections::VecDeque;
use crate::{
    BuildingId, CivId, CivicRefs, CityId, GrievanceId, TechRefs, UnitCategory, UnitDomain, UnitId, UnitTypeId, EraId, VictoryId, YieldBundle,
};
use super::victory::VictoryCondition;
use crate::civ::{
    BasicUnit, Civilization, City, CityKind, DiplomaticRelation, GreatPerson, GreatPersonDef,
    Governor, PlacedDistrict, Religion, TradeRoute, WonderTourism,
};
use crate::rules::{TechTree, CivicTree, Government, Policy, OneShotEffect};
use crate::rules::tech::{build_tech_tree, build_civic_tree};
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
}

/// Deterministic ID generator backed by a seeded RNG.
pub struct IdGenerator {
    rng: SmallRng,
    timestamp_ms: u64,
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

    /// Returns a pseudo-random f32 in [0.0, 1.0) drawn from the seeded RNG.
    /// Used for combat randomisation; does not affect the ULID sequence.
    pub fn next_f32(&mut self) -> f32 {
        use rand::RngCore;
        (self.rng.next_u32() as f32) / (u32::MAX as f32 + 1.0)
    }
}

/// The full game state.
#[derive(Debug)]
pub struct GameState {
    pub turn: u32,
    pub seed: u64,
    pub board: WorldBoard,
    pub id_gen: IdGenerator,
    pub civilizations: Vec<Civilization>,
    pub cities: Vec<City>,
    pub units: Vec<BasicUnit>,
    pub placed_districts: Vec<PlacedDistrict>,
    pub diplomatic_relations: Vec<DiplomaticRelation>,
    pub religions: Vec<Religion>,
    pub trade_routes: Vec<TradeRoute>,
    pub great_people: Vec<GreatPerson>,
    /// Registry of great person definitions. Populated before the game loop.
    pub great_person_defs: Vec<GreatPersonDef>,
    pub tech_tree: TechTree,
    pub tech_refs: TechRefs,
    pub civic_tree: CivicTree,
    pub civic_refs: CivicRefs,
    pub governments: Vec<Government>,
    pub policies: Vec<Policy>,
    pub current_era: EraId,
    /// Registry of unit types. Populated by callers before the game loop.
    /// `apply_effect(FreeUnit)` looks up entries by name to spawn real units.
    pub unit_type_defs: Vec<UnitTypeDef>,
    /// Registry of building types. Populated by callers before the game loop.
    /// `apply_effect(FreeBuilding)` looks up entries by name to place real buildings.
    pub building_defs: Vec<BuildingDef>,
    /// Governors owned by civilizations. Loyalty computation checks for
    /// established governors assigned to cities.
    pub governors: Vec<Governor>,
    // TODO(PHASE3-8.8): Add era_triggers: Vec<Box<dyn EraTrigger>> (or on Era struct).
    /// Active victory conditions evaluated each turn by `advance_turn`.
    /// Register before the game loop. Can be empty (no win condition).
    pub victory_conditions: Vec<Box<dyn VictoryCondition>>,
    /// Set when a civilization has won the game. `advance_turn` no longer
    /// evaluates victory conditions once this is `Some`.
    pub game_over: Option<super::victory::GameOver>,
    /// Built wonders that generate tourism per turn. Entries are added when a
    /// wonder completes production (or manually for testing).
    pub wonder_tourism: Vec<WonderTourism>,
    /// Pending one-shot effects to be drained at the end of each turn's
    /// completion sweep (Phase 4 of `advance_turn`).
    pub effect_queue: VecDeque<(CivId, OneShotEffect)>,
}

impl GameState {
    pub fn new(seed: u64, width: u32, height: u32) -> Self {
        let board = WorldBoard::new(width, height);
        let mut id_gen = IdGenerator::new(seed);
        let era_id = EraId::from_ulid(id_gen.next_ulid());
        let (tech_tree, tech_refs)   = build_tech_tree(&mut id_gen);
        let (civic_tree, civic_refs) = build_civic_tree(&mut id_gen);

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
            tech_tree,
            tech_refs,
            civic_tree,
            civic_refs,
            governments: Vec::new(),
            policies: Vec::new(),
            current_era: era_id,
            governors: Vec::new(),
            unit_type_defs: Vec::new(),
            building_defs: Vec::new(),
            victory_conditions: Vec::new(),
            game_over: None,
            wonder_tourism: Vec::new(),
            effect_queue: VecDeque::new(),
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

    pub fn great_person(&self, id: crate::GreatPersonId) -> Option<&GreatPerson> {
        self.great_people.iter().find(|gp| gp.id == id)
    }

    pub fn great_person_mut(&mut self, id: crate::GreatPersonId) -> Option<&mut GreatPerson> {
        self.great_people.iter_mut().find(|gp| gp.id == id)
    }
}
