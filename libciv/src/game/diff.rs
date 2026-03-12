use crate::{CivId, CityId, PolicyId, UnitId};
use crate::civ::DiplomaticStatus;
use crate::world::improvement::BuiltinImprovement;
use crate::world::resource::BuiltinResource;
use libhexgrid::coord::HexCoord;

/// Distinguishes how a combat event was initiated.
/// `CityAssault` is a stub for future work (attacking city walls / HP directly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    Melee,
    Ranged,
    // TODO(PHASE3-8.1): CityAssault -- siege / wall damage; stub
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
    DistrictBuilt        { city: CityId, district: &'static str, coord: HexCoord },
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

    // ── TODO(PHASE3-8.8): Era advancement ────────────────────────────────────
    // EraAdvanced { civ: CivId, new_era: crate::AgeType },

    // ── TODO(PHASE3-8.9): Victory condition ──────────────────────────────────
    // VictoryAchieved { civ: CivId, condition: &'static str },
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
