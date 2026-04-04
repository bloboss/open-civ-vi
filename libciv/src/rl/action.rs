//! Discrete action space for the RL agent.

use crate::{CityId, UnitId};
use crate::civ::ProductionItem;
use crate::world::improvement::BuiltinImprovement;
use libhexgrid::coord::HexCoord;

/// An action the RL agent can take on a single step.
///
/// The action space is intentionally small for the first version.  Higher-level
/// decisions (e.g., civic research, government adoption) can be added later.
#[derive(Debug, Clone)]
pub enum Action {
    /// End the current turn. Triggers `advance_turn`, opponent agents, and
    /// visibility recalculation.
    EndTurn,

    /// Move a unit to an adjacent hex.
    MoveUnit { unit: UnitId, to: HexCoord },

    /// Attack an enemy unit.
    Attack { attacker: UnitId, target: UnitId },

    /// Found a city with a settler unit.
    FoundCity { settler: UnitId, name: String },

    /// Queue a production item in a city.
    QueueProduction { city: CityId, item: ProductionItem },

    /// Queue a technology for research (by name in the tech tree).
    ResearchTech { tech_name: &'static str },

    /// Place an improvement on a tile (builder action).
    PlaceImprovement { coord: HexCoord, improvement: BuiltinImprovement },
}
