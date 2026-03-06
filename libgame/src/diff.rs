use libcommon::{CivId, CityId, UnitId};
use libhexgrid::coord::HexCoord;

/// A single atomic change to the game state.
#[derive(Debug, Clone)]
pub enum StateDelta {
    TurnAdvanced { from: u32, to: u32 },
    UnitMoved { unit: UnitId, from: HexCoord, to: HexCoord },
    UnitCreated { unit: UnitId, coord: HexCoord, owner: CivId },
    UnitDestroyed { unit: UnitId },
    CityFounded { city: CityId, coord: HexCoord, owner: CivId },
    CityCaptured { city: CityId, new_owner: CivId, old_owner: CivId },
    GoldChanged { civ: CivId, delta: i32 },
    TechResearched { civ: CivId, tech: &'static str },
    CivicCompleted { civ: CivId, civic: &'static str },
    DiplomacyChanged { civ_a: CivId, civ_b: CivId, new_status: String },
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
