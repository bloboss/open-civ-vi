//! Phase 3 -- Terrain, mountain ranges, hills, and interior features.
//!
//! Consumes the zone map and continent map produced by Phases 1-2 to write
//! final terrain types, hills flags, and feature overlays onto the board.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use rand::rngs::SmallRng;

use libhexgrid::board::HexBoard;
use libhexgrid::coord::{HexCoord, HexDir};

use crate::game::board::WorldBoard;
use crate::world::feature::BuiltinFeature;
use crate::world::terrain::BuiltinTerrain;

use super::MapGenConfig;
use super::continents::ContinentId;
use super::zones::EcoZone;

// ---------------------------------------------------------------------------
// Phase entry point
// ---------------------------------------------------------------------------

pub fn generate(
    config:        &MapGenConfig,
    board:         &mut WorldBoard,
    zone_map:      &HashMap<HexCoord, EcoZone>,
    continent_map: &HashMap<HexCoord, ContinentId>,
    rng:           &mut SmallRng,
) {
    step_a_base_terrain(board, zone_map, rng);
    step_b_mountain_ranges(config, board, zone_map, continent_map, rng);
    step_c_hill_scatter(board, zone_map, rng);
    step_d_interior_features(board, zone_map, rng);
}

// ---------------------------------------------------------------------------
// Step 3a -- base terrain from zone
// ---------------------------------------------------------------------------

fn step_a_base_terrain(
    board:    &mut WorldBoard,
    zone_map: &HashMap<HexCoord, EcoZone>,
    rng:      &mut SmallRng,
) {
    let all_coords: Vec<HexCoord> = board.all_coords();

    for coord in all_coords {
        let Some(tile) = board.tile(coord) else { continue };

        // Leave ocean / coast terrain as-is; only rewrite land tiles.
        if !is_land(tile.terrain) {
            continue;
        }

        let zone = zone_map.get(&coord).copied().unwrap_or(EcoZone::Temperate);
        let terrain = match zone {
            EcoZone::Polar    => BuiltinTerrain::Snow,
            EcoZone::Tundra   => BuiltinTerrain::Tundra,
            EcoZone::Temperate => {
                if rng.random::<f32>() < 0.60 { BuiltinTerrain::Grassland }
                else                           { BuiltinTerrain::Plains    }
            }
            EcoZone::Desert => {
                if rng.random::<f32>() < 0.90 { BuiltinTerrain::Desert }
                else                           { BuiltinTerrain::Plains }
            }
            EcoZone::Tropical => {
                if rng.random::<f32>() < 0.70 { BuiltinTerrain::Grassland }
                else                           { BuiltinTerrain::Plains    }
            }
        };

        if let Some(tile) = board.tile_mut(coord) {
            tile.terrain = terrain;
        }
    }

    // Coastal ice (Polar ocean coast) and Reef (Tropical ocean coast).
    let coastal: Vec<HexCoord> = board
        .all_coords()
        .into_iter()
        .filter(|c| {
            board.tile(*c).map(|t| t.terrain == BuiltinTerrain::Coast).unwrap_or(false)
        })
        .collect();

    for coord in coastal {
        let zone = zone_map.get(&coord).copied().unwrap_or(EcoZone::Temperate);
        let p: f32 = rng.random();
        match zone {
            EcoZone::Polar    if p < 0.40 => {
                if let Some(t) = board.tile_mut(coord) { t.feature = Some(BuiltinFeature::Ice); }
            }
            EcoZone::Tropical if p < 0.25 => {
                if let Some(t) = board.tile_mut(coord) { t.feature = Some(BuiltinFeature::Reef); }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Step 3b -- linear mountain ranges
// ---------------------------------------------------------------------------

fn step_b_mountain_ranges(
    config:        &MapGenConfig,
    board:         &mut WorldBoard,
    zone_map:      &HashMap<HexCoord, EcoZone>,
    continent_map: &HashMap<HexCoord, ContinentId>,
    rng:           &mut SmallRng,
) {
    // Group land tiles by continent; sort tile lists for deterministic RNG consumption.
    let mut by_continent: HashMap<ContinentId, Vec<HexCoord>> = HashMap::new();
    for (&coord, &cid) in continent_map {
        by_continent.entry(cid).or_default().push(coord);
    }
    for tiles in by_continent.values_mut() {
        tiles.sort();
    }
    // Sort continent entries by ID for deterministic iteration order.
    let mut continent_entries: Vec<(ContinentId, &Vec<HexCoord>)> = by_continent
        .iter()
        .map(|(&id, tiles)| (id, tiles))
        .collect();
    continent_entries.sort_by_key(|(id, _)| *id);

    let all_dirs = HexDir::ALL;
    let _w = config.width as f32;
    let _h = config.height as f32;

    for (_, land_tiles) in &continent_entries {
        let size = land_tiles.len();
        let range_count: usize = match size {
            0..=39  => 0,
            40..=120 => 1,
            121..=300 => 2,
            _ => 3,
        };

        if range_count == 0 {
            continue;
        }

        // Approximate diagonal of this continent's bounding box.
        let (min_q, max_q, min_r, max_r) = land_tiles.iter().fold(
            (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
            |(miq, maq, mir, mar), c| {
                (miq.min(c.q), maq.max(c.q), mir.min(c.r), mar.max(c.r))
            },
        );
        let diag = (((max_q - min_q) as f32).powi(2) + ((max_r - min_r) as f32).powi(2)).sqrt();
        let diag = diag.max(4.0);

        // Build a quick lookup for zone-boundary tiles (potential range starts).
        let boundary_set: HashSet<HexCoord> = land_tiles
            .iter()
            .filter(|&&coord| {
                cross_zone_count(coord, zone_map, board) >= 1
                    && coast_distance(coord, board) >= 4
            })
            .copied()
            .collect();

        let land_set: HashSet<HexCoord> = land_tiles.iter().copied().collect();

        for _ in 0..range_count {
            // Pick start tile: prefer zone boundary, interior.
            let start = {
                let interior: Vec<HexCoord> = land_tiles
                    .iter()
                    .filter(|&&c| coast_distance(c, board) >= 4)
                    .copied()
                    .collect();

                let preferred: Vec<HexCoord> = interior
                    .iter()
                    .filter(|c| boundary_set.contains(*c))
                    .copied()
                    .collect();

                let candidates = if !preferred.is_empty() { &preferred } else { &interior };
                if candidates.is_empty() {
                    continue;
                }
                candidates[rng.random_range(0..candidates.len())]
            };

            // Random initial direction.
            let mut dir = all_dirs[rng.random_range(0..6)];

            // Walk length in tiles.
            let length_f: f32 = diag * (0.20 + rng.random::<f32>() * 0.20);
            let length = (length_f as usize).max(2);

            let inertia: f32 = 0.70;
            let mut cur = start;
            let mut spine: Vec<HexCoord> = vec![cur];

            for _ in 0..length {
                // Possibly turn +-60 degrees.
                if rng.random::<f32>() > inertia {
                    dir = turn_dir(dir, rng.random::<bool>());
                }

                let next_raw = cur + dir.unit_vec();
                // Wrap q for CylindricalEW.
                let next_q = next_raw.q.rem_euclid(config.width as i32);
                let next = HexCoord::from_qr(next_q, next_raw.r);

                // Stop if off the map vertically or not land.
                if !land_set.contains(&next) {
                    break;
                }
                cur = next;
                spine.push(cur);
            }

            // Set spine tiles to Mountain.
            for &coord in &spine {
                if let Some(tile) = board.tile_mut(coord) {
                    tile.terrain = BuiltinTerrain::Mountain;
                    tile.hills   = false;
                }
            }

            // Set adjacent land tiles to hills.
            for &coord in &spine {
                for neighbor in board.neighbors(coord) {
                    if !land_set.contains(&neighbor) { continue; }
                    if let Some(tile) = board.tile(neighbor)
                        && tile.terrain == BuiltinTerrain::Mountain { continue; }
                    if let Some(tile) = board.tile_mut(neighbor) {
                        tile.hills = true;
                    }
                }
            }
        }
    }

    // --- Accent mountains (scattered, zone-boundary only) ---
    let land_coords: Vec<HexCoord> = board
        .all_coords()
        .into_iter()
        .filter(|c| board.tile(*c).map(|t| is_land(t.terrain)).unwrap_or(false))
        .collect();

    let accent_pairs: &[(EcoZone, EcoZone, f32)] = &[
        (EcoZone::Temperate, EcoZone::Desert,  0.12),
        (EcoZone::Desert,    EcoZone::Tropical, 0.08),
        (EcoZone::Tundra,    EcoZone::Temperate, 0.06),
    ];

    for &coord in &land_coords {
        let Some(tile) = board.tile(coord) else { continue };
        if tile.terrain == BuiltinTerrain::Mountain { continue; }

        let zone = zone_map.get(&coord).copied().unwrap_or(EcoZone::Temperate);

        for &(za, zb, prob) in accent_pairs {
            if zone != za && zone != zb { continue; }

            let neighbor_has_other = board.neighbors(coord).iter().any(|n| {
                let nz = zone_map.get(n).copied().unwrap_or(EcoZone::Temperate);
                (zone == za && nz == zb) || (zone == zb && nz == za)
            });

            if neighbor_has_other && rng.random::<f32>() < prob
                && let Some(tile) = board.tile_mut(coord) {
                tile.terrain = BuiltinTerrain::Mountain;
                tile.hills   = false;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Step 3c -- hill scatter
// ---------------------------------------------------------------------------

fn step_c_hill_scatter(
    board:    &mut WorldBoard,
    zone_map: &HashMap<HexCoord, EcoZone>,
    rng:      &mut SmallRng,
) {
    let land_coords: Vec<HexCoord> = board
        .all_coords()
        .into_iter()
        .filter(|c| board.tile(*c).map(|t| is_land(t.terrain)).unwrap_or(false))
        .collect();

    for &coord in &land_coords {
        let Some(tile) = board.tile(coord) else { continue };
        if tile.terrain == BuiltinTerrain::Mountain || tile.hills { continue; }

        let cross = cross_zone_count(coord, zone_map, board) as f32 / 6.0;
        let p     = 0.30 * cross + 0.06;

        if rng.random::<f32>() < p
            && let Some(tile) = board.tile_mut(coord) {
            tile.hills = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Step 3d -- interior features
// ---------------------------------------------------------------------------

fn step_d_interior_features(
    board:    &mut WorldBoard,
    zone_map: &HashMap<HexCoord, EcoZone>,
    rng:      &mut SmallRng,
) {
    let all_coords: Vec<HexCoord> = board.all_coords();

    // Track oasis placements per continent (max 4, min 4-tile separation).
    // We approximate "continent" here by just enforcing global oasis separation.
    let mut oasis_placed: Vec<HexCoord> = Vec::new();

    for &coord in &all_coords {
        let Some(tile) = board.tile(coord) else { continue };

        // Skip mountain, existing feature, water.
        if tile.terrain == BuiltinTerrain::Mountain { continue; }
        if tile.feature.is_some() { continue; }

        let is_water = matches!(tile.terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast);
        let zone     = zone_map.get(&coord).copied().unwrap_or(EcoZone::Temperate);

        if is_water {
            // Coastal ocean features already handled in step_a.
            continue;
        }

        // Is this tile adjacent to coast?
        let coast_adjacent = board.neighbors(coord).iter().any(|n| {
            board.tile(*n).map(|t| !is_land(t.terrain)).unwrap_or(false)
        });

        let r: f32 = rng.random();
        match zone {
            EcoZone::Temperate => {
                if !tile.hills && r < 0.35
                    && let Some(t) = board.tile_mut(coord) {
                    t.feature = Some(BuiltinFeature::Forest);
                }
            }
            EcoZone::Tropical => {
                // First preference: Rainforest on non-hill interior.
                if !tile.hills && !coast_adjacent && r < 0.60
                    && let Some(t) = board.tile_mut(coord) {
                    t.feature = Some(BuiltinFeature::Rainforest);
                } else if coast_adjacent && r < 0.15
                    && let Some(t) = board.tile_mut(coord) {
                    // Marsh on tropical coast-adjacent tiles.
                    t.feature = Some(BuiltinFeature::Marsh);
                }
            }
            EcoZone::Desert
                // Oasis: non-hill, isolated, global quota ~4 per "continent region".
                // We cap at 40 total oases on the whole map and enforce 4-tile separation.
                if !tile.hills && oasis_placed.len() < 40 => {
                let too_close = oasis_placed
                    .iter()
                    .any(|o: &HexCoord| o.distance(&coord) < 4);
                if !too_close && r < 0.02
                    && let Some(t) = board.tile_mut(coord) {
                    t.feature = Some(BuiltinFeature::Oasis);
                    oasis_placed.push(coord);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn is_land(terrain: BuiltinTerrain) -> bool {
    !matches!(terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast)
}

/// Count how many neighbors of `coord` are in a different EcoZone.
fn cross_zone_count(
    coord:    HexCoord,
    zone_map: &HashMap<HexCoord, EcoZone>,
    board:    &WorldBoard,
) -> usize {
    let zone = zone_map.get(&coord).copied().unwrap_or(EcoZone::Temperate);
    board
        .neighbors(coord)
        .iter()
        .filter(|n| zone_map.get(*n).copied().unwrap_or(EcoZone::Temperate) != zone)
        .count()
}

/// Minimum distance from `coord` to any non-land tile.
fn coast_distance(coord: HexCoord, board: &WorldBoard) -> usize {
    for dist in 0..=10usize {
        let ring_tiles: Vec<HexCoord> = if dist == 0 {
            vec![coord]
        } else {
            coord.ring(dist as u32)
        };
        for c in ring_tiles {
            if let Some(norm) = board.normalize(c)
                && board.tile(norm).map(|t| !is_land(t.terrain)).unwrap_or(false) {
                return dist;
            }
        }
    }
    11
}

/// Turn a HexDir by +-60 degrees.
fn turn_dir(dir: HexDir, clockwise: bool) -> HexDir {
    let dirs = HexDir::ALL;
    let idx  = dirs.iter().position(|&d| d == dir).unwrap_or(0);
    if clockwise {
        dirs[(idx + 1) % 6]
    } else {
        dirs[(idx + 5) % 6]
    }
}
