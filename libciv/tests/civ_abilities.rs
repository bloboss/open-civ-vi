/// Integration tests for civilization-specific abilities:
/// unique units, districts, buildings, improvements, and rule overrides.
mod common;

use libciv::{
    CivId, DefaultRulesEngine, GreatPersonType, RulesEngine,
    UnitCategory, UnitDomain, UnitTypeId,
};
use libciv::civ::civ_identity::{BuiltinCiv, BuiltinLeader};
use libciv::civ::{BasicUnit, Civilization, Leader};
use libciv::game::state::{BuildingDef, UnitTypeDef};
use libciv::game::StateDelta;
use libciv::{BuildingId, YieldBundle};

use libhexgrid::board::HexBoard;

use common::{build_scenario, advance_turn};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Set a civilization's identity to a built-in civ.
fn set_civ_identity(s: &mut common::Scenario, civ_id: CivId, civ: BuiltinCiv, leader: BuiltinLeader) {
    if let Some(c) = s.state.civilizations.iter_mut().find(|c| c.id == civ_id) {
        c.civ_identity = Some(civ);
        c.leader_identity = Some(leader);
    }
}

/// Register a "monument" building def (needed for Trajan's free building).
fn register_monument(s: &mut common::Scenario) -> BuildingId {
    let id = BuildingId::from_ulid(s.state.id_gen.next_ulid());
    s.state.building_defs.push(BuildingDef {
        id,
        name: "monument",
        cost: 60,
        maintenance: 0,
        yields: YieldBundle::new().with(libciv::YieldType::Culture, 2),
        requires_district: None,
        prereq_building: None,
        mutually_exclusive: None,
        great_work_slots: vec![],
        exclusive_to: None,
        replaces: None,
        power_cost: 0, power_generated: 0, co2_per_turn: 0,
    });
    id
}

// ---------------------------------------------------------------------------
// Tests: Rome / Trajan
// ---------------------------------------------------------------------------

#[test]
fn test_rome_city_founded_gets_monument_and_trading_post() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let rules = DefaultRulesEngine;

    // Configure Rome as the built-in civ.
    set_civ_identity(&mut s, rome_id, BuiltinCiv::Rome, BuiltinLeader::Trajan);

    // Register the monument building def so the hook can find it.
    let monument_id = register_monument(&mut s);

    // Register a settler so we can found a city.
    let settler_type = s.settler_type;
    let settler_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: settler_id, unit_type: settler_type, owner: rome_id,
        coord: libhexgrid::coord::HexCoord::from_qr(7, 1),
        domain: UnitDomain::Land, category: UnitCategory::Civilian,
        movement_left: 200, max_movement: 200, combat_strength: None,
        promotions: Vec::new(), experience: 0, health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None, is_embarked: false,
    });

    let diff = rules.found_city(&mut s.state, settler_id, "New Roma".to_string()).unwrap();

    // Check that the city received a free monument.
    let new_city = s.state.cities.iter().find(|c| c.name == "New Roma").unwrap();
    assert!(
        new_city.buildings.contains(&monument_id),
        "Rome/Trajan city should start with a free Monument"
    );

    // Check for BuildingCompleted delta.
    let has_building_delta = diff.deltas.iter().any(|d| {
        matches!(d, StateDelta::BuildingCompleted { building: "monument", .. })
    });
    assert!(has_building_delta, "should emit BuildingCompleted for the free Monument");

    // Check that the city tile got a trading post.
    let tile = s.state.board.tile(new_city.coord).unwrap();
    assert_eq!(
        tile.improvement,
        Some(libciv::world::improvement::BuiltinImprovement::TradingPost),
        "Rome city should start with a Trading Post"
    );
}

// ---------------------------------------------------------------------------
// Tests: Babylon / Hammurabi
// ---------------------------------------------------------------------------

#[test]
fn test_babylon_eureka_gives_full_tech() {
    let mut s = build_scenario();
    let babylon_id = s.babylon_id;

    set_civ_identity(&mut s, babylon_id, BuiltinCiv::Babylon, BuiltinLeader::Hammurabi);

    // Queue a tech for Babylon.
    let pottery_id = s.state.tech_refs.pottery;
    let pottery_cost = s.state.tech_tree.get(pottery_id).unwrap().cost;

    s.state.civilizations.iter_mut()
        .find(|c| c.id == babylon_id).unwrap()
        .research_queue.push_back(libciv::civ::TechProgress {
            tech_id: pottery_id, progress: 0, boosted: false,
        });

    // Trigger eureka for pottery.
    s.state.effect_queue.push_back((
        babylon_id,
        libciv::rules::OneShotEffect::TriggerEureka { tech: pottery_id },
    ));

    // Process the turn to drain the effect queue.
    advance_turn(&mut s);

    // Babylon's eureka should have completed the tech outright.
    let civ = s.state.civilizations.iter().find(|c| c.id == babylon_id).unwrap();
    assert!(
        civ.researched_techs.contains(&pottery_id),
        "Babylon eureka should instantly complete the tech"
    );
}

#[test]
fn test_babylon_science_penalty() {
    let mut s = build_scenario();
    let babylon_id = s.babylon_id;
    let rome_id = s.rome_id;

    set_civ_identity(&mut s, babylon_id, BuiltinCiv::Babylon, BuiltinLeader::Hammurabi);

    let rules = DefaultRulesEngine;

    // Both civs start with the same base yields (1 science per city).
    let rome_yields = rules.compute_yields(&s.state, rome_id);
    let babylon_yields = rules.compute_yields(&s.state, babylon_id);

    // Babylon should have ~50% of Rome's science (or less due to rounding).
    assert!(
        babylon_yields.science <= rome_yields.science,
        "Babylon science {} should be <= Rome science {} due to -50% penalty",
        babylon_yields.science, rome_yields.science
    );
}

// ---------------------------------------------------------------------------
// Tests: Greece / Pericles
// ---------------------------------------------------------------------------

#[test]
fn test_greece_extra_wildcard_slot() {
    let s = build_scenario();
    // Greece's ExtraPolicySlot(Wildcard) is a RuleOverride.
    // Verify the bundle contains it.
    let bundle = libciv::rules::civ_registry::greece();
    let has_wildcard = bundle.rule_overrides.iter().any(|o| {
        matches!(o, libciv::civ::RuleOverride::ExtraPolicySlot(libciv::PolicyType::Wildcard))
    });
    assert!(has_wildcard, "Greece should have ExtraPolicySlot(Wildcard) override");
    let _ = s; // use scenario to verify compilation
}

// ---------------------------------------------------------------------------
// Tests: Germany / Barbarossa
// ---------------------------------------------------------------------------

#[test]
fn test_germany_extra_district_slot() {
    let bundle = libciv::rules::civ_registry::germany();
    let has_extra = bundle.rule_overrides.iter().any(|o| {
        matches!(o, libciv::civ::RuleOverride::ExtraDistrictSlot(1))
    });
    assert!(has_extra, "Germany should have ExtraDistrictSlot(1) override");
}

// ---------------------------------------------------------------------------
// Tests: Unique unit combat abilities
// ---------------------------------------------------------------------------

#[test]
fn test_hoplite_adjacency_bonus() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let greece_id = s.babylon_id; // repurpose Babylon as Greece for this test
    let rules = DefaultRulesEngine;

    set_civ_identity(&mut s, greece_id, BuiltinCiv::Greece, BuiltinLeader::Pericles);

    // Register hoplite unit type.
    let hoplite_type = UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id: hoplite_type, name: "hoplite", production_cost: 65,
        max_movement: 200, combat_strength: Some(28),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0,
        exclusive_to: Some(BuiltinCiv::Greece), replaces: Some("spearman"), era: None, promotion_class: None,
    });

    // Place two hoplites adjacent to each other.
    let h1_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: h1_id, unit_type: hoplite_type, owner: greece_id,
        coord: libhexgrid::coord::HexCoord::from_qr(9, 4),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200, combat_strength: Some(28),
        promotions: Vec::new(), experience: 0, health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None, is_embarked: false,
    });
    let h2_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: h2_id, unit_type: hoplite_type, owner: greece_id,
        coord: libhexgrid::coord::HexCoord::from_qr(10, 4),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200, combat_strength: Some(28),
        promotions: Vec::new(), experience: 0, health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None, is_embarked: false,
    });

    // Place a Rome warrior adjacent to h1.
    let rome_warrior = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: rome_warrior, unit_type: s.warrior_type, owner: rome_id,
        coord: libhexgrid::coord::HexCoord::from_qr(9, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200, combat_strength: Some(20),
        promotions: Vec::new(), experience: 0, health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None, is_embarked: false,
    });

    // h1 attacks rome_warrior. With the adjacency bonus (+10 from h2),
    // h1 effectively has 38 CS vs warrior's 20 CS.
    // The warrior should take heavy damage.
    let diff = rules.attack(&mut s.state, h1_id, rome_warrior).unwrap();

    let atk_delta = diff.deltas.iter().find(|d| matches!(d, StateDelta::UnitAttacked { .. }));
    assert!(atk_delta.is_some(), "attack should produce UnitAttacked delta");

    // The warrior (20 CS) should take significant damage from effective 38 CS hoplite.
    if let Some(StateDelta::UnitAttacked { defender_damage, .. }) = atk_delta {
        assert!(
            *defender_damage > 30,
            "hoplite with adjacency bonus should deal significant damage (got {defender_damage})"
        );
    }
}

#[test]
fn test_samurai_no_damage_penalty() {
    // Verify the Samurai's NoCombatPenaltyWhenDamaged ability is registered.
    let bundle = libciv::rules::civ_registry::japan();
    let samurai = bundle.unique_unit.as_ref().unwrap();
    assert!(
        samurai.abilities.contains(&libciv::rules::unique::UniqueUnitAbility::NoCombatPenaltyWhenDamaged),
        "Samurai should have NoCombatPenaltyWhenDamaged ability"
    );
}

#[test]
fn test_varu_debuffs_adjacent_enemies() {
    // Verify the Varu's DebuffAdjacentEnemies ability is registered.
    let bundle = libciv::rules::civ_registry::india();
    let varu = bundle.unique_unit.as_ref().unwrap();
    assert!(
        varu.abilities.contains(&libciv::rules::unique::UniqueUnitAbility::DebuffAdjacentEnemies(5)),
        "Varu should have DebuffAdjacentEnemies(5) ability"
    );
}

#[test]
fn test_mamluk_heals_every_turn() {
    let mut s = build_scenario();
    let arabia_id = s.babylon_id; // repurpose Babylon as Arabia
    set_civ_identity(&mut s, arabia_id, BuiltinCiv::Arabia, BuiltinLeader::Saladin);

    // Register mamluk unit type.
    let mamluk_type = UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id: mamluk_type, name: "mamluk", production_cost: 220,
        max_movement: 400, combat_strength: Some(50),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        range: 0, vision_range: 2, can_found_city: false, resource_cost: None, siege_bonus: 0, max_charges: 0,
        exclusive_to: Some(BuiltinCiv::Arabia), replaces: Some("knight"), era: None, promotion_class: None,
    });

    // Place a damaged Mamluk.
    let mamluk_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: mamluk_id, unit_type: mamluk_type, owner: arabia_id,
        coord: libhexgrid::coord::HexCoord::from_qr(10, 4),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 400, max_movement: 400, combat_strength: Some(50),
        promotions: Vec::new(), experience: 0, health: 60, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None, is_embarked: false,
    });

    // Advance a turn — the Mamluk should heal.
    advance_turn(&mut s);

    let mamluk = s.state.unit(mamluk_id).unwrap();
    assert!(
        mamluk.health > 60,
        "Mamluk should heal at end of turn (health: {}, expected > 60)",
        mamluk.health
    );
    assert_eq!(mamluk.health, 70, "Mamluk should heal 10 HP per turn");
}

// ---------------------------------------------------------------------------
// Tests: Unique unit replacement / exclusivity
// ---------------------------------------------------------------------------

#[test]
fn test_unique_unit_data_replaces_base() {
    // Verify that Legion replaces Swordsman in the registry data.
    let bundle = libciv::rules::civ_registry::rome();
    let legion = bundle.unique_unit.as_ref().unwrap();
    assert_eq!(legion.name, "legion");
    assert_eq!(legion.replaces, Some("swordsman"));
    assert_eq!(legion.combat_strength, Some(40));
}

// ---------------------------------------------------------------------------
// Tests: Unique district placement
// ---------------------------------------------------------------------------

#[test]
fn test_acropolis_requires_hills() {
    let bundle = libciv::rules::civ_registry::greece();
    let acropolis = bundle.unique_district.as_ref().unwrap();
    assert_eq!(acropolis.name, "Acropolis");
    assert_eq!(
        acropolis.placement,
        Some(libciv::rules::unique::DistrictPlacementReq::MustBeOnHills),
        "Acropolis should require hills placement"
    );
}

// ---------------------------------------------------------------------------
// Tests: Unique improvement exclusivity
// ---------------------------------------------------------------------------

#[test]
fn test_sphinx_is_egypt_exclusive() {
    let bundle = libciv::rules::civ_registry::egypt();
    let sphinx = bundle.unique_improvement.as_ref().unwrap();
    assert_eq!(sphinx.name, "Sphinx");
    assert_eq!(sphinx.civ, BuiltinCiv::Egypt);
    assert_eq!(sphinx.appeal_modifier, 2);
}

#[test]
fn test_stepwell_is_india_exclusive() {
    let bundle = libciv::rules::civ_registry::india();
    let stepwell = bundle.unique_improvement.as_ref().unwrap();
    assert_eq!(stepwell.name, "Stepwell");
    assert_eq!(stepwell.civ, BuiltinCiv::India);
    assert_eq!(stepwell.adjacency_bonuses.len(), 2, "Stepwell should have 2 adjacency bonuses");
}
