use std::collections::HashSet;
use crate::CivId;
use crate::game::board::WorldBoard;
use crate::game::diff::{GameStateDiff, StateDelta};
use crate::game::state::GameState;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

/// Recompute `visible_tiles` and `explored_tiles` for `civ_id` using the
/// current positions and vision ranges of all that civ's units and cities.
///
/// Cities always contribute a vision radius of 2 around their center tile.
/// Returns a diff containing `TilesRevealed` for any tiles newly explored
/// (seen for the first time). The caller should merge this into the
/// surrounding diff so the TUI / RL layer can observe new discoveries.
pub fn recalculate_visibility(state: &mut GameState, civ_id: CivId) -> GameStateDiff {
    // Collect (origin, radius) from owned units.
    let mut sources: Vec<(HexCoord, u8)> = state.units.iter()
        .filter(|u| u.owner == civ_id)
        .map(|u| (u.coord, u.vision_range))
        .collect();

    // Cities always see 2 tiles around their center.
    for city in state.cities.iter().filter(|c| c.owner == civ_id) {
        sources.push((city.coord, 2));
    }

    let mut new_visible: HashSet<HexCoord> = HashSet::new();
    for (origin, radius) in &sources {
        new_visible.extend(tiles_in_vision(&state.board, *origin, *radius));
    }

    let civ_idx = match state.civilizations.iter().position(|c| c.id == civ_id) {
        Some(i) => i,
        None    => return GameStateDiff::new(),
    };

    let newly_explored: Vec<HexCoord> = new_visible.iter()
        .copied()
        .filter(|coord| !state.civilizations[civ_idx].explored_tiles.contains(coord))
        .collect();

    state.civilizations[civ_idx].explored_tiles.extend(new_visible.iter().copied());
    state.civilizations[civ_idx].visible_tiles = new_visible;

    let mut diff = GameStateDiff::new();
    if !newly_explored.is_empty() {
        // Check newly explored tiles for natural wonders.
        for &coord in &newly_explored {
            if let Some(tile) = state.board.tile(coord)
                && let Some(ref wonder) = tile.natural_wonder
            {
                diff.push(StateDelta::NaturalWonderDiscovered {
                    civ: civ_id,
                    wonder_name: wonder.as_def().name(),
                    coord,
                });
            }
        }
        diff.push(StateDelta::TilesRevealed { civ: civ_id, coords: newly_explored });
    }
    diff
}

/// Returns all tiles within `radius` hexes of `origin` that have line-of-sight
/// from `origin` on `board`. `origin` itself is always included.
///
/// Uses `WorldBoard::has_los` for each candidate, so elevation blocking
/// is respected according to the existing LOS implementation.
fn tiles_in_vision(board: &WorldBoard, origin: HexCoord, radius: u8) -> Vec<HexCoord> {
    let mut result = Vec::new();
    result.push(origin);
    for r in 1..=(radius as u32) {
        for candidate in origin.ring(r) {
            if board.normalize(candidate).is_none() {
                continue;
            }
            if board.has_los(origin, candidate) {
                result.push(candidate);
            }
        }
    }
    result
}
