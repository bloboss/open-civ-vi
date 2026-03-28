/// Integration tests for the era score system.
///
/// These tests verify that historic moments are detected from game events,
/// era score accumulates correctly, and era transitions produce the correct
/// era ages (Dark/Normal/Golden/Heroic).
mod common;

use libciv::{AgeType, DefaultRulesEngine, EraAge, RulesEngine, TechId};
use libciv::civ::era::{self, Era, DARK_AGE_THRESHOLD, GOLDEN_AGE_THRESHOLD};
use libciv::civ::TechProgress;
use libciv::game::StateDelta;
use libciv::rules::TechNode;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Historic moment detection
// ---------------------------------------------------------------------------

/// Founding a city produces a CityFounded delta. The observer correctly
/// identifies this as a historic moment.
#[test]
fn test_historic_moment_earned_on_city_founded() {
    let mut s = common::build_scenario();
    let engine = DefaultRulesEngine;

    // Give Rome a settler and found a city.
    let settler_id = s.state.id_gen.next_unit_id();
    // Place far enough from existing cities to avoid TooCloseToCity.
    let coord = HexCoord::from_qr(0, 0);
    s.state.units.push(libciv::civ::BasicUnit {
        id: settler_id,
        unit_type: s.settler_type,
        owner: s.rome_id,
        coord,
        domain: libciv::UnitDomain::Land,
        category: libciv::UnitCategory::Civilian,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None,
    });

    let found_diff = engine.found_city(&mut s.state, settler_id, "Antium".to_string()).unwrap();

    // The found_city diff contains a CityFounded delta. Test the observer directly.
    use libciv::civ::historic_moments::observe_deltas;
    let moments = observe_deltas(&found_diff.deltas, &s.state);
    let city_moment = moments.iter().find(|(civ, m)| {
        *civ == s.rome_id && m.name == "City Founded"
    });
    assert!(city_moment.is_some(), "expected 'City Founded' historic moment for Rome");

    // Verify era_score increases when the observer result is applied.
    let (_, moment_def) = city_moment.unwrap();
    assert!(moment_def.era_score > 0, "City Founded should award era score");
}

/// Completing a tech research should earn era score.
#[test]
fn test_historic_moment_earned_on_tech_researched() {
    let mut s = common::build_scenario();
    let engine = DefaultRulesEngine;

    // Set up a cheap tech that will complete in one turn.
    let tech_id = TechId::from_ulid(s.state.id_gen.next_ulid());
    s.state.tech_tree.add_node(TechNode {
        id: tech_id,
        name: "Pottery",
        cost: 1, // completes immediately with any science
        prerequisites: vec![],
        effects: vec![],
        eureka_description: "",
        eureka_effects: vec![],
    });

    let rome = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome.research_queue.push_back(TechProgress {
        tech_id,
        progress: 0,
        boosted: false,
    });

    let diff = engine.advance_turn(&mut s.state);

    // Check that a TechResearched delta was emitted.
    let has_tech = diff.deltas.iter().any(|d| matches!(d, StateDelta::TechResearched { civ, .. } if *civ == s.rome_id));
    assert!(has_tech, "expected TechResearched delta");

    // Check that a HistoricMomentEarned delta was also emitted.
    let has_moment = diff.deltas.iter().any(|d| {
        matches!(d, StateDelta::HistoricMomentEarned { civ, moment, .. }
            if *civ == s.rome_id && *moment == "Technology Researched")
    });
    assert!(has_moment, "expected HistoricMomentEarned for tech research");

    let rome = s.state.civ(s.rome_id).unwrap();
    assert!(rome.era_score > 0, "expected era score > 0 after tech research, got {}", rome.era_score);
}

/// A unique moment should only be earned once per civ per era.
#[test]
fn test_unique_moment_not_duplicated() {
    let mut s = common::build_scenario();
    let engine = DefaultRulesEngine;

    // Found two cities in succession -- "First City Founded" is unique and
    // should only award era score once.
    // Place settlers far from existing cities and from each other.
    let coords = [HexCoord::from_qr(0, 0), HexCoord::from_qr(13, 7)];
    let names = ["Antium", "Cumae"];
    for i in 0..2 {
        let settler_id = s.state.id_gen.next_unit_id();
        s.state.units.push(libciv::civ::BasicUnit {
            id: settler_id,
            unit_type: s.settler_type,
            owner: s.rome_id,
            coord: coords[i],
            domain: libciv::UnitDomain::Land,
            category: libciv::UnitCategory::Civilian,
            movement_left: 200,
            max_movement: 200,
            combat_strength: None,
            promotions: Vec::new(),
            health: 100,
            range: 0,
            vision_range: 2,
            charges: None, trade_origin: None, trade_destination: None,
        });
        let _ = engine.found_city(&mut s.state, settler_id, names[i].to_string());
    }

    let diff = engine.advance_turn(&mut s.state);

    // Count "First City Founded" moments for Rome in the diff.
    let first_city_moments: Vec<_> = diff.deltas.iter().filter(|d| {
        matches!(d, StateDelta::HistoricMomentEarned { civ, moment, .. }
            if *civ == s.rome_id && *moment == "First City Founded")
    }).collect();
    assert!(first_city_moments.len() <= 1,
        "unique moment 'First City Founded' should appear at most once, got {}",
        first_city_moments.len());
}

// ---------------------------------------------------------------------------
// Era advancement and age determination
// ---------------------------------------------------------------------------

/// Helper to set up a two-era scenario (Ancient -> Classical).
fn setup_two_era_scenario(s: &mut common::Scenario) {
    let era_ancient = Era {
        id: s.state.current_era,
        name: "Ancient",
        age: AgeType::Ancient,
        tech_count: 1,
        civic_count: 0,
    };
    let era_classical_id = libciv::EraId::from_ulid(s.state.id_gen.next_ulid());
    let era_classical = Era {
        id: era_classical_id,
        name: "Classical",
        age: AgeType::Classical,
        tech_count: 3,
        civic_count: 0,
    };
    s.state.eras = vec![era_ancient, era_classical];
    s.state.current_era_index = 0;
}

/// Helper to add a researched tech to trigger era advancement.
fn add_trigger_tech(s: &mut common::Scenario, name: &'static str) {
    let tech_id = TechId::from_ulid(s.state.id_gen.next_ulid());
    s.state.tech_tree.add_node(TechNode {
        id: tech_id, name, cost: 1,
        prerequisites: vec![], effects: vec![],
        eureka_description: "", eureka_effects: vec![],
    });
    let rome = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome.researched_techs.push(tech_id);
}

/// When the global era advances, civ era_score should reset to 0.
#[test]
fn test_era_advancement_resets_score() {
    let mut s = common::build_scenario();
    setup_two_era_scenario(&mut s);

    let rome = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome.era_score = 15;

    add_trigger_tech(&mut s, "Mining");

    let engine = DefaultRulesEngine;
    let _diff = engine.advance_turn(&mut s.state);

    assert_eq!(s.state.current_era_index, 1, "expected era to advance to index 1");

    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(rome.era_score, 0, "expected era score to reset after era advancement");
}

/// High era score should result in a Golden Age.
#[test]
fn test_golden_age_from_high_score() {
    let mut s = common::build_scenario();
    setup_two_era_scenario(&mut s);

    let rome = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome.era_score = GOLDEN_AGE_THRESHOLD + 5;

    add_trigger_tech(&mut s, "Writing");

    let engine = DefaultRulesEngine;
    let _diff = engine.advance_turn(&mut s.state);

    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(rome.era_age, EraAge::Golden, "expected Golden Age from high era score");
}

/// Low era score should result in a Dark Age.
#[test]
fn test_dark_age_from_low_score() {
    let mut s = common::build_scenario();
    setup_two_era_scenario(&mut s);

    let rome = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome.era_score = DARK_AGE_THRESHOLD - 5;

    add_trigger_tech(&mut s, "Irrigation");

    let engine = DefaultRulesEngine;
    let _diff = engine.advance_turn(&mut s.state);

    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(rome.era_age, EraAge::Dark, "expected Dark Age from low era score");
}

/// A high era score after a Dark Age should result in a Heroic Age.
#[test]
fn test_heroic_age_after_dark() {
    let mut s = common::build_scenario();
    setup_two_era_scenario(&mut s);

    let rome = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome.era_age = EraAge::Dark;
    rome.era_score = GOLDEN_AGE_THRESHOLD + 10;

    add_trigger_tech(&mut s, "Masonry");

    let engine = DefaultRulesEngine;
    let _diff = engine.advance_turn(&mut s.state);

    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(rome.era_age, EraAge::Heroic, "expected Heroic Age after Dark Age with high score");
}

/// An EraAdvanced delta should be emitted for each civ when the era advances.
#[test]
fn test_era_advanced_delta_emitted() {
    let mut s = common::build_scenario();
    setup_two_era_scenario(&mut s);

    let rome = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome.era_score = 15;

    add_trigger_tech(&mut s, "Sailing");

    let engine = DefaultRulesEngine;
    let diff = engine.advance_turn(&mut s.state);

    let era_deltas: Vec<_> = diff.deltas.iter().filter(|d| matches!(d, StateDelta::EraAdvanced { .. })).collect();
    assert_eq!(era_deltas.len(), 2, "expected EraAdvanced delta for each civ, got {}", era_deltas.len());
}

/// Killing an enemy unit in combat should earn era score for the attacker.
#[test]
fn test_battle_won_earns_era_score() {
    let mut s = common::build_scenario();
    let engine = DefaultRulesEngine;

    // Move Rome's warrior adjacent to Babylon's warrior and attack.
    let attack_coord = HexCoord::from_qr(7, 5);
    if let Some(u) = s.state.unit_mut(s.rome_warrior) {
        u.coord = attack_coord;
        u.movement_left = 200;
    }

    // Weaken the Babylon warrior so it dies in one hit.
    if let Some(u) = s.state.unit_mut(s.babylon_warrior) {
        u.health = 1;
    }

    let attack_diff = engine.attack(&mut s.state, s.rome_warrior, s.babylon_warrior);
    assert!(attack_diff.is_ok(), "attack should succeed");
    let attack_diff = attack_diff.unwrap();

    // Verify the attack produced both UnitAttacked and UnitDestroyed.
    let has_attack = attack_diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitAttacked { .. }));
    let has_destroy = attack_diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitDestroyed { .. }));
    assert!(has_attack, "expected UnitAttacked delta");
    assert!(has_destroy, "expected UnitDestroyed delta");

    // The observer processes deltas from the current advance_turn pass.
    // Since the attack happened outside advance_turn, we test the observer
    // function directly.
    use libciv::civ::historic_moments::observe_deltas;
    let moments = observe_deltas(&attack_diff.deltas, &s.state);
    let battle_moment = moments.iter().find(|(civ, m)| *civ == s.rome_id && m.name == "Enemy Defeated in Battle");
    assert!(battle_moment.is_some(), "expected BattleWon historic moment for Rome");
}

// ---------------------------------------------------------------------------
// compute_era_age unit tests (pure logic, no game state)
// ---------------------------------------------------------------------------

#[test]
fn test_compute_era_age_boundaries() {
    assert_eq!(era::compute_era_age(0, false), EraAge::Dark);
    assert_eq!(era::compute_era_age(DARK_AGE_THRESHOLD, false), EraAge::Dark);
    assert_eq!(era::compute_era_age(DARK_AGE_THRESHOLD + 1, false), EraAge::Normal);
    assert_eq!(era::compute_era_age(GOLDEN_AGE_THRESHOLD - 1, false), EraAge::Normal);
    assert_eq!(era::compute_era_age(GOLDEN_AGE_THRESHOLD, false), EraAge::Golden);
    assert_eq!(era::compute_era_age(GOLDEN_AGE_THRESHOLD, true), EraAge::Heroic);
}

/// Verify that should_advance_era returns true when threshold crossed.
#[test]
fn test_should_advance_era_threshold() {
    let mut s = common::build_scenario();
    let era = Era {
        id: s.state.current_era,
        name: "Ancient",
        age: AgeType::Ancient,
        tech_count: 2,
        civic_count: 0,
    };

    // No techs yet -- should not advance.
    assert!(!era::should_advance_era(&era, &s.state.civilizations));

    // Add 2 techs to Rome.
    let t1 = TechId::from_ulid(s.state.id_gen.next_ulid());
    let t2 = TechId::from_ulid(s.state.id_gen.next_ulid());
    let rome = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    rome.researched_techs.push(t1);
    rome.researched_techs.push(t2);

    assert!(era::should_advance_era(&era, &s.state.civilizations));
}
