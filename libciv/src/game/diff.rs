use crate::{
  AgeType, CivId, CityId, GreatPersonId, PolicyId, 
  TradeRouteId, UnitId};
use crate::civ::DiplomaticStatus;
use crate::civ::city::WallLevel;
use crate::civ::district::BuiltinDistrict;
use crate::civ::era::EraAge;
use crate::world::improvement::BuiltinImprovement;
use crate::world::resource::BuiltinResource;
use crate::world::road::BuiltinRoad;
use libhexgrid::coord::HexCoord;

/// Distinguishes how a combat event was initiated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttackType {
    Melee,
    Ranged,
    /// A city with walls fires a ranged attack at a nearby enemy unit.
    CityBombard,
}

/// A single atomic change to the game state.
#[derive(Debug, Clone)]
pub enum StateDelta {
    TurnAdvanced    { from: u32, to: u32 },
    UnitMoved       { unit: UnitId, from: HexCoord, to: HexCoord, cost: u32 },
    UnitCreated     { unit: UnitId, coord: HexCoord, owner: CivId },
    UnitDestroyed   { unit: UnitId },
    CityFounded     { city: CityId, coord: HexCoord, owner: CivId },
    CityCaptured    { city: CityId, new_owner: CivId, old_owner: CivId },
    PopulationGrew  { city: CityId, new_population: u32 },
    GoldChanged     { civ: CivId, delta: i32 },
    TechResearched  { civ: CivId, tech: &'static str },
    CivicCompleted  { civ: CivId, civic: &'static str },
    DiplomacyChanged { civ_a: CivId, civ_b: CivId, new_status: DiplomaticStatus },
    // ── OneShotEffect outcomes ──────────────────────────────────────────────
    ResourceRevealed     { civ: CivId, resource: BuiltinResource },
    EurekaTriggered      { civ: CivId, tech:  &'static str },
    InspirationTriggered { civ: CivId, civic: &'static str },
    UnitUnlocked         { civ: CivId, unit_type:    &'static str },
    BuildingUnlocked     { civ: CivId, building:     &'static str },
    ImprovementUnlocked  { civ: CivId, improvement:  &'static str },
    GovernmentUnlocked   { civ: CivId, government:   &'static str },
    GovernmentAdopted    { civ: CivId, government:   &'static str },
    PolicyUnlocked       { civ: CivId, policy: &'static str },
    /// A policy was removed from the slot because the new government has fewer slots.
    PolicyUnslotted      { civ: CivId, policy: PolicyId },
    PolicyAssigned       { civ: CivId, policy: PolicyId },
    /// Emitted when a free unit grant is processed. Full unit creation
    /// requires a unit-type registry (Phase 4).
    FreeUnitGranted      { civ: CivId, unit_type: &'static str, coord: HexCoord },
    /// Emitted when a free building grant is processed. Full building creation
    /// requires a building registry (Phase 4).
    FreeBuildingGranted  { civ: CivId, building: &'static str, city: CityId },
    // ── Production queue outcomes (PHASE3-4.3) ──────────────────────────────
    /// A building has been completed and added to the city.
    BuildingCompleted    { city: CityId, building: &'static str },
    /// A district has been placed on the map.
    DistrictBuilt        { city: CityId, district: BuiltinDistrict, coord: HexCoord },
    /// A wonder has been completed globally.
    WonderBuilt          { civ: CivId, wonder: &'static str, city: CityId },
    /// A new production item has moved to the front of the queue.
    ProductionStarted    { city: CityId, item: &'static str },
    // ── Citizen assignment (PHASE3-4.1) ──────────────────────────────────────
    /// A citizen has been assigned (or auto-assigned) to work a tile.
    CitizenAssigned      { city: CityId, tile: HexCoord },

    // ── Combat outcome (PHASE3-8.1) ───────────────────────────────────────────
    UnitAttacked {
        attacker:        UnitId,
        defender:        UnitId,
        attack_type:     AttackType,
        attacker_damage: u32,
        defender_damage: u32,
    },

    // ── Fog of war (PHASE3-10.2) ─────────────────────────────────────────────
    /// Tiles newly added to `explored_tiles` this move (not previously explored).
    TilesRevealed { civ: CivId, coords: Vec<HexCoord> },

    /// An improvement was placed on a tile.
    ImprovementPlaced { coord: HexCoord, improvement: BuiltinImprovement },

    /// A road was placed on a tile.
    RoadPlaced { coord: HexCoord, road: BuiltinRoad },

    /// A builder unit's remaining charges changed (after placing an improvement or road).
    ChargesChanged { unit: UnitId, remaining: u8 },

    /// A civilization's strategic resource stockpile changed (positive = gained, negative = consumed).
    StrategicResourceChanged { civ: CivId, resource: BuiltinResource, delta: i32 },

    /// A tile has been claimed by a civilization (city founding or cultural expansion).
    TileClaimed { civ: CivId, city: CityId, coord: HexCoord },

    /// A tile has been moved from one city to another within the same civilization.
    TileReassigned { civ: CivId, from_city: CityId, to_city: CityId, coord: HexCoord },

    // ── Trade routes (PHASE3-8.4) ─────────────────────────────────────────────
    /// A trade route was established by a trader unit (which is consumed).
    TradeRouteEstablished {
        route:       TradeRouteId,
        origin:      CityId,
        destination: CityId,
        owner:       CivId,
    },
    /// A trade route has expired (turns_remaining reached 0) or was cancelled.
    TradeRouteExpired { route: TradeRouteId },

    // ── City defense (PHASE3-8.3) ──────────────────────────────────────────────
    /// City walls took damage from a melee attack.
    WallDamaged { city: CityId, damage: u32, hp_remaining: u32 },
    /// City walls were destroyed (HP reached 0); walls breached.
    WallDestroyed { city: CityId, previous_level: WallLevel },

    // ── Tourism (PHASE3-8.6) ──────────────────────────────────────────────────
    /// Emitted each turn when a civ generates tourism. Records total tourism
    /// output and how much lifetime culture was accumulated this turn.
    TourismGenerated { civ: CivId, tourism: u32, lifetime_culture: u32 },
    // ── Loyalty system (PHASE3-8.6) ──────────────────────────────────────────
    /// A city's loyalty score changed during the turn. `delta` is the net
    /// change (positive = towards owner, negative = away); `new_value` is
    /// the clamped result in 0–100.
    LoyaltyChanged { city: CityId, delta: i32, new_value: i32 },
    /// A city revolted due to loyalty reaching 0. If `new_owner` is `Some`,
    /// the city flipped to the civilization exerting the highest loyalty
    /// pressure. If `None`, the city became a Free City (independent).
    CityRevolted { city: CityId, new_owner: Option<CivId>, old_owner: CivId },

    // ── Era score (PHASE3-8.8) ─────────────────────────────────────────────
    /// A civilization earned a historic moment, gaining era score.
    HistoricMomentEarned { civ: CivId, moment: &'static str, era_score: u32 },
    /// A civilization transitioned to a new era with a determined age.
    EraAdvanced { civ: CivId, new_era: AgeType, era_age: EraAge },
    // ── Great persons (PHASE3-8.6) ─────────────────────────────────────────
    /// A great person was retired (consumed) by its owner.
    GreatPersonRetired { great_person: GreatPersonId, owner: CivId },
    /// A one-time production burst was applied to a city (e.g. Great Engineer).
    ProductionBurst { city: CityId, amount: u32 },

    // ── TODO(PHASE3-8.8): Era advancement ────────────────────────────────────
    // EraAdvanced { civ: CivId, new_era: crate::AgeType },

    // ── Great works / tourism (cultural victory) ──────────────────────────────
    /// A great person created a great work and it was slotted into a city.
    GreatWorkCreated { civ: CivId, work_name: &'static str, city: CityId },

    // ── Victory condition (PHASE3-8.9) ────────────────────────────────────────
    /// Emitted when a civ wins the game. After this delta `GameState::game_over` is set.
    VictoryAchieved { civ: CivId, condition: &'static str },
}

/// A batch of deltas representing a complete state transition.
#[derive(Debug, Clone, Default)]
pub struct GameStateDiff {
    pub deltas: Vec<StateDelta>,
}

impl GameStateDiff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, delta: StateDelta) {
        self.deltas.push(delta);
    }

    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    pub fn len(&self) -> usize {
        self.deltas.len()
    }
}
