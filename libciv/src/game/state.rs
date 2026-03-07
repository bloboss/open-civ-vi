use std::collections::VecDeque;
use crate::{
    CivId, CityId, UnitId, EraId,
};
use crate::civ::{
    BasicUnit, Civilization, City, CityKind, DiplomaticRelation, GreatPerson, Religion, TradeRoute,
};
use crate::rules::{TechTree, CivicTree, Government, Policy, OneShotEffect};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use ulid::Ulid;

use super::board::WorldBoard;

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
    pub diplomatic_relations: Vec<DiplomaticRelation>,
    pub religions: Vec<Religion>,
    pub trade_routes: Vec<TradeRoute>,
    pub great_people: Vec<GreatPerson>,
    pub tech_tree: TechTree,
    pub civic_tree: CivicTree,
    pub governments: Vec<Government>,
    pub policies: Vec<Policy>,
    pub current_era: EraId,
    /// Pending one-shot effects to be drained at the end of each turn's
    /// completion sweep (Phase 4 of `advance_turn`).
    pub effect_queue: VecDeque<(CivId, OneShotEffect)>,
}

impl GameState {
    pub fn new(seed: u64, width: u32, height: u32) -> Self {
        let board = WorldBoard::new(width, height);
        let mut id_gen = IdGenerator::new(seed);
        let era_id = EraId::from_ulid(id_gen.next_ulid());

        Self {
            turn: 0,
            seed,
            board,
            id_gen,
            civilizations: Vec::new(),
            cities: Vec::new(),
            units: Vec::new(),
            diplomatic_relations: Vec::new(),
            religions: Vec::new(),
            trade_routes: Vec::new(),
            great_people: Vec::new(),
            tech_tree: TechTree::new(),
            civic_tree: CivicTree::new(),
            governments: Vec::new(),
            policies: Vec::new(),
            current_era: era_id,
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
}
