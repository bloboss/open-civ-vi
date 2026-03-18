//! Phase 4 -- River generation.
//!
//! Selects source tiles (preferring hills), then traces a path to the nearest
//! Coast/Ocean tile using Dijkstra with terrain-based flow costs.  River edges
//! are placed on the board via `set_edge()` and recorded in `tile.rivers`.

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;

use libhexgrid::board::HexBoard;
use libhexgrid::coord::{HexCoord, HexDir};

use crate::game::board::WorldBoard;
use crate::world::edge::{BuiltinEdgeFeature, River, WorldEdge};
use crate::world::terrain::BuiltinTerrain;

use super::features::is_land;

// ---------------------------------------------------------------------------
// Edge canonicalization helper
// ---------------------------------------------------------------------------

/// Return the canonical (forward-half) form of a (coord, dir) edge reference.
/// Forward half: {E, NE, NW}.  Backward half: step to neighbor, flip direction.
fn canonical_edge(coord: HexCoord, dir: HexDir) -> (HexCoord, HexDir) {
    match dir {
        HexDir::E | HexDir::NE | HexDir::NW => (coord, dir),
        backward => (coord + backward.unit_vec(), backward.opposite()),
    }
}

// Flow cost: lower = preferred path for river to flow through.
fn flow_cost(terrain: BuiltinTerrain, hills: bool) -> Option<u32> {
    match terrain {
        BuiltinTerrain::Mountain              => None,      // impassable
        BuiltinTerrain::Ocean | BuiltinTerrain::Coast => Some(0), // terminus
        _ if hills                            => Some(1),   // headwaters
        BuiltinTerrain::Plains
            | BuiltinTerrain::Tundra
            | BuiltinTerrain::Desert          => Some(2),
        BuiltinTerrain::Snow                  => Some(3),
        _                                     => Some(3),   // Grassland etc.
    }
}

fn is_terminus(terrain: BuiltinTerrain) -> bool {
    matches!(terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast)
}

// ---------------------------------------------------------------------------
// Phase entry point
// ---------------------------------------------------------------------------

pub fn generate(board: &mut WorldBoard, rng: &mut SmallRng) {
    let all_coords: Vec<HexCoord> = board.all_coords();
    let land_count = all_coords
        .iter()
        .filter(|&&c| board.tile(c).map(|t| is_land(t.terrain)).unwrap_or(false))
        .count();

    // One source per 200 land tiles, minimum 1.
    let target_sources = ((land_count / 200) + 1).max(1);

    // Build candidate list: land, non-Mountain, non-Coast.
    // Hills get 2x weight (appear twice in the list).
    let mut candidates: Vec<HexCoord> = Vec::new();
    for &coord in &all_coords {
        let Some(tile) = board.tile(coord) else { continue };
        if !is_land(tile.terrain) { continue; }
        if tile.terrain == BuiltinTerrain::Mountain { continue; }
        if tile.terrain == BuiltinTerrain::Coast    { continue; }
        candidates.push(coord);
        if tile.hills { candidates.push(coord); } // 2x weight for hills
    }
    candidates.shuffle(rng);

    // Select sources with minimum 5-tile separation.
    let min_sep: u32 = 5;
    let mut sources: Vec<HexCoord> = Vec::new();
    for &cand in &candidates {
        if sources.len() >= target_sources { break; }
        if sources.iter().all(|s| s.distance(&cand) >= min_sep) {
            sources.push(cand);
        }
    }

    // For each source: find path to coast, place river edges.
    for source in sources {
        if let Some(path) = dijkstra_to_coast(source, board) {
            place_river_edges(&path, board);
        }
    }
}

// ---------------------------------------------------------------------------
// Dijkstra to nearest coast
// ---------------------------------------------------------------------------

fn dijkstra_to_coast(source: HexCoord, board: &WorldBoard) -> Option<Vec<HexCoord>> {
    let mut dist: HashMap<HexCoord, u32> = HashMap::new();
    let mut prev: HashMap<HexCoord, HexCoord> = HashMap::new();
    let mut heap: BinaryHeap<(Reverse<u32>, HexCoord)> = BinaryHeap::new();

    dist.insert(source, 0);
    heap.push((Reverse(0), source));

    let mut terminus: Option<HexCoord> = None;

    while let Some((Reverse(cost), coord)) = heap.pop() {
        if cost > *dist.get(&coord).unwrap_or(&u32::MAX) {
            continue;
        }

        if let Some(tile) = board.tile(coord)
            && is_terminus(tile.terrain) {
            terminus = Some(coord);
            break;
        }

        for dir in HexDir::ALL {
            let neighbor_raw = coord + dir.unit_vec();
            let Some(neighbor) = board.normalize(neighbor_raw) else { continue };
            let Some(tile)     = board.tile(neighbor)          else { continue };

            let Some(step_cost) = flow_cost(tile.terrain, tile.hills) else { continue };

            let next_cost = cost + step_cost;
            let prev_best = *dist.get(&neighbor).unwrap_or(&u32::MAX);
            if next_cost < prev_best {
                dist.insert(neighbor, next_cost);
                prev.insert(neighbor, coord);
                heap.push((Reverse(next_cost), neighbor));
            }
        }
    }

    let end = terminus?;

    // Reconstruct path.
    let mut path = vec![end];
    let mut cur  = end;
    while cur != source {
        cur = *prev.get(&cur)?;
        path.push(cur);
    }
    path.reverse();
    Some(path)
}

// ---------------------------------------------------------------------------
// Edge placement
// ---------------------------------------------------------------------------

fn place_river_edges(path: &[HexCoord], board: &mut WorldBoard) {
    if path.len() < 2 {
        return;
    }

    // Collect (a, dir) pairs before mutating board.
    let mut edge_specs: Vec<(HexCoord, HexDir)> = Vec::with_capacity(path.len() - 1);
    for window in path.windows(2) {
        let a   = window[0];
        let b   = window[1];
        let delta = b - a;
        let Some(dir) = HexDir::ALL.iter().find(|&&d| d.unit_vec() == delta).copied() else {
            continue; // b is not adjacent to a (shouldn't happen after Dijkstra)
        };
        edge_specs.push((a, dir));
    }

    // Place edges and record in tile.rivers.
    for (a, dir) in edge_specs {
        let b_raw = a + dir.unit_vec();
        let Some(b) = board.normalize(b_raw) else { continue };

        let river_feature = BuiltinEdgeFeature::River(River);

        // WorldEdge::new requires a forward-half direction; canonicalize first.
        let (canon_coord, canon_dir) = canonical_edge(a, dir);
        let edge = WorldEdge::new(canon_coord, canon_dir).with_feature(river_feature);
        board.set_edge(canon_coord, canon_dir, edge);

        // Append to tile.rivers on both sides.
        if let Some(tile) = board.tile_mut(a) {
            tile.rivers.push(river_feature);
        }
        if let Some(tile) = board.tile_mut(b) {
            tile.rivers.push(river_feature);
        }
    }

    // Post-river terrain: Floodplain in Desert, Marsh near coast.
    apply_post_river_features(path, board);
}

fn apply_post_river_features(path: &[HexCoord], board: &mut WorldBoard) {
    let coast_threshold = 2usize; // last N steps before terminus

    for (i, &coord) in path.iter().enumerate() {
        let Some(tile) = board.tile(coord) else { continue };
        if tile.feature.is_some() { continue; }

        let near_coast = i + coast_threshold >= path.len();

        // Count river edges on this tile.
        let river_edge_count = tile.rivers.len();

        // tile borrow ends here; mutable borrow follows.
        let needs_floodplain = tile.terrain == BuiltinTerrain::Desert && river_edge_count >= 2;
        let needs_marsh = near_coast
            && matches!(tile.terrain, BuiltinTerrain::Grassland | BuiltinTerrain::Plains);

        if needs_floodplain && let Some(t) = board.tile_mut(coord) {
            t.feature = Some(super::super::feature::BuiltinFeature::Floodplain);
        } else if needs_marsh && let Some(t) = board.tile_mut(coord) {
            t.feature = Some(super::super::feature::BuiltinFeature::Marsh);
        }
    }
}
