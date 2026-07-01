mod common;

use libciv::CivId;
use libciv::game::diff::StateDelta;
use libciv::game::recalculate_visibility;
use libciv::game::state::BuildingDef;
use libciv::{BuildingId, DefaultRulesEngine, TurnEngine, YieldBundle};

use common::build_scenario;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Advance one turn and return the diff (unlike common::advance_turn which discards it).
fn advance_turn_with_diff(s: &mut common::Scenario) -> libciv::GameStateDiff {
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;
    let diff = engine.process_turn(&mut s.state, &rules);
    for unit in &mut s.state.units {
        unit.movement_left = unit.max_movement;
    }
    let civ_ids: Vec<CivId> = s.state.civilizations.iter().map(|c| c.id).collect();
    for cid in civ_ids {
        recalculate_visibility(&mut s.state, cid);
    }
    diff
}

/// Register a building def with the given power fields and add it to a city.
fn add_power_building(
    s: &mut common::Scenario,
    city_idx: usize,
    name: &'static str,
    power_cost: u32,
    power_generated: u32,
    co2_per_turn: u32,
) -> BuildingId {
    let id = BuildingId::from_ulid(s.state.id_gen.next_ulid());
    s.state.building_defs.push(BuildingDef {
        id,
        name,
        cost: 580,
        maintenance: 3,
        yields: YieldBundle::default(),
        requires_district: None,
        prereq_building: None,
        mutually_exclusive: None,
        great_work_slots: vec![],
        exclusive_to: None,
        replaces: None,
        power_cost,
        power_generated,
        co2_per_turn,
    });
    s.state.cities[city_idx].buildings.push(id);
    id
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn co2_accumulates_from_coal_plant() {
    let mut s = build_scenario();
    assert_eq!(s.state.global_co2, 0);

    // Find Rome's city index.
    let rome_city_idx = s
        .state
        .cities
        .iter()
        .position(|c| c.id == s.rome_city)
        .unwrap();

    // Add a Coal Power Plant to Rome's city (power_generated=4, co2_per_turn=1).
    add_power_building(&mut s, rome_city_idx, "Coal Power Plant", 0, 4, 1);

    // Advance one turn.
    let diff = advance_turn_with_diff(&mut s);

    // CO2 should have increased by 1.
    assert_eq!(
        s.state.global_co2, 1,
        "global_co2 should be 1 after one turn with coal plant"
    );

    // Check the diff for CO2Accumulated delta.
    let co2_delta = diff
        .deltas
        .iter()
        .find(|d| matches!(d, StateDelta::CO2Accumulated { .. }));
    assert!(
        co2_delta.is_some(),
        "diff should contain CO2Accumulated delta"
    );
    if let Some(StateDelta::CO2Accumulated { total }) = co2_delta {
        assert_eq!(*total, 1);
    }

    // Advance another turn: CO2 should accumulate.
    let _diff2 = advance_turn_with_diff(&mut s);
    assert_eq!(
        s.state.global_co2, 2,
        "global_co2 should be 2 after two turns"
    );
}

#[test]
fn power_balance_computed() {
    let mut s = build_scenario();

    let rome_city_idx = s
        .state
        .cities
        .iter()
        .position(|c| c.id == s.rome_city)
        .unwrap();

    // Add a Factory (power_cost=1) and a Coal Power Plant (power_generated=4).
    add_power_building(&mut s, rome_city_idx, "Factory", 1, 0, 0);
    add_power_building(&mut s, rome_city_idx, "Coal Power Plant", 0, 4, 1);

    // Advance a turn to trigger the power balance recomputation.
    advance_turn_with_diff(&mut s);

    let city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert_eq!(
        city.power_consumed, 1,
        "city should consume 1 power from Factory"
    );
    assert_eq!(
        city.power_generated, 4,
        "city should generate 4 power from Coal Power Plant"
    );
}

#[test]
fn no_co2_from_nuclear_plant() {
    let mut s = build_scenario();

    let rome_city_idx = s
        .state
        .cities
        .iter()
        .position(|c| c.id == s.rome_city)
        .unwrap();

    // Add a Nuclear Power Plant (power_generated=16, co2_per_turn=0).
    add_power_building(&mut s, rome_city_idx, "Nuclear Power Plant", 0, 16, 0);

    advance_turn_with_diff(&mut s);

    assert_eq!(s.state.global_co2, 0, "nuclear plant should not emit CO2");

    let city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert_eq!(
        city.power_generated, 16,
        "nuclear plant should generate 16 power"
    );
}

#[test]
fn multiple_cities_accumulate_co2() {
    let mut s = build_scenario();

    let rome_city_idx = s
        .state
        .cities
        .iter()
        .position(|c| c.id == s.rome_city)
        .unwrap();
    let babylon_city_idx = s
        .state
        .cities
        .iter()
        .position(|c| c.id == s.babylon_city)
        .unwrap();

    // Coal plant in Rome and Oil plant in Babylon.
    add_power_building(&mut s, rome_city_idx, "Coal Power Plant", 0, 4, 1);
    add_power_building(&mut s, babylon_city_idx, "Oil Power Plant", 0, 4, 1);

    advance_turn_with_diff(&mut s);

    assert_eq!(
        s.state.global_co2, 2,
        "both fossil plants should contribute to global CO2"
    );
}

/// Add a *builtin* building (looked up by name from the already-registered
/// `building_defs`) to a city. Exercises the real `co2_per_turn` data assigned
/// in `rules::building_defs`, rather than a synthetic def.
fn add_builtin_building(s: &mut common::Scenario, city_idx: usize, name: &str) {
    let id = s
        .state
        .building_defs
        .iter()
        .find(|d| d.name == name)
        .unwrap_or_else(|| panic!("builtin building {name:?} not found"))
        .id;
    s.state.cities[city_idx].buildings.push(id);
}

#[test]
fn factory_emits_co2_clean_building_does_not() {
    let mut s = build_scenario();
    assert_eq!(s.state.global_co2, 0);

    let rome_idx = s
        .state
        .cities
        .iter()
        .position(|c| c.id == s.rome_city)
        .unwrap();

    // A Factory now emits co2_per_turn = 1 (heavy industry).
    add_builtin_building(&mut s, rome_idx, "Factory");
    advance_turn_with_diff(&mut s);
    assert_eq!(
        s.state.global_co2, 1,
        "a city with a Factory should add 1 CO2 per turn"
    );

    // A clean building (Library) adds nothing on top.
    add_builtin_building(&mut s, rome_idx, "Library");
    advance_turn_with_diff(&mut s);
    assert_eq!(
        s.state.global_co2, 2,
        "a clean building adds 0; only the Factory keeps emitting (1 -> 2)"
    );
}

#[test]
fn fossil_power_plants_emit_per_assigned_scale() {
    let mut s = build_scenario();

    let rome_idx = s
        .state
        .cities
        .iter()
        .position(|c| c.id == s.rome_city)
        .unwrap();

    // Coal (3) + Oil (2) + Power Plant (2) + Nuclear (0, clean) = 7 per turn.
    add_builtin_building(&mut s, rome_idx, "Coal Power Plant");
    add_builtin_building(&mut s, rome_idx, "Oil Power Plant");
    add_builtin_building(&mut s, rome_idx, "Power Plant");
    add_builtin_building(&mut s, rome_idx, "Nuclear Power Plant");

    advance_turn_with_diff(&mut s);
    assert_eq!(
        s.state.global_co2, 7,
        "emitters should sum per assigned scale; nuclear (clean) adds 0"
    );
}
