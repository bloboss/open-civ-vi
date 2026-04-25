//! Phase 6 -- Starting position selection.
//!
//! Scores all eligible land tiles and greedily picks `num_starts` positions
//! that are well-separated from each other.

use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;

use crate::game::board::WorldBoard;
use crate::world::feature::BuiltinFeature;
use crate::world::resource::BuiltinResource;
use crate::world::terrain::BuiltinTerrain;

use super::features::is_land;

// ---------------------------------------------------------------------------
// Phase entry point
// ---------------------------------------------------------------------------

pub fn generate(board: &WorldBoard, num_starts: u32, rng: &mut SmallRng) -> Vec<HexCoord> {
    if num_starts == 0 {
        return Vec::new();
    }

    let land_count = board
        .all_coords()
        .iter()
        .filter(|&&c| board.tile(c).map(|t| is_land(t.terrain)).unwrap_or(false))
        .count();

    let min_sep = ((land_count as f32 / num_starts as f32).sqrt() * 0.7)
        .max(8.0) as u32;

    // First pass: strict eligibility
    if let Some(positions) = pick_starts(board, num_starts, min_sep, false, rng) {
        return positions;
    }

    // Relaxed: allow hills
    if let Some(positions) = pick_starts(board, num_starts, min_sep, true, rng) {
        return positions;
    }

    // Last resort: any land tile that isn't Mountain
    fallback_starts(board, num_starts, min_sep, rng)
}

// ---------------------------------------------------------------------------
// Eligibility scoring
// ---------------------------------------------------------------------------

/// Returns `true` if the tile is an eligible start (strict mode: no hills).
fn is_eligible(tile: &crate::world::tile::WorldTile, allow_hills: bool) -> bool {
    matches!(tile.terrain, BuiltinTerrain::Grassland | BuiltinTerrain::Plains)
        && (allow_hills || !tile.hills)
        && !matches!(tile.feature, Some(BuiltinFeature::Rainforest)
                                 | Some(BuiltinFeature::Marsh)
                                 | Some(BuiltinFeature::Ice))
}

/// Score a candidate tile based on its surroundings.
///
/// Higher is better:
///   +2 per adjacent Grassland tile
///   +1 per adjacent Plains tile
///   +2 if any adjacent river edge
///   +1 if any adjacent bonus resource
fn score(coord: HexCoord, board: &WorldBoard) -> i32 {
    let mut s = 0i32;

    let has_river = board
        .tile(coord)
        .map(|t| !t.rivers.is_empty())
        .unwrap_or(false);
    if has_river {
        s += 2;
    }

    for neighbor in board.neighbors(coord) {
        let Some(t) = board.tile(neighbor) else { continue };
        match t.terrain {
            BuiltinTerrain::Grassland => s += 2,
            BuiltinTerrain::Plains    => s += 1,
            _ => {}
        }
        // Adjacent Mountain: penalise
        if t.terrain == BuiltinTerrain::Mountain {
            s -= 4;
        }
        if let Some(res) = t.resource
            && is_bonus_resource(res) {
            s += 1;
        }
    }

    // Own-tile bonus resource
    if let Some(t) = board.tile(coord)
        && let Some(res) = t.resource
        && is_bonus_resource(res) {
        s += 1;
    }

    s
}

fn is_bonus_resource(r: BuiltinResource) -> bool {
    matches!(r, BuiltinResource::Wheat
              | BuiltinResource::Rice
              | BuiltinResource::Cattle
              | BuiltinResource::Sheep
              | BuiltinResource::Fish
              | BuiltinResource::Stone
              | BuiltinResource::Copper
              | BuiltinResource::Deer)
}

// ---------------------------------------------------------------------------
// Greedy selection
// ---------------------------------------------------------------------------

fn pick_starts(
    board:      &WorldBoard,
    num_starts: u32,
    min_sep:    u32,
    allow_hills: bool,
    rng:        &mut SmallRng,
) -> Option<Vec<HexCoord>> {
    // Collect and score eligible candidates.
    let mut candidates: Vec<(HexCoord, i32)> = board
        .all_coords()
        .into_iter()
        .filter_map(|c| {
            let t = board.tile(c)?;
            if !is_eligible(t, allow_hills) { return None; }
            Some((c, score(c, board)))
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Shuffle first to break ties randomly, then sort descending by score.
    candidates.shuffle(rng);
    candidates.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut placed: Vec<HexCoord> = Vec::new();
    for (coord, _s) in &candidates {
        if placed.len() >= num_starts as usize { break; }
        let too_close = placed.iter().any(|p| p.distance(coord) < min_sep);
        if !too_close {
            placed.push(*coord);
        }
    }

    if placed.len() >= num_starts as usize {
        Some(placed)
    } else {
        None
    }
}

fn fallback_starts(
    board:      &WorldBoard,
    num_starts: u32,
    min_sep:    u32,
    rng:        &mut SmallRng,
) -> Vec<HexCoord> {
    let mut candidates: Vec<HexCoord> = board
        .all_coords()
        .into_iter()
        .filter(|&c| {
            board.tile(c)
                .map(|t| is_land(t.terrain) && t.terrain != BuiltinTerrain::Mountain)
                .unwrap_or(false)
        })
        .collect();

    candidates.shuffle(rng);

    let mut placed: Vec<HexCoord> = Vec::new();
    for coord in &candidates {
        if placed.len() >= num_starts as usize { break; }
        let too_close = placed.iter().any(|p| p.distance(coord) < min_sep);
        if !too_close {
            placed.push(*coord);
        }
    }

    // If even that fails, just return the first `num_starts` land tiles.
    if placed.len() < num_starts as usize {
        for coord in &candidates {
            if placed.len() >= num_starts as usize { break; }
            if !placed.contains(coord) {
                placed.push(*coord);
            }
        }
    }

    placed
}
