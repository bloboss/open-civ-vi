//! Phase 1 — Continental growth.
//!
//! Places `n` continent seeds across the map using a jitter-grid approach,
//! then grows them via iterative frontier BFS until the target land fraction
//! is reached.  Remaining tiles are set to Ocean.  A final pass converts
//! ocean tiles adjacent to land into Coast.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;

use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::game::board::WorldBoard;
use crate::world::terrain::BuiltinTerrain;

use super::MapGenConfig;

/// Continent identifier (0-indexed).
pub type ContinentId = u8;

/// Run Phase 1.  Returns a map from every land tile to its continent ID.
/// Ocean / Coast tiles are absent from the returned map.
pub fn generate(
    config: &MapGenConfig,
    board:  &mut WorldBoard,
    rng:    &mut SmallRng,
) -> HashMap<HexCoord, ContinentId> {
    let n              = config.resolved_num_continents() as usize;
    let total_tiles    = (config.width * config.height) as usize;
    let land_fraction  = config.resolved_land_fraction();
    let target_land    = (total_tiles as f32 * land_fraction).round() as usize;
    let per_continent  = target_land as f32 / n as f32;

    // --- 1. Seed placement (jitter grid) ------------------------------------

    let seeds = place_seeds(config, n, rng);

    // --- 2. Frontier BFS growth ---------------------------------------------

    // continent_map records which continent owns each tile.
    let mut continent_map: HashMap<HexCoord, ContinentId> = HashMap::new();
    // Per-continent current size.
    let mut sizes: Vec<usize>                             = vec![0; n];
    // Frontier: (coord, continent_id)
    let mut frontier: Vec<(HexCoord, ContinentId)>       = Vec::new();
    // Unoccupied set for fast membership test.
    let mut unoccupied: HashSet<HexCoord>                 = board.all_coords().into_iter().collect();

    for (i, seed) in seeds.iter().enumerate() {
        let cid = i as ContinentId;
        continent_map.insert(*seed, cid);
        unoccupied.remove(seed);
        sizes[i] = 1;
        frontier.push((*seed, cid));
    }

    let mut total_land = seeds.len();

    while total_land < target_land && !frontier.is_empty() {
        frontier.shuffle(rng);

        let old_len = frontier.len();
        let mut new_frontier: Vec<(HexCoord, ContinentId)> = Vec::new();

        for &(coord, cid) in &frontier[..old_len] {
            let p: f32 = {
                let frac = sizes[cid as usize] as f32 / per_continent;
                (1.0 - frac).clamp(0.25, 0.95)
            };

            for neighbor in board.neighbors(coord) {
                if total_land >= target_land {
                    break;
                }
                if unoccupied.contains(&neighbor) && rng.random::<f32>() < p {
                    continent_map.insert(neighbor, cid);
                    unoccupied.remove(&neighbor);
                    sizes[cid as usize] += 1;
                    total_land += 1;
                    new_frontier.push((neighbor, cid));
                }
            }
        }
        frontier = new_frontier;
    }

    // --- 3. Set non-land tiles to Ocean -------------------------------------

    for coord in board.all_coords() {
        if !continent_map.contains_key(&coord) && let Some(tile) = board.tile_mut(coord) {
            tile.terrain = BuiltinTerrain::Ocean;
        }
    }

    // --- 4. Coast pass: ocean tiles adjacent to land become Coast -----------

    let ocean_coords: Vec<HexCoord> = board
        .all_coords()
        .into_iter()
        .filter(|c| board.tile(*c).map(|t| t.terrain == BuiltinTerrain::Ocean).unwrap_or(false))
        .collect();

    for coord in ocean_coords {
        let adjacent_land = board
            .neighbors(coord)
            .iter()
            .any(|n| board.tile(*n).map(|t| is_land(t.terrain)).unwrap_or(false));
        if adjacent_land && let Some(tile) = board.tile_mut(coord) {
            tile.terrain = BuiltinTerrain::Coast;
        }
    }

    continent_map
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_land(terrain: BuiltinTerrain) -> bool {
    !matches!(terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast)
}

/// Place `n` seeds across the map using a jitter-grid approach.
///
/// The map is divided into `n` equal-area rectangular sectors.  Within each
/// sector a random tile is sampled, rejected if it falls within `min_sep`
/// hexes of an already-placed seed.  Up to 8 retries per sector.
fn place_seeds(config: &MapGenConfig, n: usize, rng: &mut SmallRng) -> Vec<HexCoord> {
    let w = config.width as f32;
    let h = config.height as f32;

    // Split into a roughly square grid of n cells.
    let cols = ((n as f32).sqrt().ceil() as u32).max(1);
    let rows = (n as u32).div_ceil(cols);

    let min_sep = ((w * h / n as f32).sqrt() * 0.55) as u32;

    let mut seeds: Vec<HexCoord> = Vec::with_capacity(n);
    let mut sector_index: usize  = 0;

    'outer: for row in 0..rows {
        for col in 0..cols {
            if sector_index >= n {
                break 'outer;
            }

            let q_min = ((col as f32 / cols as f32) * w) as i32;
            let q_max = (((col + 1) as f32 / cols as f32) * w) as i32;
            let r_min = ((row as f32 / rows as f32) * h) as i32;
            let r_max = (((row + 1) as f32 / rows as f32) * h) as i32;

            let q_range = (q_max - q_min).max(1);
            let r_range = (r_max - r_min).max(1);

            let mut placed = false;
            for _attempt in 0..8 {
                let q = q_min + rng.random_range(0..q_range);
                let r = r_min + rng.random_range(0..r_range);
                let candidate = HexCoord::from_qr(q, r);

                let too_close = seeds.iter().any(|s: &HexCoord| s.distance(&candidate) < min_sep);
                if !too_close {
                    seeds.push(candidate);
                    placed = true;
                    break;
                }
            }

            // If all attempts failed, just place anywhere in the sector.
            if !placed {
                let q = q_min + rng.random_range(0..q_range);
                let r = r_min + rng.random_range(0..r_range);
                seeds.push(HexCoord::from_qr(q, r));
            }

            sector_index += 1;
        }
    }

    seeds
}
