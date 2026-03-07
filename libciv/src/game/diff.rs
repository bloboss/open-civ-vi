use crate::{CivId, CityId, UnitId};
use crate::world::resource::BuiltinResource;
use libhexgrid::coord::HexCoord;

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
    DiplomacyChanged { civ_a: CivId, civ_b: CivId, new_status: String },
    // ── OneShotEffect outcomes ──────────────────────────────────────────────
    ResourceRevealed     { civ: CivId, resource: BuiltinResource },
    EurekaTriggered      { civ: CivId, tech:  &'static str },
    InspirationTriggered { civ: CivId, civic: &'static str },
    UnitUnlocked         { civ: CivId, unit_type:    &'static str },
    BuildingUnlocked     { civ: CivId, building:     &'static str },
    ImprovementUnlocked  { civ: CivId, improvement:  &'static str },
    GovernmentUnlocked   { civ: CivId, government:   &'static str },
    GovernmentAdopted    { civ: CivId, government:   &'static str },
    /// Emitted when a free unit grant is processed. Full unit creation
    /// requires a unit-type registry (Phase 4).
    FreeUnitGranted      { civ: CivId, unit_type: &'static str, coord: HexCoord },
    /// Emitted when a free building grant is processed. Full building creation
    /// requires a building registry (Phase 4).
    FreeBuildingGranted  { civ: CivId, building: &'static str, city: CityId },
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
