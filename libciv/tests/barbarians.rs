//! Integration tests for the barbarian camps and barbarian clans system.

mod common;

use libciv::{
    BarbarianCampId, CivId, DefaultRulesEngine, GameState, GameStateDiff,
    RulesEngine, TurnEngine, UnitCategory, UnitDomain, UnitId, UnitTypeId,
};
use libciv::civ::{BasicUnit, City, CityKind, Civilization};
use libciv::civ::barbarian::{BarbarianCamp, BarbarianConfig, ClanType, ScoutState};
use libciv::game::{recalculate_visibility, StateDelta};
use libciv::game::state::UnitTypeDef;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn enable_barbarians(state: &mut GameState) {
    state.barbarian_config.enabled = true;
    state.barbarian_config.spawn_interval = 1; // spawn every turn for testing
    state.barbarian_config.min_distance_from_city = 3;
    state.barbarian_config.min_distance_between_camps = 3;
    state.barbarian_config.max_camps = 4;
    state.barbarian_config.unit_generation_interval = 1;
}

fn enable_clans_mode(state: &mut GameState) {
    enable_barbarians(state);
    state.barbarian_config.clans_mode = true;
    state.barbarian_config.hire_cost = 50;
    state.barbarian_config.hire_cooldown = 5;
    state.barbarian_config.bribe_cost = 40;
    state.barbarian_config.bribe_duration = 10;
    state.barbarian_config.incite_cost = 30;
    state.barbarian_config.incite_duration = 8;
    state.barbarian_config.conversion_threshold = 20;
    state.barbarian_config.conversion_rate = 5;
    state.barbarian_config.bribe_conversion_bonus = 3;
    state.barbarian_config.incite_conversion_penalty = 2;
}

fn register_scout_type(state: &mut GameState) -> UnitTypeId {
    let id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.push(UnitTypeDef {
        id,
        name: "Scout",
        production_cost: 30,
        domain: UnitDomain::Land,
        category: UnitCategory::Combat,
        max_movement: 300,
        combat_strength: Some(10),
        range: 0,
        vision_range: 3,
        can_found_city: false,
        resource_cost: None,
        siege_bonus: 0,
        max_charges: 0,
        exclusive_to: None,
        replaces: None,
    });
    id
}

/// Place a barbarian camp manually for testing (bypasses spawn logic).
fn place_camp(state: &mut GameState, coord: HexCoord, clan_type: Option<ClanType>) -> BarbarianCampId {
    let barb_civ = *state.barbarian_civ.get_or_insert_with(|| state.id_gen.next_civ_id());
    let camp_id = state.id_gen.next_barbarian_camp_id();
    state.barbarian_camps.push(BarbarianCamp::new(
        camp_id, coord, barb_civ, state.turn, clan_type,
    ));
    camp_id
}

fn advance_turn(state: &mut GameState) -> GameStateDiff {
    let rules = DefaultRulesEngine;
    let diff = rules.advance_turn(state);
    for unit in &mut state.units {
        unit.movement_left = unit.max_movement;
    }
    let civ_ids: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();
    for cid in civ_ids {
        recalculate_visibility(state, cid);
    }
    diff
}

// ---------------------------------------------------------------------------
// Tests: Base barbarian mechanics
// ---------------------------------------------------------------------------

#[test]
fn barbarian_camp_spawns_on_turn() {
    let mut s = common::build_scenario();
    enable_barbarians(&mut s.state);
    register_scout_type(&mut s.state);

    // Advance a turn — barbarian processing should attempt to spawn a camp.
    let diff = advance_turn(&mut s.state);

    // At least one camp should have been spawned.
    assert!(
        !s.state.barbarian_camps.is_empty(),
        "expected at least one barbarian camp to spawn"
    );

    // Verify the diff records the spawn.
    let camp_spawned = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianCampSpawned { .. }));
    assert!(camp_spawned, "expected BarbarianCampSpawned delta");
}

#[test]
fn barbarian_disabled_no_camps() {
    let mut s = common::build_scenario();
    s.state.barbarian_config.enabled = false;

    let diff = advance_turn(&mut s.state);

    assert!(s.state.barbarian_camps.is_empty());
    let has_barb = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianCampSpawned { .. }));
    assert!(!has_barb);
}

#[test]
fn barbarian_scout_spawns_from_camp() {
    let mut s = common::build_scenario();
    enable_barbarians(&mut s.state);
    register_scout_type(&mut s.state);

    // Manually place a camp far from cities.
    let camp_coord = HexCoord::from_qr(0, 0);
    let camp_id = place_camp(&mut s.state, camp_coord, None);

    // Advance turn — scout should spawn.
    let diff = advance_turn(&mut s.state);

    let camp = s.state.barbarian_camp(camp_id).expect("camp should exist");
    assert!(
        matches!(&camp.scout_state, ScoutState::Exploring { .. }),
        "expected scout to be exploring, got {:?}", camp.scout_state
    );

    let scout_spawned = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianScoutSpawned { camp, .. } if *camp == camp_id));
    assert!(scout_spawned, "expected BarbarianScoutSpawned delta");
}

#[test]
fn barbarian_scout_discovers_player_and_returns() {
    let mut s = common::build_scenario();
    enable_barbarians(&mut s.state);
    let scout_type = register_scout_type(&mut s.state);

    // Place camp near Rome's warrior at (5,3), so the scout will discover Rome quickly.
    let camp_coord = HexCoord::from_qr(7, 3);
    let barb_civ = *s.state.barbarian_civ.get_or_insert_with(|| s.state.id_gen.next_civ_id());
    let camp_id = s.state.id_gen.next_barbarian_camp_id();

    // Spawn scout manually right next to Rome's warrior.
    let scout_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: scout_id, unit_type: scout_type, owner: barb_civ,
        coord: HexCoord::from_qr(6, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 300, max_movement: 300,
        combat_strength: Some(10), promotions: Vec::new(),
        health: 100, range: 0, vision_range: 3, charges: None,
        trade_origin: None, trade_destination: None, religion_id: None,
        spread_charges: None, religious_strength: None,
    });

    let mut camp = BarbarianCamp::new(camp_id, camp_coord, barb_civ, 0, None);
    camp.scout_state = ScoutState::Exploring { scout_id };
    s.state.barbarian_camps.push(camp);

    // Advance — scout is within vision range of Rome's warrior at (5,3).
    let diff = advance_turn(&mut s.state);

    let camp = s.state.barbarian_camp(camp_id).expect("camp should exist");
    let discovered = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianScoutDiscovered { .. }));

    // Scout should have discovered Rome (within 3 tiles of Rome's warrior).
    assert!(
        matches!(&camp.scout_state, ScoutState::Returning { .. }) || discovered,
        "expected scout to discover Rome and start returning"
    );
}

#[test]
fn barbarian_generates_units_after_scout_returns() {
    let mut s = common::build_scenario();
    enable_barbarians(&mut s.state);
    register_scout_type(&mut s.state);

    // Place camp and manually set scout_state to Returned.
    let camp_coord = HexCoord::from_qr(0, 0);
    let barb_civ = *s.state.barbarian_civ.get_or_insert_with(|| s.state.id_gen.next_civ_id());
    let camp_id = s.state.id_gen.next_barbarian_camp_id();
    let mut camp = BarbarianCamp::new(camp_id, camp_coord, barb_civ, 0, None);
    camp.scout_state = ScoutState::Returned { discovered_civs: vec![s.rome_id] };
    s.state.barbarian_camps.push(camp);

    // Advance turn — should generate a combat unit.
    let diff = advance_turn(&mut s.state);

    let unit_generated = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianUnitGenerated { .. }));
    assert!(unit_generated, "expected barbarian unit to be generated");

    // Verify unit exists in state.
    let barb_units: Vec<&BasicUnit> = s.state.units.iter()
        .filter(|u| u.owner == barb_civ && u.combat_strength.is_some())
        .collect();
    assert!(!barb_units.is_empty(), "expected barbarian combat unit in game state");
}

// ---------------------------------------------------------------------------
// Tests: Barbarian Clans mode
// ---------------------------------------------------------------------------

#[test]
fn clans_hire_unit_from_camp() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    // Give Rome gold.
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 200;

    // Place a clan camp.
    let camp_coord = HexCoord::from_qr(0, 0);
    let camp_id = place_camp(&mut s.state, camp_coord, Some(ClanType::Melee));

    let rules = DefaultRulesEngine;
    let diff = rules.hire_from_barbarian_camp(&mut s.state, camp_id, s.rome_id)
        .expect("hire should succeed");

    // Rome should have spent gold.
    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(rome.gold, 150, "expected 50 gold deducted");

    // A new unit owned by Rome should exist near the camp.
    let hired_unit = diff.deltas.iter().find(|d| matches!(d, StateDelta::UnitCreated { owner, .. } if *owner == s.rome_id));
    assert!(hired_unit.is_some(), "expected UnitCreated delta for Rome");

    // Hire delta should be recorded.
    let hire_delta = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianClanHired { .. }));
    assert!(hire_delta, "expected BarbarianClanHired delta");
}

#[test]
fn clans_hire_cooldown() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 500;

    let camp_coord = HexCoord::from_qr(0, 0);
    let camp_id = place_camp(&mut s.state, camp_coord, Some(ClanType::Melee));

    let rules = DefaultRulesEngine;
    rules.hire_from_barbarian_camp(&mut s.state, camp_id, s.rome_id).unwrap();

    // Second hire should fail (cooldown).
    let result = rules.hire_from_barbarian_camp(&mut s.state, camp_id, s.rome_id);
    assert!(result.is_err(), "expected hire to fail on cooldown");
}

#[test]
fn clans_bribe_camp() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 200;

    let camp_coord = HexCoord::from_qr(0, 0);
    let camp_id = place_camp(&mut s.state, camp_coord, Some(ClanType::Ranged));

    let rules = DefaultRulesEngine;
    let diff = rules.bribe_barbarian_camp(&mut s.state, camp_id, s.rome_id)
        .expect("bribe should succeed");

    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(rome.gold, 160, "expected 40 gold deducted");

    let camp = s.state.barbarian_camp(camp_id).unwrap();
    assert!(camp.is_bribed_by(s.rome_id, s.state.turn), "camp should be bribed");

    let bribe_delta = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianClanBribed { .. }));
    assert!(bribe_delta, "expected BarbarianClanBribed delta");
}

#[test]
fn clans_incite_camp() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 200;

    let camp_coord = HexCoord::from_qr(0, 0);
    let camp_id = place_camp(&mut s.state, camp_coord, Some(ClanType::Cavalry));

    let rules = DefaultRulesEngine;
    let diff = rules.incite_barbarian_camp(&mut s.state, camp_id, s.rome_id, s.babylon_id)
        .expect("incite should succeed");

    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(rome.gold, 170, "expected 30 gold deducted");

    let camp = s.state.barbarian_camp(camp_id).unwrap();
    assert!(camp.is_incited_against(s.babylon_id, s.state.turn), "camp should be incited against Babylon");

    let incite_delta = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianClanIncited { .. }));
    assert!(incite_delta, "expected BarbarianClanIncited delta");
}

#[test]
fn clans_incite_self_fails() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 200;

    let camp_id = place_camp(&mut s.state, HexCoord::from_qr(0, 0), Some(ClanType::Melee));

    let rules = DefaultRulesEngine;
    let result = rules.incite_barbarian_camp(&mut s.state, camp_id, s.rome_id, s.rome_id);
    assert!(result.is_err(), "should not be able to incite against yourself");
}

#[test]
fn clans_insufficient_gold_hire() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    // Rome has no gold.
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 0;

    let camp_id = place_camp(&mut s.state, HexCoord::from_qr(0, 0), Some(ClanType::Melee));

    let rules = DefaultRulesEngine;
    let result = rules.hire_from_barbarian_camp(&mut s.state, camp_id, s.rome_id);
    assert!(result.is_err(), "should fail with insufficient gold");
}

#[test]
fn clans_mode_disabled_hire_fails() {
    let mut s = common::build_scenario();
    enable_barbarians(&mut s.state);
    // clans_mode is NOT enabled
    register_scout_type(&mut s.state);

    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 200;
    let camp_id = place_camp(&mut s.state, HexCoord::from_qr(0, 0), None);

    let rules = DefaultRulesEngine;
    let result = rules.hire_from_barbarian_camp(&mut s.state, camp_id, s.rome_id);
    assert!(result.is_err(), "hire should fail when clans mode is disabled");
}

// ---------------------------------------------------------------------------
// Tests: Camp-to-city-state conversion
// ---------------------------------------------------------------------------

#[test]
fn clans_camp_converts_to_city_state() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    // Place a clan camp far from all cities.
    let camp_coord = HexCoord::from_qr(0, 0);
    let camp_id = place_camp(&mut s.state, camp_coord, Some(ClanType::Melee));

    // Fast-forward conversion by setting progress close to threshold.
    s.state.barbarian_camps.iter_mut()
        .find(|c| c.id == camp_id).unwrap()
        .conversion_progress = 15; // threshold is 20, rate is 5

    // Advance turn — conversion should complete (15 + 5 = 20 >= 20).
    let diff = advance_turn(&mut s.state);

    let converted = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianCampConverted { .. }));
    assert!(converted, "expected BarbarianCampConverted delta");

    // A city-state city should exist at the camp's coord.
    let cs_city = s.state.cities.iter().find(|c| c.coord == camp_coord);
    assert!(cs_city.is_some(), "expected city-state city at camp coord");
    let cs = cs_city.unwrap();
    assert!(matches!(cs.kind, CityKind::CityState(_)), "expected CityKind::CityState");

    // Camp should be marked as converted.
    let camp = s.state.barbarian_camp(camp_id).unwrap();
    assert!(camp.converted);
}

#[test]
fn clans_bribe_accelerates_conversion() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    let camp_coord = HexCoord::from_qr(0, 0);
    let camp_id = place_camp(&mut s.state, camp_coord, Some(ClanType::Ranged));

    // Bribe the camp.
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 200;
    let rules = DefaultRulesEngine;
    rules.bribe_barbarian_camp(&mut s.state, camp_id, s.rome_id).unwrap();

    // Advance turn — conversion_progress should increase by rate(5) + bribe_bonus(3) = 8.
    advance_turn(&mut s.state);

    let camp = s.state.barbarian_camp(camp_id).unwrap();
    assert_eq!(camp.conversion_progress, 8, "expected base(5) + bribe_bonus(3) = 8");
}

#[test]
fn clans_incite_slows_conversion() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    let camp_coord = HexCoord::from_qr(0, 0);
    let camp_id = place_camp(&mut s.state, camp_coord, Some(ClanType::Cavalry));

    // Incite the camp.
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().gold = 200;
    let rules = DefaultRulesEngine;
    rules.incite_barbarian_camp(&mut s.state, camp_id, s.rome_id, s.babylon_id).unwrap();

    // Advance turn — conversion_progress should increase by rate(5) - incite_penalty(2) = 3.
    advance_turn(&mut s.state);

    let camp = s.state.barbarian_camp(camp_id).unwrap();
    assert_eq!(camp.conversion_progress, 3, "expected base(5) - incite_penalty(2) = 3");
}

#[test]
fn clans_conversion_respects_city_placement_rules() {
    let mut s = common::build_scenario();
    enable_clans_mode(&mut s.state);
    register_scout_type(&mut s.state);

    // Place camp very close to Rome's capital at (3,3) — within 3 tiles.
    let camp_coord = HexCoord::from_qr(4, 3);
    let camp_id = place_camp(&mut s.state, camp_coord, Some(ClanType::Melee));

    // Set conversion past threshold.
    s.state.barbarian_camps.iter_mut()
        .find(|c| c.id == camp_id).unwrap()
        .conversion_progress = 100;

    advance_turn(&mut s.state);

    // Conversion should NOT happen because camp is too close to a city.
    let camp = s.state.barbarian_camp(camp_id).unwrap();
    assert!(!camp.converted, "camp should NOT convert when too close to a city");
}

// ---------------------------------------------------------------------------
// Tests: Camp clearing
// ---------------------------------------------------------------------------

#[test]
fn clear_barbarian_camp_removes_units() {
    let mut s = common::build_scenario();
    enable_barbarians(&mut s.state);
    register_scout_type(&mut s.state);

    let camp_coord = HexCoord::from_qr(0, 0);
    let barb_civ = *s.state.barbarian_civ.get_or_insert_with(|| s.state.id_gen.next_civ_id());
    let camp_id = s.state.id_gen.next_barbarian_camp_id();

    // Spawn a barbarian unit belonging to this camp.
    let barb_unit_id = s.state.id_gen.next_unit_id();
    let warrior_type = s.warrior_type;
    s.state.units.push(BasicUnit {
        id: barb_unit_id, unit_type: warrior_type, owner: barb_civ,
        coord: HexCoord::from_qr(1, 0),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(20), promotions: Vec::new(),
        health: 100, range: 0, vision_range: 2, charges: None,
        trade_origin: None, trade_destination: None, religion_id: None,
        spread_charges: None, religious_strength: None,
    });

    let mut camp = BarbarianCamp::new(camp_id, camp_coord, barb_civ, 0, None);
    camp.spawned_units.push(barb_unit_id);
    camp.scout_state = ScoutState::Returned { discovered_civs: vec![s.rome_id] };
    s.state.barbarian_camps.push(camp);

    // Clear the camp.
    let rules = DefaultRulesEngine;
    let diff = rules.clear_barbarian_camp(&mut s.state, camp_id, s.rome_id).unwrap();

    // Camp should be removed.
    assert!(s.state.barbarian_camp(camp_id).is_none(), "camp should be removed");

    // Barbarian unit should be destroyed.
    assert!(s.state.unit(barb_unit_id).is_none(), "barbarian unit should be destroyed");

    let destroyed = diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianCampDestroyed { .. }));
    assert!(destroyed, "expected BarbarianCampDestroyed delta");
}
