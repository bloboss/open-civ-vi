//! Procedural map generation pipeline.
//!
//! Call order: continents -> zones -> features -> rivers -> resources -> starts
//!
//! Each phase is a pure function over a `WorldBoard` and a seeded RNG.
//! Phases communicate through plain data (HashMaps, Vecs) — no shared mutable state.

mod continents;
mod features;
mod resources;
mod rivers;
mod starts;
mod zones;

use rand::rngs::SmallRng;
use rand::SeedableRng;
use libhexgrid::coord::HexCoord;

use crate::game::board::WorldBoard;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Full configuration for one map generation run.
///
/// All `Option` fields default to a value derived from the map dimensions when
/// `None`.  The public `resolved_*` helpers expose the final values so callers
/// can inspect what was actually used.
#[derive(Debug, Clone)]
pub struct MapGenConfig {
    pub width:          u32,
    pub height:         u32,
    pub seed:           u64,
    /// Target land/ocean ratio.  `None` = linear interpolation from map size:
    ///   tiny (~2 280 tiles) -> 0.50, huge (~7 000 tiles) -> 0.35.
    pub land_fraction:  Option<f32>,
    /// Number of continent seeds.  `None` = area / 400, clamped 2..=7.
    pub num_continents: Option<u32>,
    /// Zone seeds per zone type.  `None` = area / 300, clamped 2..=8.
    pub num_zone_seeds: Option<u32>,
    /// How many valid starting positions Phase 6 must return.  0 = skip.
    pub num_starts:     u32,
}

impl MapGenConfig {
    /// Convenience constructor: all optional fields set to auto-derived defaults.
    pub fn standard(width: u32, height: u32, seed: u64) -> Self {
        Self {
            width,
            height,
            seed,
            land_fraction:  None,
            num_continents: None,
            num_zone_seeds: None,
            num_starts:     0,
        }
    }

    /// Resolved land fraction: 0.50 on tiny maps, 0.35 on huge, linear blend.
    pub fn resolved_land_fraction(&self) -> f32 {
        self.land_fraction.unwrap_or_else(|| {
            let area = (self.width * self.height) as f32;
            let t = ((area - 2_280.0) / (7_000.0 - 2_280.0)).clamp(0.0, 1.0);
            0.50 - t * 0.15
        })
    }

    /// Resolved continent count.
    pub fn resolved_num_continents(&self) -> u32 {
        self.num_continents.unwrap_or_else(|| {
            let area = self.width * self.height;
            (area / 400).clamp(2, 7)
        })
    }

    /// Resolved zone seeds per type.
    pub fn resolved_num_zone_seeds(&self) -> u32 {
        self.num_zone_seeds.unwrap_or_else(|| {
            let area = self.width * self.height;
            (area / 300).clamp(2, 8)
        })
    }
}

/// Output from the generation pipeline.
#[derive(Debug, Default)]
pub struct MapGenResult {
    /// Habitable starting positions (one per `config.num_starts`).
    /// Empty when `num_starts == 0`.
    pub starting_positions: Vec<HexCoord>,
}

// ---------------------------------------------------------------------------
// Pipeline entry point
// ---------------------------------------------------------------------------

/// Run the full map-generation pipeline against `board`, mutating it in place.
///
/// `board` must have been freshly constructed with `WorldBoard::new(w, h)`.
/// An independent `SmallRng` is seeded from `config.seed` so that terrain
/// generation does not interfere with the game's `IdGenerator` RNG stream.
pub fn generate(config: &MapGenConfig, board: &mut WorldBoard) -> MapGenResult {
    let mut rng = SmallRng::seed_from_u64(config.seed.wrapping_add(0xDEAD_BEEF));

    // Phase 1: continental growth
    let continent_map = continents::generate(config, board, &mut rng);

    // Phase 2: ecological zones (covers all tiles, land and ocean)
    let zone_map = zones::generate(config, board, &mut rng);

    // Phase 3: terrain types, mountain ranges, hills, interior features
    features::generate(config, board, &zone_map, &continent_map, &mut rng);

    // Phase 4: rivers
    rivers::generate(board, &mut rng);

    // Phase 5: resources
    resources::generate(board, &zone_map, &mut rng);

    // Phase 6: starting positions
    let starting_positions = if config.num_starts > 0 {
        starts::generate(board, config.num_starts, &mut rng)
    } else {
        Vec::new()
    };

    MapGenResult { starting_positions }
}
