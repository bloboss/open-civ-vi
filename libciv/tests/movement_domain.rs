//! Tests for unit domain restrictions on movement.
//!
//! Land units must not be able to move onto water tiles (Coast, Ocean).
//! Sea units must not be able to move onto land tiles.

mod common;

use common::SpawnUnit;
use libciv::game::diff::StateDelta;
use libciv::world::terrain::BuiltinTerrain;
use libciv::{DefaultRulesEngine, RulesEngine, UnitDomain};
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
