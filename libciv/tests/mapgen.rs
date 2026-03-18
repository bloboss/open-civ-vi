//! Integration tests for the map generation pipeline.

use libciv::game::board::WorldBoard;
use libciv::world::mapgen::{MapGenConfig, generate};
use libciv::world::terrain::BuiltinTerrain;
use libhexgrid::board::HexBoard;

fn standard_config(seed: u64) -> MapGenConfig {
    MapGenConfig {
        width: 40,
        height: 25,
        seed,
        land_fraction: None,
        num_continents: None,
        num_zone_seeds: None,
        num_starts: 0,
    }
}

fn is_land(t: BuiltinTerrain) -> bool {
    !matches!(t, BuiltinTerrain::Ocean | BuiltinTerrain::Coast)
}

// ---------------------------------------------------------------------------
// Test 1: land fraction within tolerance (35% – 55%)
// ---------------------------------------------------------------------------

#[test]
fn land_fraction_within_tolerance() {
    let config = standard_config(1);
    let mut board = WorldBoard::new(config.width, config.height);
    generate(&config, &mut board);

    let total = board.all_coords().len();
    let land  = board.all_coords().iter()
        .filter(|&&c| board.tile(c).map(|t| is_land(t.terrain)).unwrap_or(false))
        .count();

    let fraction = land as f32 / total as f32;
    assert!(
        fraction >= 0.30 && fraction <= 0.60,
        "land fraction {fraction:.2} outside expected 0.30..0.60"
    );
}

// ---------------------------------------------------------------------------
// Test 2: no tile has terrain == Mountain && hills == true simultaneously
// ---------------------------------------------------------------------------

#[test]
fn no_mountain_with_hills() {
    let config = standard_config(2);
    let mut board = WorldBoard::new(config.width, config.height);
    generate(&config, &mut board);

    for coord in board.all_coords() {
        if let Some(tile) = board.tile(coord) {
            assert!(
                !(tile.terrain == BuiltinTerrain::Mountain && tile.hills),
                "tile at {:?} is Mountain+hills", coord,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test 3: every land tile has a terrain that isn't the default Grassland
//         placeholder (i.e. features.rs ran and set real terrain)
// ---------------------------------------------------------------------------

#[test]
fn land_tiles_have_real_terrain() {
    let config = standard_config(3);
    let mut board = WorldBoard::new(config.width, config.height);
    // Pre-fill all tiles with a sentinel we can detect if mapgen skips them.
    // WorldBoard::new initialises to Grassland, so we check that at least
    // some non-Grassland land types exist (Desert/Plains/Snow/Tundra/Mountain).
    generate(&config, &mut board);

    let varied = board.all_coords().iter().any(|&c| {
        board.tile(c).map(|t| {
            is_land(t.terrain) && t.terrain != BuiltinTerrain::Grassland
        }).unwrap_or(false)
    });
    assert!(varied, "all land tiles are still Grassland — features phase may not have run");
}

// ---------------------------------------------------------------------------
// Test 4: starting positions validity
// ---------------------------------------------------------------------------

#[test]
fn starting_positions_valid() {
    let config = MapGenConfig {
        width: 40, height: 25, seed: 4,
        land_fraction: None, num_continents: None,
        num_zone_seeds: None, num_starts: 4,
    };
    let mut board = WorldBoard::new(config.width, config.height);
    let result = generate(&config, &mut board);

    assert!(
        !result.starting_positions.is_empty(),
        "expected at least one starting position"
    );

    for coord in &result.starting_positions {
        let tile = board.tile(*coord)
            .unwrap_or_else(|| panic!("starting position {:?} not on board", coord));

        assert!(
            matches!(tile.terrain, BuiltinTerrain::Grassland | BuiltinTerrain::Plains
                                  | BuiltinTerrain::Tundra | BuiltinTerrain::Desert
                                  | BuiltinTerrain::Snow),
            "starting position {:?} has terrain {:?} — should be land",
            coord, tile.terrain,
        );
        assert_ne!(
            tile.terrain, BuiltinTerrain::Mountain,
            "starting position {:?} is on Mountain", coord,
        );
        assert!(
            !matches!(tile.terrain, BuiltinTerrain::Ocean | BuiltinTerrain::Coast),
            "starting position {:?} is on water ({:?})", coord, tile.terrain,
        );
    }
}

// ---------------------------------------------------------------------------
// Test 5: deterministic — two calls with the same seed produce identical terrain
// ---------------------------------------------------------------------------

#[test]
fn deterministic() {
    let config = standard_config(5);

    let mut board_a = WorldBoard::new(config.width, config.height);
    generate(&config, &mut board_a);

    let mut board_b = WorldBoard::new(config.width, config.height);
    generate(&config, &mut board_b);

    for coord in board_a.all_coords() {
        let ta = board_a.tile(coord).map(|t| (t.terrain, t.hills, t.feature, t.resource));
        let tb = board_b.tile(coord).map(|t| (t.terrain, t.hills, t.feature, t.resource));
        assert_eq!(ta, tb, "mismatch at {:?}", coord);
    }
}

// ---------------------------------------------------------------------------
// Test 6: different seeds produce different boards
// ---------------------------------------------------------------------------

#[test]
fn different_seeds_differ() {
    let mut board_a = WorldBoard::new(40, 25);
    generate(&MapGenConfig::standard(40, 25, 10), &mut board_a);

    let mut board_b = WorldBoard::new(40, 25);
    generate(&MapGenConfig::standard(40, 25, 99), &mut board_b);

    let mismatches = board_a.all_coords().iter().filter(|&&c| {
        let ta = board_a.tile(c).map(|t| t.terrain);
        let tb = board_b.tile(c).map(|t| t.terrain);
        ta != tb
    }).count();

    assert!(
        mismatches > 10,
        "boards with different seeds are suspiciously similar ({mismatches} tile differences)"
    );
}
