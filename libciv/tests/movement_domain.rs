//! Tests for unit domain restrictions on movement and embarkation.
//!
//! Land units must not be able to move onto water tiles (Coast, Ocean)
//! unless embarkation is unlocked for their civilization.

mod common;

use common::SpawnUnit;
use libciv::game::diff::StateDelta;
use libciv::world::terrain::BuiltinTerrain;
use libciv::{apply_diff, DefaultRulesEngine, RulesEngine, UnitDomain};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

#[test]
fn land_unit_cannot_move_onto_ocean() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let ocean_coord = HexCoord::from_qr(6, 3);
    s.state.board.tile_mut(ocean_coord).unwrap().terrain = BuiltinTerrain::Ocean;

    let result = rules.move_unit(&mut s.state, s.rome_warrior, ocean_coord);
    assert!(result.is_err(), "Land unit should not be able to move onto Ocean");
}

#[test]
fn land_unit_cannot_move_onto_coast() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let coast_coord = HexCoord::from_qr(6, 3);
    s.state.board.tile_mut(coast_coord).unwrap().terrain = BuiltinTerrain::Coast;

    let result = rules.move_unit(&mut s.state, s.rome_warrior, coast_coord);
    assert!(result.is_err(), "Land unit should not be able to move onto Coast");
}

#[test]
fn land_unit_cannot_path_through_water() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Water barrier at (6,3) between warrior at (5,3) and destination (7,3).
    s.state.board.tile_mut(HexCoord::from_qr(6, 3)).unwrap().terrain = BuiltinTerrain::Ocean;

    let dest = HexCoord::from_qr(7, 3);
    match rules.move_unit(&mut s.state, s.rome_warrior, dest) {
        Err(_) => {} // No path or insufficient movement — expected.
        Ok(diff) => {
            for delta in &diff.deltas {
                if let StateDelta::UnitMoved { to, .. } = delta {
                    let tile = s.state.board.tile(*to).unwrap();
                    assert!(
                        tile.terrain.is_land(),
                        "Land unit should not pass through water at ({}, {})",
                        to.q, to.r
                    );
                }
            }
        }
    }
}

#[test]
fn land_unit_can_still_move_on_land() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let dest = HexCoord::from_qr(6, 3);
    assert!(s.state.board.tile(dest).unwrap().terrain.is_land());

    let result = rules.move_unit(&mut s.state, s.rome_warrior, dest);
    assert!(result.is_ok(), "Land unit should be able to move on land");
}

#[test]
fn sea_unit_cannot_move_onto_land() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let sea_coord = HexCoord::from_qr(0, 0);
    s.state.board.tile_mut(sea_coord).unwrap().terrain = BuiltinTerrain::Coast;

    let ship_id = SpawnUnit::combat(s.warrior_type, s.rome_id, sea_coord)
        .domain(UnitDomain::Sea)
        .movement(400)
        .build(&mut s.state);

    let land_dest = HexCoord::from_qr(1, 0);
    assert!(s.state.board.tile(land_dest).unwrap().terrain.is_land());

    let result = rules.move_unit(&mut s.state, ship_id, land_dest);
    assert!(result.is_err(), "Sea unit should not be able to move onto land");
}

#[test]
fn sea_unit_can_move_on_water() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let from = HexCoord::from_qr(0, 0);
    let to = HexCoord::from_qr(1, 0);
    s.state.board.tile_mut(from).unwrap().terrain = BuiltinTerrain::Coast;
    s.state.board.tile_mut(to).unwrap().terrain = BuiltinTerrain::Coast;

    let ship_id = SpawnUnit::combat(s.warrior_type, s.rome_id, from)
        .domain(UnitDomain::Sea)
        .movement(400)
        .build(&mut s.state);

    let result = rules.move_unit(&mut s.state, ship_id, to);
    assert!(result.is_ok(), "Sea unit should be able to move on water");
}

#[test]
fn settler_cannot_move_onto_ocean() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let settler_coord = HexCoord::from_qr(4, 3);
    let settler_id = SpawnUnit::civilian(s.settler_type, s.rome_id, settler_coord)
        .build(&mut s.state);

    let ocean_coord = HexCoord::from_qr(5, 4);
    s.state.board.tile_mut(ocean_coord).unwrap().terrain = BuiltinTerrain::Ocean;

    let result = rules.move_unit(&mut s.state, settler_id, ocean_coord);
    assert!(result.is_err(), "Settler should not be able to move onto Ocean");
}

// ── Embarkation tests ──────────────────────────────────────────────────────

/// Helper: enable coast embarkation for a civ.
fn enable_coast_embark(s: &mut common::Scenario) {
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .can_embark_coast = true;
}

/// Helper: enable full (coast + ocean) embarkation for a civ.
fn enable_full_embark(s: &mut common::Scenario) {
    let civ = s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap();
    civ.can_embark_coast = true;
    civ.can_embark_ocean = true;
}

#[test]
fn land_unit_cannot_embark_without_tech() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let coast_coord = HexCoord::from_qr(6, 3);
    s.state.board.tile_mut(coast_coord).unwrap().terrain = BuiltinTerrain::Coast;

    // No embarkation unlocked — should fail.
    let result = rules.move_unit(&mut s.state, s.rome_warrior, coast_coord);
    assert!(result.is_err(), "Should not embark without tech");
}

#[test]
fn land_unit_can_embark_coast_with_tech() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    enable_coast_embark(&mut s);

    let coast_coord = HexCoord::from_qr(6, 3);
    s.state.board.tile_mut(coast_coord).unwrap().terrain = BuiltinTerrain::Coast;

    let result = rules.move_unit(&mut s.state, s.rome_warrior, coast_coord);

    // Should succeed (possibly as partial move with InsufficientMovement carrying a diff).
    let diff = match result {
        Ok(d) => d,
        Err(libciv::game::RulesError::InsufficientMovement(d)) => d,
        Err(e) => panic!("Expected successful embark, got: {e:?}"),
    };

    // Should contain UnitEmbarked delta.
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitEmbarked { .. })),
        "Should emit UnitEmbarked delta"
    );
}

#[test]
fn land_unit_cannot_embark_ocean_with_coast_only() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    enable_coast_embark(&mut s); // coast only, no ocean

    let ocean_coord = HexCoord::from_qr(6, 3);
    s.state.board.tile_mut(ocean_coord).unwrap().terrain = BuiltinTerrain::Ocean;

    let result = rules.move_unit(&mut s.state, s.rome_warrior, ocean_coord);
    assert!(result.is_err(), "Should not embark onto Ocean with coast-only tech");
}

#[test]
fn land_unit_can_embark_ocean_with_full_tech() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    enable_full_embark(&mut s);

    let ocean_coord = HexCoord::from_qr(6, 3);
    s.state.board.tile_mut(ocean_coord).unwrap().terrain = BuiltinTerrain::Ocean;

    let result = rules.move_unit(&mut s.state, s.rome_warrior, ocean_coord);
    let diff = match result {
        Ok(d) => d,
        Err(libciv::game::RulesError::InsufficientMovement(d)) => d,
        Err(e) => panic!("Expected successful ocean embark, got: {e:?}"),
    };

    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitEmbarked { .. })),
        "Should emit UnitEmbarked delta for Ocean"
    );
}

#[test]
fn embarking_costs_all_movement() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    enable_coast_embark(&mut s);

    let coast_coord = HexCoord::from_qr(6, 3);
    s.state.board.tile_mut(coast_coord).unwrap().terrain = BuiltinTerrain::Coast;

    let result = rules.move_unit(&mut s.state, s.rome_warrior, coast_coord);
    let diff = match result {
        Ok(d) => d,
        Err(libciv::game::RulesError::InsufficientMovement(d)) => d,
        Err(e) => panic!("Unexpected error: {e:?}"),
    };

    // The UnitMoved delta should cost the full movement budget (200).
    let moved = diff.deltas.iter().find_map(|d| {
        if let StateDelta::UnitMoved { cost, .. } = d { Some(*cost) } else { None }
    });
    assert_eq!(moved, Some(200), "Embarking should consume all 200 movement points");
}

#[test]
fn disembarking_costs_all_movement() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    enable_coast_embark(&mut s);

    // Place warrior on a Coast tile, already embarked.
    let coast_coord = HexCoord::from_qr(4, 3);
    s.state.board.tile_mut(coast_coord).unwrap().terrain = BuiltinTerrain::Coast;
    let embarked_id = SpawnUnit::combat(s.warrior_type, s.rome_id, coast_coord)
        .build(&mut s.state);
    // Manually set embarked state.
    s.state.units.iter_mut()
        .find(|u| u.id == embarked_id).unwrap()
        .is_embarked = true;

    // Move to adjacent land tile.
    let land_coord = HexCoord::from_qr(4, 2);
    assert!(s.state.board.tile(land_coord).unwrap().terrain.is_land());

    let result = rules.move_unit(&mut s.state, embarked_id, land_coord);
    let diff = match result {
        Ok(d) => d,
        Err(libciv::game::RulesError::InsufficientMovement(d)) => d,
        Err(e) => panic!("Unexpected error: {e:?}"),
    };

    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitDisembarked { .. })),
        "Should emit UnitDisembarked delta"
    );

    let moved = diff.deltas.iter().find_map(|d| {
        if let StateDelta::UnitMoved { cost, .. } = d { Some(*cost) } else { None }
    });
    assert_eq!(moved, Some(200), "Disembarking should consume all movement");
}

#[test]
fn embarked_unit_can_move_on_water() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    enable_coast_embark(&mut s);

    // Set up two adjacent Coast tiles (E direction: same r, q+1).
    let from = HexCoord::from_qr(2, 2);
    let to = HexCoord::from_qr(3, 2);
    s.state.board.tile_mut(from).unwrap().terrain = BuiltinTerrain::Coast;
    s.state.board.tile_mut(to).unwrap().terrain = BuiltinTerrain::Coast;

    let unit_id = SpawnUnit::combat(s.warrior_type, s.rome_id, from)
        .build(&mut s.state);
    s.state.units.iter_mut()
        .find(|u| u.id == unit_id).unwrap()
        .is_embarked = true;

    let result = rules.move_unit(&mut s.state, unit_id, to);
    assert!(result.is_ok(), "Embarked unit should be able to move on water");

    let diff = result.unwrap();
    // Should NOT emit embark/disembark — it's water-to-water.
    assert!(
        !diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitEmbarked { .. } | StateDelta::UnitDisembarked { .. })),
        "Water-to-water move should not emit embark/disembark deltas"
    );
}

#[test]
fn apply_delta_embark_disembark() {
    let mut s = common::build_scenario();

    // Manually construct and apply an embark delta.
    let mut diff = libciv::GameStateDiff::new();
    diff.push(StateDelta::UnitEmbarked {
        unit: s.rome_warrior,
        coord: HexCoord::from_qr(6, 3),
    });
    apply_diff(&mut s.state, &diff);

    let unit = s.state.units.iter().find(|u| u.id == s.rome_warrior).unwrap();
    assert!(unit.is_embarked, "Unit should be embarked after apply_delta");

    // Now disembark.
    let mut diff2 = libciv::GameStateDiff::new();
    diff2.push(StateDelta::UnitDisembarked {
        unit: s.rome_warrior,
        coord: HexCoord::from_qr(5, 3),
    });
    apply_diff(&mut s.state, &diff2);

    let unit = s.state.units.iter().find(|u| u.id == s.rome_warrior).unwrap();
    assert!(!unit.is_embarked, "Unit should not be embarked after disembark delta");
}
