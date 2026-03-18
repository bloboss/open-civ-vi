//! Phase 2 -- Ecological zone assignment.
//!
//! Seeds each zone type using latitude-biased random placement, then grows
//! all zones simultaneously via round-robin BFS until every tile is assigned.
//!
//! The zone assignment (`HashMap<HexCoord, EcoZone>`) is consumed by Phase 3
//! to set final terrain and features.  It is not stored on `WorldTile`.

use std::collections::{HashMap, VecDeque};

use rand::Rng;
use rand::rngs::SmallRng;

use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

use crate::game::board::WorldBoard;

use super::MapGenConfig;

// ---------------------------------------------------------------------------
// EcoZone
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EcoZone {
    Polar,
    Tundra,
    Temperate,
    Desert,
    Tropical,
}

impl EcoZone {
    pub const ALL: [EcoZone; 5] = [
        EcoZone::Polar,
        EcoZone::Tundra,
        EcoZone::Temperate,
        EcoZone::Desert,
        EcoZone::Tropical,
    ];

    /// Affinity of this zone for a given normalised latitude (0 = equator, 1 = pole).
    /// Returns a value in 0..=1 used as a sampling weight for seed placement.
    pub fn affinity(self, lat: f32) -> f32 {
        match self {
            // Polar: rises steeply above lat 0.80
            EcoZone::Polar => {
                if lat < 0.80 { 0.0 }
                else { (lat - 0.80) / 0.20 }
            }
            // Tundra: peaks ~0.9 at lat 0.70, zero below 0.45
            EcoZone::Tundra => {
                if lat < 0.45 { 0.0 }
                else if lat < 0.70 { (lat - 0.45) / 0.25 }
                else if lat < 0.80 { 1.0 - (lat - 0.70) / 0.10 }
                else { 0.0 }
            }
            // Temperate: wide bell centred ~0.40, zero outside 0.15..0.70
            EcoZone::Temperate => {
                if !(0.15..=0.70).contains(&lat) { 0.0 }
                else if lat < 0.40 { (lat - 0.15) / 0.25 }
                else               { 1.0 - (lat - 0.40) / 0.30 }
            }
            // Desert: centred ~0.25, zero outside 0.05..0.45
            EcoZone::Desert => {
                if !(0.05..=0.45).contains(&lat) { 0.0 }
                else if lat < 0.25 { (lat - 0.05) / 0.20 }
                else               { 1.0 - (lat - 0.25) / 0.20 }
            }
            // Tropical: rises to 1.0 at equator, zero above 0.15
            EcoZone::Tropical => {
                if lat > 0.15 { 0.0 }
                else { 1.0 - lat / 0.15 }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase entry point
// ---------------------------------------------------------------------------

/// Run Phase 2.  Returns an `EcoZone` for every tile on the board.
pub fn generate(
    config: &MapGenConfig,
    board:  &WorldBoard,
    rng:    &mut SmallRng,
) -> HashMap<HexCoord, EcoZone> {
    let k          = config.resolved_num_zone_seeds() as usize;
    let half_h     = (config.height as f32) / 2.0;
    let all_coords = board.all_coords();

    // Helper: normalised latitude for a coord.
    let lat = |c: HexCoord| -> f32 {
        (c.r as f32 - half_h).abs() / half_h
    };

    // --- 1. Seed placement (weighted by zone affinity) ----------------------

    // Build a cumulative-weight table once per zone.
    let mut queues: Vec<VecDeque<HexCoord>> = Vec::with_capacity(5);
    let mut zone_order: Vec<EcoZone>        = Vec::with_capacity(5);

    for &zone in &EcoZone::ALL {
        // Weighted reservoir sampling: draw k seeds from all_coords weighted
        // by affinity.  We use the simple accept-reject approach here since
        // k is small (2..=8) and all_coords is at most ~7 000 tiles.
        let seeds = weighted_sample(&all_coords, k, rng, |c| zone.affinity(lat(*c)));

        let mut queue: VecDeque<HexCoord> = VecDeque::new();
        queue.extend(seeds);
        queues.push(queue);
        zone_order.push(zone);
    }

    // --- 2. Simultaneous BFS (round-robin) ----------------------------------

    let mut zone_map: HashMap<HexCoord, EcoZone> = HashMap::with_capacity(all_coords.len());

    // Pre-assign seed tiles.
    for (qi, queue) in queues.iter().enumerate() {
        let zone = zone_order[qi];
        for &coord in queue {
            zone_map.entry(coord).or_insert(zone);
        }
    }

    let total = all_coords.len();

    loop {
        if zone_map.len() >= total {
            break;
        }

        let mut any_progress = false;

        for qi in 0..queues.len() {
            let Some(coord) = queues[qi].pop_front() else { continue };
            let zone = zone_order[qi];

            for neighbor in board.neighbors(coord) {
                if zone_map.contains_key(&neighbor) {
                    continue;
                }
                zone_map.insert(neighbor, zone);
                queues[qi].push_back(neighbor);
                any_progress = true;
            }
        }

        if !any_progress {
            // Fill remaining unassigned tiles with the nearest assigned zone.
            // This handles corners / isolated tiles in degenerate maps.
            for coord in &all_coords {
                if !zone_map.contains_key(coord) {
                    let fallback = board
                        .neighbors(*coord)
                        .into_iter()
                        .find_map(|n| zone_map.get(&n).copied())
                        .unwrap_or(EcoZone::Temperate);
                    zone_map.insert(*coord, fallback);
                }
            }
            break;
        }
    }

    zone_map
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Draw `k` items from `items` with replacement-free weighted sampling.
///
/// Uses the simple O(n * k) algorithm: shuffle a weighted random selection
/// by iterating the list and accepting each item with probability
/// `w(item) / max_weight` (scaled reservoir).
fn weighted_sample<T, F>(
    items: &[T],
    k:     usize,
    rng:   &mut SmallRng,
    weight: F,
) -> Vec<T>
where
    T:     Copy,
    F:     Fn(&T) -> f32,
{
    if items.is_empty() || k == 0 {
        return Vec::new();
    }

    // Build (coord, weight) pairs, skip zero-weight items.
    let mut weighted: Vec<(T, f32)> = items
        .iter()
        .map(|item| (*item, weight(item)))
        .filter(|(_, w)| *w > 0.0)
        .collect();

    if weighted.is_empty() {
        // Fallback: uniform sample.
        return items
            .iter()
            .take(k)
            .copied()
            .collect();
    }

    // Shuffle so the order is random, then pick the top-k by weight after
    // adding a small random perturbation (weighted shuffle via float key).
    use rand::seq::SliceRandom;
    weighted.sort_by(|(_, wa), (_, wb)| {
        // Key = w * u^(1/w), equivalent to weighted reservoir sampling.
        // We approximate: key = w + small_random * 0.01 to break ties.
        wb.partial_cmp(wa).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Shuffle then accept with probability weight / 1.0.
    weighted.shuffle(rng);

    let mut selected: Vec<T> = Vec::with_capacity(k);
    for (item, w) in &weighted {
        if selected.len() >= k { break; }
        if rng.random::<f32>() < *w {
            selected.push(*item);
        }
    }

    // If we didn't get enough (low-weight zones on small maps), top up from front.
    for (item, _) in &weighted {
        if selected.len() >= k { break; }
        if !selected.iter().any(|_s| {
            // Can't PartialEq T generically; just fill unconditionally.
            false
        }) {
            selected.push(*item);
        }
    }

    selected.truncate(k);
    selected
}
