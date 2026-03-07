use crate::{CivId, UnitId};
use libhexgrid::coord::HexCoord;

use super::diff::GameStateDiff;
use super::state::GameState;

/// Core rules evaluation interface.
pub trait RulesEngine: std::fmt::Debug {
    /// Validate and apply a unit move. Returns the resulting diff.
    fn move_unit(
        &self,
        state: &GameState,
        unit: UnitId,
        to: HexCoord,
    ) -> Result<GameStateDiff, RulesError>;

    /// Compute all yields for a civilization this turn.
    fn compute_yields(&self, state: &GameState, civ: CivId) -> crate::YieldBundle;

    /// Advance the game state by one turn. Returns diff.
    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulesError {
    UnitNotFound,
    DestinationImpassable,
    InsufficientMovement,
    InvalidCoord,
    NotYourTurn,
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulesError::UnitNotFound => write!(f, "unit not found"),
            RulesError::DestinationImpassable => write!(f, "destination is impassable"),
            RulesError::InsufficientMovement => write!(f, "insufficient movement points"),
            RulesError::InvalidCoord => write!(f, "invalid coordinate"),
            RulesError::NotYourTurn => write!(f, "not your turn"),
        }
    }
}

impl std::error::Error for RulesError {}

/// Stub implementation — Phase 2 will fill this in.
#[derive(Debug, Default)]
pub struct DefaultRulesEngine;

impl RulesEngine for DefaultRulesEngine {
    fn move_unit(
        &self,
        _state: &GameState,
        _unit: UnitId,
        _to: HexCoord,
    ) -> Result<GameStateDiff, RulesError> {
        todo!("Phase 2: implement unit movement rules")
    }

    fn compute_yields(&self, _state: &GameState, _civ: CivId) -> crate::YieldBundle {
        todo!("Phase 2: implement yield computation")
    }

    fn advance_turn(&self, _state: &mut GameState) -> GameStateDiff {
        todo!("Phase 2: implement turn advancement")
    }
}
