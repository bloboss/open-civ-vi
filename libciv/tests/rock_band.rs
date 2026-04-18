/// Integration tests for Rock Band cultural combat (GS-16).
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::game::{RulesError, StateDelta};
use libciv::{UnitCategory, UnitDomain, UnitTypeId};
use libciv::game::state::UnitTypeDef;
use libciv::civ::BasicUnit;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Add a Rock Band unit at `coord` owned by `owner` and return its UnitId.
fn spawn_rock_band(s: &mut common::Scenario, owner: libciv::CivId, coord: HexCoord) -> libciv::UnitId {
    let rock_band_type = UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id: rock_band_type,
        name: "Rock Band",
        production_cost: 600,
        domain: UnitDomain::Land,
        category: UnitCategory::Civilian,
        max_movement: 400,
        combat_strength: None,
        range: 0,
        vision_range: 2,
        can_found_city: false,
        resource_cost: None,
        siege_bonus: 0,
        max_charges: 1,
        exclusive_to: None,
        replaces: None,
        era: Some(libciv::AgeType::Modern),
        promotion_class: None,
    });
    let unit_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: unit_id,
        unit_type: rock_band_type,
        owner,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Civilian,
        movement_left: 400,
        max_movement: 400,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: Some(1),
        trade_origin: None,
        trade_destination: None,
        religion_id: None,
        spread_charges: None,
        religious_strength: None, is_embarked: false,
    });
    unit_id
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A Rock Band on a foreign city tile should successfully perform and emit
/// a RockBandPerformed delta with tourism > 0.
#[test]
fn rock_band_performs_at_foreign_city() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a Rome Rock Band on Babylon's city tile.
    let babylon_coord = s.state.cities.iter()
        .find(|c| c.id == s.babylon_city).unwrap().coord;
    let rome_id = s.rome_id;
    let babylon_city = s.babylon_city;
    let band = spawn_rock_band(&mut s, rome_id, babylon_coord);

    let diff = rules.rock_band_perform(&mut s.state, band).unwrap();

    // Should have RockBandPerformed with tourism >= 500.
    let performed = diff.deltas.iter().find(|d| matches!(d, StateDelta::RockBandPerformed { .. }));
    assert!(performed.is_some(), "expected RockBandPerformed delta");
    if let Some(StateDelta::RockBandPerformed { tourism_gained, city, .. }) = performed {
        assert!(*tourism_gained >= 500, "tourism should be at least 500, got {tourism_gained}");
        assert_eq!(*city, babylon_city);
    }
}

/// A Rock Band on its own city should fail with NotOnForeignCity.
#[test]
fn rock_band_fails_on_own_city() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a Rome Rock Band on Rome's own city tile.
    let rome_coord = s.state.cities.iter()
        .find(|c| c.id == s.rome_city).unwrap().coord;
    let rome_id = s.rome_id;
    let band = spawn_rock_band(&mut s, rome_id, rome_coord);

    let result = rules.rock_band_perform(&mut s.state, band);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RulesError::NotOnForeignCity));
}

/// A Rock Band should be destroyed after performing (either by disband roll or
/// charges reaching 0, since it has 1 charge). Run the test and verify the
/// unit no longer exists after a successful perform.
#[test]
fn rock_band_destroyed_after_performance() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let babylon_coord = s.state.cities.iter()
        .find(|c| c.id == s.babylon_city).unwrap().coord;
    let rome_id = s.rome_id;
    let band = spawn_rock_band(&mut s, rome_id, babylon_coord);

    let diff = rules.rock_band_perform(&mut s.state, band).unwrap();

    // With 1 charge, the band should always be destroyed:
    // either the 30% disband roll triggers, or the charge reaches 0.
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitDestroyed { unit } if *unit == band)),
        "Rock Band should be destroyed after performing with 1 charge"
    );
    assert!(
        !s.state.units.iter().any(|u| u.id == band),
        "Rock Band unit should be removed from state"
    );
}

/// A non-Rock-Band unit should fail with NotARockBand.
#[test]
fn non_rock_band_fails() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // The Rome warrior is not a Rock Band.
    let result = rules.rock_band_perform(&mut s.state, s.rome_warrior);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RulesError::NotARockBand));
}
