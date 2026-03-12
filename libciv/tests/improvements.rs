/// Tests for terrain–improvement placement validity.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::game::StateDelta;
use libciv::world::feature::BuiltinFeature;
use libciv::world::improvement::{BuiltinImprovement, Farm, LumberMill, Mine};
use libciv::world::terrain::BuiltinTerrain;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Farm on a flat Grassland tile succeeds and emits ImprovementPlaced.
#[test]
fn farm_on_grassland_succeeds() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    // Confirm the tile is Grassland (default board setup).
    assert!(s.state.board.tile(coord).is_some());

    let rules = DefaultRulesEngine;
    let result = rules.place_improvement(
        &mut s.state,
        coord,
        BuiltinImprovement::Farm(Farm),
    );

    assert!(result.is_ok(), "Farm on Grassland should succeed");

    let diff = result.unwrap();
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::ImprovementPlaced { .. })),
        "Diff should contain ImprovementPlaced"
    );

    // Improvement is set on the tile.
    assert!(
        s.state.board.tile(coord).unwrap().improvement.is_some(),
        "Tile should now have an improvement"
    );
}

/// Farm on an Ocean tile is rejected (water tile).
#[test]
fn farm_on_ocean_fails() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    if let Some(t) = s.state.board.tile_mut(coord) {
        t.terrain = BuiltinTerrain::Ocean;
    }

    let rules = DefaultRulesEngine;
    let result = rules.place_improvement(
        &mut s.state,
        coord,
        BuiltinImprovement::Farm(Farm),
    );

    assert!(
        matches!(result, Err(libciv::game::RulesError::InvalidImprovement)),
        "Farm on Ocean should return InvalidImprovement"
    );
}

/// LumberMill requires a Forest feature; a tile without Forest is rejected.
#[test]
fn lumbermill_without_forest_fails_and_with_forest_succeeds() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    let rules = DefaultRulesEngine;

    // Without Forest: should fail.
    let result_no_forest = rules.place_improvement(
        &mut s.state,
        coord,
        BuiltinImprovement::LumberMill(LumberMill),
    );
    assert!(
        matches!(result_no_forest, Err(libciv::game::RulesError::InvalidImprovement)),
        "LumberMill without Forest should be rejected"
    );

    // Add Forest.
    if let Some(t) = s.state.board.tile_mut(coord) {
        t.feature = Some(BuiltinFeature::Forest);
    }

    let result_with_forest = rules.place_improvement(
        &mut s.state,
        coord,
        BuiltinImprovement::LumberMill(LumberMill),
    );
    assert!(result_with_forest.is_ok(), "LumberMill on Forest tile should succeed");
}
