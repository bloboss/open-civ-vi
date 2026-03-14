/// Integration tests for district placement validation.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::civ::district::BuiltinDistrict;
use libciv::game::{RulesError, StateDelta};
use libciv::world::terrain::BuiltinTerrain;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Claim a tile for a civilization by setting tile.owner.
fn claim_tile(state: &mut libciv::GameState, coord: HexCoord, civ_id: libciv::CivId) {
    if let Some(t) = state.board.tile_mut(coord) {
        t.owner = Some(civ_id);
    }
}

// ---------------------------------------------------------------------------
// Basic placement
// ---------------------------------------------------------------------------

/// Campus placement succeeds when tech + territory conditions are met.
#[test]
fn campus_on_owned_land_succeeds() {
    let mut s = common::build_scenario();
    let city_coord = HexCoord::from_qr(3, 3); // Rome city
    // Place district 1 tile away from city.
    let target = HexCoord::from_qr(4, 3);

    // Claim the tile for Rome.
    claim_tile(&mut s.state, target, s.rome_id);

    // Grant Writing so the tech check passes.
    let writing_id = s.state.tech_refs.writing;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(writing_id);

    // Ensure target is land (Grassland by default in our scenario).
    assert!(s.state.board.tile(target).is_some());

    let rules = DefaultRulesEngine;
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target);

    assert!(result.is_ok(), "Campus should succeed: {result:?}");

    let diff = result.unwrap();
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::DistrictBuilt { .. })),
        "Diff should contain DistrictBuilt"
    );

    // City's district list updated.
    let city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert!(city.districts.contains(&BuiltinDistrict::Campus));

    // placed_districts updated.
    assert!(s.state.placed_districts.iter().any(|d| d.coord == target));

    assert_eq!(city_coord.distance(&target), 1);
}

/// Campus blocked without Writing tech.
#[test]
fn campus_blocked_without_writing() {
    let mut s = common::build_scenario();
    let target = HexCoord::from_qr(4, 3);
    claim_tile(&mut s.state, target, s.rome_id);

    let rules = DefaultRulesEngine;
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target);

    assert!(
        matches!(result, Err(RulesError::TechRequired)),
        "Campus without Writing should return TechRequired, got: {result:?}"
    );
}

/// District cannot be placed on unowned tile.
#[test]
fn district_on_unowned_tile_fails() {
    let mut s = common::build_scenario();
    let target = HexCoord::from_qr(4, 3);
    // Do NOT claim the tile for Rome — it stays owner=None.

    let writing_id = s.state.tech_refs.writing;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(writing_id);

    let rules = DefaultRulesEngine;
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target);

    assert!(
        matches!(result, Err(RulesError::TileNotOwned)),
        "District on unowned tile should return TileNotOwned, got: {result:?}"
    );
}

/// District cannot be placed more than 3 tiles from the city center.
#[test]
fn district_too_far_from_city_fails() {
    let mut s = common::build_scenario();
    let target = HexCoord::from_qr(7, 3); // 4 tiles from (3,3)
    claim_tile(&mut s.state, target, s.rome_id);

    let writing_id = s.state.tech_refs.writing;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(writing_id);

    let rules = DefaultRulesEngine;
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target);

    assert!(
        matches!(result, Err(RulesError::TileNotInCityRange)),
        "District 4+ tiles away should return TileNotInCityRange, got: {result:?}"
    );
}

/// District cannot be placed on the city center tile (distance 0).
#[test]
fn district_on_city_center_fails() {
    let mut s = common::build_scenario();
    let city_coord = HexCoord::from_qr(3, 3);
    claim_tile(&mut s.state, city_coord, s.rome_id);

    let writing_id = s.state.tech_refs.writing;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(writing_id);

    let rules = DefaultRulesEngine;
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, city_coord);

    assert!(
        matches!(result, Err(RulesError::TileNotInCityRange)),
        "District on city center (dist=0) should return TileNotInCityRange, got: {result:?}"
    );
}

/// Duplicate district type is rejected.
#[test]
fn duplicate_district_fails() {
    let mut s = common::build_scenario();
    let target1 = HexCoord::from_qr(4, 3);
    let target2 = HexCoord::from_qr(3, 4);
    claim_tile(&mut s.state, target1, s.rome_id);
    claim_tile(&mut s.state, target2, s.rome_id);

    let writing_id = s.state.tech_refs.writing;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(writing_id);

    let rules = DefaultRulesEngine;
    rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target1)
        .expect("First Campus should succeed");

    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target2);
    assert!(
        matches!(result, Err(RulesError::DistrictAlreadyPresent)),
        "Second Campus should return DistrictAlreadyPresent, got: {result:?}"
    );
}

/// Two districts of different types can coexist in the same city.
#[test]
fn two_different_districts_succeed() {
    let mut s = common::build_scenario();
    let target1 = HexCoord::from_qr(4, 3);
    let target2 = HexCoord::from_qr(3, 4);
    claim_tile(&mut s.state, target1, s.rome_id);
    claim_tile(&mut s.state, target2, s.rome_id);

    let writing_id    = s.state.tech_refs.writing;
    let bronze_id     = s.state.tech_refs.bronze_working;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.extend([writing_id, bronze_id]);

    let rules = DefaultRulesEngine;
    rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target1)
        .expect("Campus should succeed");
    rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Encampment, target2)
        .expect("Encampment should succeed");

    let city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert!(city.districts.contains(&BuiltinDistrict::Campus));
    assert!(city.districts.contains(&BuiltinDistrict::Encampment));
}

/// District is rejected when target tile is already occupied by a different district.
#[test]
fn district_on_occupied_tile_fails() {
    let mut s = common::build_scenario();
    let target = HexCoord::from_qr(4, 3);
    claim_tile(&mut s.state, target, s.rome_id);

    let writing_id = s.state.tech_refs.writing;
    let bronze_id  = s.state.tech_refs.bronze_working;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.extend([writing_id, bronze_id]);

    let rules = DefaultRulesEngine;
    rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target)
        .expect("Campus should succeed");

    // Try to place Encampment on the same tile.
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Encampment, target);
    assert!(
        matches!(result, Err(RulesError::TileOccupiedByDistrict)),
        "Second district on same tile should return TileOccupiedByDistrict, got: {result:?}"
    );
}

/// Harbor requires a Coast tile; placing it on Grassland fails.
#[test]
fn harbor_on_land_fails() {
    let mut s = common::build_scenario();
    let target = HexCoord::from_qr(4, 3);
    claim_tile(&mut s.state, target, s.rome_id);
    // Leave terrain as Grassland (land).

    let sailing_id = s.state.tech_refs.sailing;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(sailing_id);

    let rules = DefaultRulesEngine;
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Harbor, target);

    assert!(
        matches!(result, Err(RulesError::InvalidDistrict)),
        "Harbor on land should return InvalidDistrict, got: {result:?}"
    );
}

/// Harbor succeeds on a Coast tile with Sailing.
#[test]
fn harbor_on_coast_succeeds() {
    let mut s = common::build_scenario();
    let target = HexCoord::from_qr(4, 3);
    if let Some(t) = s.state.board.tile_mut(target) {
        t.terrain = BuiltinTerrain::Coast;
        t.owner = Some(s.rome_id);
    }

    let sailing_id = s.state.tech_refs.sailing;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(sailing_id);

    let rules = DefaultRulesEngine;
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Harbor, target);

    assert!(result.is_ok(), "Harbor on Coast should succeed: {result:?}");
}

/// Campus cannot be placed on a Mountain tile.
#[test]
fn campus_on_mountain_fails() {
    let mut s = common::build_scenario();
    let target = HexCoord::from_qr(4, 3);
    if let Some(t) = s.state.board.tile_mut(target) {
        t.terrain = BuiltinTerrain::Mountain;
        t.owner = Some(s.rome_id);
    }

    let writing_id = s.state.tech_refs.writing;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .researched_techs.push(writing_id);

    let rules = DefaultRulesEngine;
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, target);

    assert!(
        matches!(result, Err(RulesError::InvalidDistrict)),
        "Campus on Mountain should return InvalidDistrict, got: {result:?}"
    );
}

/// Theater Square requires the Craftsmanship civic (no tech).
#[test]
fn theater_square_requires_craftsmanship_civic() {
    let mut s = common::build_scenario();
    let target = HexCoord::from_qr(4, 3);
    claim_tile(&mut s.state, target, s.rome_id);

    let rules = DefaultRulesEngine;

    // Without the civic — should fail.
    let result = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::TheaterSquare, target);
    assert!(
        matches!(result, Err(RulesError::CivicRequired)),
        "TheaterSquare without civic should return CivicRequired, got: {result:?}"
    );

    // Grant the civic.
    let civic_id = s.state.civic_refs.craftsmanship;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .completed_civics.push(civic_id);

    let result2 = rules.place_district(&mut s.state, s.rome_city, BuiltinDistrict::TheaterSquare, target);
    assert!(result2.is_ok(), "TheaterSquare with civic should succeed: {result2:?}");
}
