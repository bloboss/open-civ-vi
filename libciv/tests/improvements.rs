/// Tests for terrain–improvement placement validity.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::game::StateDelta;
use libciv::world::feature::BuiltinFeature;
use libciv::world::improvement::BuiltinImprovement;
use libciv::world::terrain::BuiltinTerrain;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Mark a tile as owned by the given civilization.
fn claim_tile(state: &mut libciv::GameState, coord: HexCoord, civ_id: libciv::CivId) {
    if let Some(t) = state.board.tile_mut(coord) {
        t.owner = Some(civ_id);
    }
}

// ---------------------------------------------------------------------------
// Original 3 tests (updated)
// ---------------------------------------------------------------------------

/// Farm on a flat Grassland tile succeeds (with Pottery) and emits ImprovementPlaced.
#[test]
fn farm_on_grassland_succeeds() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    // Claim tile for Rome so the ownership check passes.
    claim_tile(&mut s.state, coord, s.rome_id);

    // Grant Pottery so the tech check passes.
    let pottery_id = s.state.tech_refs.pottery;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(pottery_id);

    assert!(s.state.board.tile(coord).is_some());

    let rules = DefaultRulesEngine;
    let result = rules.place_improvement(
        &mut s.state,
        s.rome_id,
        coord,
        BuiltinImprovement::Farm,
    );

    assert!(result.is_ok(), "Farm on Grassland should succeed: {result:?}");

    let diff = result.unwrap();
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::ImprovementPlaced { .. })),
        "Diff should contain ImprovementPlaced"
    );

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

    // Grant Pottery — we want the terrain check to fire, not tech.
    let pottery_id = s.state.tech_refs.pottery;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(pottery_id);

    if let Some(t) = s.state.board.tile_mut(coord) {
        t.terrain = BuiltinTerrain::Ocean;
        t.owner = Some(s.rome_id);
    }

    let rules = DefaultRulesEngine;
    let result = rules.place_improvement(
        &mut s.state,
        s.rome_id,
        coord,
        BuiltinImprovement::Farm,
    );

    assert!(
        matches!(result, Err(libciv::game::RulesError::InvalidImprovement)),
        "Farm on Ocean should return InvalidImprovement"
    );
}

/// LumberMill requires a Forest feature; without it the placement is rejected.
/// With Forest it is still rejected because LumberMill uses the unreachable tech sentinel.
#[test]
fn lumbermill_without_forest_fails_and_with_forest_also_fails_tech() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    // Claim the tile so the ownership check passes.
    claim_tile(&mut s.state, coord, s.rome_id);

    let rules = DefaultRulesEngine;

    // Without Forest: fails on feature check.
    let result_no_forest = rules.place_improvement(
        &mut s.state,
        s.rome_id,
        coord,
        BuiltinImprovement::LumberMill,
    );
    assert!(
        matches!(result_no_forest, Err(libciv::game::RulesError::InvalidImprovement)),
        "LumberMill without Forest should be rejected (InvalidImprovement)"
    );

    // Add Forest — now fails on tech (unreachable sentinel).
    if let Some(t) = s.state.board.tile_mut(coord) {
        t.feature = Some(BuiltinFeature::Forest);
    }

    let result_with_forest = rules.place_improvement(
        &mut s.state,
        s.rome_id,
        coord,
        BuiltinImprovement::LumberMill,
    );
    assert!(
        matches!(result_with_forest, Err(libciv::game::RulesError::TechRequired)),
        "LumberMill on Forest should fail TechRequired (unreachable sentinel)"
    );
}

// ---------------------------------------------------------------------------
// New tests
// ---------------------------------------------------------------------------

/// Farm is blocked when the civ has not yet researched Pottery.
#[test]
fn farm_blocked_without_pottery() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    // Claim the tile so the ownership check passes; only tech should block.
    claim_tile(&mut s.state, coord, s.rome_id);

    // Confirm the tile is flat Grassland — valid terrain, only tech blocks.
    assert!(s.state.board.tile(coord).is_some());

    let rules = DefaultRulesEngine;
    let result = rules.place_improvement(
        &mut s.state,
        s.rome_id,
        coord,
        BuiltinImprovement::Farm,
    );

    assert!(
        matches!(result, Err(libciv::game::RulesError::TechRequired)),
        "Farm without Pottery should return TechRequired, got: {result:?}"
    );
}

/// Farm succeeds once the civ has researched Pottery.
#[test]
fn farm_succeeds_after_pottery_researched() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    claim_tile(&mut s.state, coord, s.rome_id);

    let pottery_id = s.state.tech_refs.pottery;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(pottery_id);

    let rules = DefaultRulesEngine;
    let result = rules.place_improvement(
        &mut s.state,
        s.rome_id,
        coord,
        BuiltinImprovement::Farm,
    );

    assert!(result.is_ok(), "Farm after Pottery should succeed: {result:?}");
    assert!(
        result.unwrap().deltas.iter().any(|d| matches!(d, StateDelta::ImprovementPlaced { .. })),
        "Diff should contain ImprovementPlaced"
    );
}

/// Mine is blocked when the civ has not yet researched Mining.
#[test]
fn mine_blocked_without_mining() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    // Make the tile hills and claim it for Rome.
    if let Some(t) = s.state.board.tile_mut(coord) {
        t.hills = true;
        t.owner = Some(s.rome_id);
    }

    let rules = DefaultRulesEngine;
    let result = rules.place_improvement(
        &mut s.state,
        s.rome_id,
        coord,
        BuiltinImprovement::Mine,
    );

    assert!(
        matches!(result, Err(libciv::game::RulesError::TechRequired)),
        "Mine without Mining should return TechRequired, got: {result:?}"
    );
}

/// LumberMill on a Forest tile returns TechRequired (uses unreachable tech sentinel).
#[test]
fn lumbermill_blocked_by_unreachable_tech() {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(2, 2);

    if let Some(t) = s.state.board.tile_mut(coord) {
        t.feature = Some(BuiltinFeature::Forest);
        t.owner = Some(s.rome_id);
    }

    let rules = DefaultRulesEngine;
    let result = rules.place_improvement(
        &mut s.state,
        s.rome_id,
        coord,
        BuiltinImprovement::LumberMill,
    );

    assert!(
        matches!(result, Err(libciv::game::RulesError::TechRequired)),
        "LumberMill should return TechRequired (unreachable sentinel), got: {result:?}"
    );
}
