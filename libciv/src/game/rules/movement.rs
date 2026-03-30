//! Movement handler: `move_unit`, `decrement_builder_charges`.

use crate::UnitId;
use crate::civ::unit::Unit;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::MovementCost;
use libhexgrid::{HexEdge, HexTile};

use super::RulesError;
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::rules_helpers::neighbor_dir;
use super::super::state::GameState;

/// Validate and apply a unit move. Returns the resulting diff, or
/// `Err(InsufficientMovement(partial_diff))` when the unit cannot reach
/// the destination within its remaining movement budget.
pub(crate) fn move_unit(
    state: &GameState,
    unit_id: UnitId,
    to: HexCoord,
) -> Result<GameStateDiff, RulesError> {
    let unit = state.unit(unit_id).ok_or(RulesError::UnitNotFound)?;
    let from   = unit.coord();
    let budget = unit.movement_left();

    let to_norm = state.board.normalize(to).ok_or(RulesError::InvalidCoord)?;

    // Determine whether a path to the destination exists at all.
    let full_path = state.board
        .find_path(from, to_norm, u32::MAX)
        .ok_or(RulesError::DestinationImpassable)?;

    // Walk the path, consuming movement budget step by step.
    let mut spent  = 0u32;
    let mut reached = from;

    for i in 1..full_path.len() {
        let prev = full_path[i - 1];
        let next = full_path[i];

        let tile_cost = match state.board.tile(next) {
            Some(t) => {
                let base = match t.road.as_ref() {
                    Some(r) => r.as_def().movement_cost(),
                    None    => t.movement_cost(),
                };
                match base {
                    MovementCost::Impassable => break,
                    MovementCost::Cost(c)    => c,
                }
            }
            None => break,
        };

        // Edge crossing cost: free (0) when no edge feature exists.
        let edge_cost: u32 = {
            let crossing = neighbor_dir(&state.board, prev, next)
                .and_then(|dir| state.board.edge(prev, dir))
                .map(|e| e.crossing_cost());
            match crossing {
                Some(MovementCost::Impassable) => break,
                Some(MovementCost::Cost(c))    => c,
                None                           => 0,
            }
        };

        let step = tile_cost + edge_cost;
        if spent + step > budget {
            break;
        }
        spent   += step;
        reached  = next;
    }

    let mut diff = GameStateDiff::new();

    if reached == from {
        // Zero movement occurred (budget was 0 or first step too costly).
        return Err(RulesError::InsufficientMovement(diff));
    }

    // Occupancy check: reject if the destination is held by any other unit.
    if let Some(occupant) = state.units.iter().find(|u| u.id != unit_id && u.coord == reached) {
        let mover_owner      = state.unit(unit_id).map(|u| u.owner);
        let mover_can_attack = state.unit(unit_id).and_then(|u| u.combat_strength).is_some();
        if occupant.owner == mover_owner.unwrap_or(occupant.owner) {
            // Friendly unit on destination — stacking not allowed.
            return Err(RulesError::TileOccupiedByUnit);
        } else if !mover_can_attack {
            // Civilian trying to move onto an enemy — it cannot fight back.
            return Err(RulesError::UnitCannotAttack);
        } else {
            // Combat unit vs enemy: player must call attack() explicitly.
            return Err(RulesError::TileOccupiedByUnit);
        }
    }

    diff.push(StateDelta::UnitMoved {
        unit: unit_id,
        from,
        to: reached,
        cost: spent });

    if reached == to_norm {
        Ok(diff)
    } else {
        // Partial move: unit moved but did not reach the destination.
        Err(RulesError::InsufficientMovement(diff))
    }
}

/// Decrement builder charges and destroy the unit if charges reach 0.
pub(crate) fn decrement_builder_charges(state: &mut GameState, unit_id: UnitId, diff: &mut GameStateDiff) {
    if let Some(unit) = state.unit_mut(unit_id)
        && let Some(ref mut c) = unit.charges
    {
        *c = c.saturating_sub(1);
        let remaining = *c;
        diff.push(StateDelta::ChargesChanged { unit: unit_id, remaining });
        if remaining == 0 {
            diff.push(StateDelta::UnitDestroyed { unit: unit_id });
            state.units.retain(|u| u.id != unit_id);
        }
    }
}
