mod common;

use libciv::{CivId, VictoryId};
use libciv::game::{ScienceVictory, VictoryCondition, VictoryKind, StateDelta};
use libciv::civ::city::ProductionItem;
use libciv::civ::district::BuiltinDistrict;
use libciv::rules::VictoryProgress;

// ---------------------------------------------------------------------------
// Helper: grant a list of techs to a civ (by TechId).
// ---------------------------------------------------------------------------
fn grant_techs(state: &mut libciv::GameState, civ_id: CivId, tech_ids: &[libciv::TechId]) {
    let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
    for &tid in tech_ids {
        if !civ.researched_techs.contains(&tid) {
            civ.researched_techs.push(tid);
        }
    }
}

/// Grant the full prerequisite chain needed to research Satellites (and place a Spaceport).
fn grant_science_victory_techs(state: &mut libciv::GameState, civ_id: CivId) {
    let tr = state.tech_refs;
    grant_techs(state, civ_id, &[
        // Ancient
        tr.pottery, tr.mining, tr.sailing, tr.archery, tr.writing, tr.masonry, tr.the_wheel,
        // Classical
        tr.currency, tr.mathematics, tr.engineering, tr.construction, tr.shipbuilding,
        // Medieval
        tr.apprenticeship, tr.education, tr.machinery, tr.military_tactics,
        tr.military_engineering, tr.castles,
        // Renaissance
        tr.printing, tr.astronomy, tr.gunpowder, tr.mass_production, tr.metal_casting,
        tr.banking, tr.cartography, tr.square_rigging, tr.siege_tactics,
        // Industrial
        tr.scientific_theory, tr.ballistics, tr.industrialization, tr.sanitation,
        tr.steam_power, tr.rifling, tr.economics,
        // Modern
        tr.flight, tr.electricity, tr.radio, tr.chemistry, tr.steel, tr.combustion,
        tr.replaceable_parts,
        // Atomic
        tr.rocketry, tr.advanced_flight, tr.combined_arms, tr.plastics,
        tr.computers, tr.nuclear_fission, tr.synthetic_materials,
        tr.advanced_ballistics,
        // Information
        tr.satellites, tr.robotics, tr.nuclear_fusion, tr.nanotechnology,
        tr.lasers, tr.composites,
    ]);
}

/// Give a city a Spaceport district (directly, bypassing placement validation).
fn give_spaceport(s: &mut common::Scenario, city_idx: usize) {
    let city = &mut s.state.cities[city_idx];
    if !city.districts.contains(&BuiltinDistrict::Spaceport) {
        city.districts.push(BuiltinDistrict::Spaceport);
    }
}

fn make_victory_id(s: &mut common::Scenario) -> VictoryId {
    VictoryId::from_ulid(s.state.id_gen.next_ulid())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn science_victory_progress_zero_without_projects() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };

    let progress = sv.check_progress(s.rome_id, &s.state);
    assert_eq!(progress.current, 0);
    assert_eq!(progress.target, 4);
    assert!(!progress.is_won());
}

#[test]
fn science_victory_progress_counts_completed_projects() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };

    // Manually add some projects
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");
    civ.completed_projects.push("Moon Colony");

    let progress = sv.check_progress(s.rome_id, &s.state);
    assert_eq!(progress.current, 2);
    assert_eq!(progress.target, 4);
    assert!(!progress.is_won());
}

#[test]
fn science_victory_wins_on_all_four() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };

    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");
    civ.completed_projects.push("Moon Colony");
    civ.completed_projects.push("Mars Colony");
    civ.completed_projects.push("Interstellar Colony Ship");

    let progress = sv.check_progress(s.rome_id, &s.state);
    assert_eq!(progress.current, 4);
    assert!(progress.is_won());
}

#[test]
fn science_victory_not_won_with_three() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };

    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");
    civ.completed_projects.push("Moon Colony");
    civ.completed_projects.push("Mars Colony");

    let progress = sv.check_progress(s.rome_id, &s.state);
    assert_eq!(progress.current, 3);
    assert!(!progress.is_won());
}

#[test]
fn science_victory_is_immediate_win() {
    let mut s = common::build_scenario();
    let vid = make_victory_id(&mut s);
    let sv = ScienceVictory { id: vid };
    assert_eq!(sv.kind(), VictoryKind::ImmediateWin);
    assert_eq!(sv.name(), "Science Victory");
}

#[test]
fn project_completion_in_advance_turn() {
    let mut s = common::build_scenario();

    // Grant all required techs including Satellites
    grant_science_victory_techs(&mut s.state, s.rome_id);

    // Give Rome's city a Spaceport
    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    give_spaceport(&mut s, rome_city_idx);

    // Queue the Launch Satellite project with enough production
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Launch Satellite")
    );
    s.state.cities[rome_city_idx].production_stored = 1500;

    // Advance one turn
    common::advance_turn(&mut s);

    // Verify the project completed
    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(civ.completed_projects.contains(&"Launch Satellite"),
        "Launch Satellite should be in completed_projects");
    assert!(s.state.cities[rome_city_idx].production_queue.is_empty(),
        "Production queue should be empty after completion");
}

#[test]
fn project_requires_spaceport() {
    let mut s = common::build_scenario();

    // Grant techs but NO Spaceport
    grant_science_victory_techs(&mut s.state, s.rome_id);

    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Launch Satellite")
    );
    s.state.cities[rome_city_idx].production_stored = 2000;

    common::advance_turn(&mut s);

    // Should NOT complete — no Spaceport
    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(!civ.completed_projects.contains(&"Launch Satellite"),
        "Project should not complete without Spaceport");
    assert!(!s.state.cities[rome_city_idx].production_queue.is_empty(),
        "Project should remain in queue");
}

#[test]
fn project_requires_prerequisite_project() {
    let mut s = common::build_scenario();

    grant_science_victory_techs(&mut s.state, s.rome_id);

    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    give_spaceport(&mut s, rome_city_idx);

    // Try building Moon Colony without Launch Satellite
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Moon Colony")
    );
    s.state.cities[rome_city_idx].production_stored = 3000;

    common::advance_turn(&mut s);

    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(!civ.completed_projects.contains(&"Moon Colony"),
        "Moon Colony should not complete without Launch Satellite prerequisite");
}

#[test]
fn project_no_double_build() {
    let mut s = common::build_scenario();
    grant_science_victory_techs(&mut s.state, s.rome_id);

    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    give_spaceport(&mut s, rome_city_idx);

    // Complete Launch Satellite first
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");

    // Try building it again
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Launch Satellite")
    );
    s.state.cities[rome_city_idx].production_stored = 2000;

    common::advance_turn(&mut s);

    // Should not complete (already built)
    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert_eq!(civ.completed_projects.iter().filter(|&&p| p == "Launch Satellite").count(), 1,
        "Should not allow double-building a project");
}

#[test]
fn science_victory_triggers_game_over() {
    let mut s = common::build_scenario();
    grant_science_victory_techs(&mut s.state, s.rome_id);

    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    give_spaceport(&mut s, rome_city_idx);

    // Register the Science Victory condition
    let vid = make_victory_id(&mut s);
    s.state.victory_conditions.push(Box::new(ScienceVictory { id: vid }));

    // Complete the first three projects manually
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.completed_projects.push("Launch Satellite");
    civ.completed_projects.push("Moon Colony");
    civ.completed_projects.push("Mars Colony");

    // Queue the final project
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Project("Interstellar Colony Ship")
    );
    s.state.cities[rome_city_idx].production_stored = 3000;

    common::advance_turn(&mut s);

    // Game should be over with Science Victory
    assert!(s.state.game_over.is_some(), "Game should be over after completing all projects");
    let game_over = s.state.game_over.as_ref().unwrap();
    assert_eq!(game_over.winner, s.rome_id);
    assert_eq!(game_over.condition, "Science Victory");
}

#[test]
fn spaceport_requires_rocketry_tech() {
    let s = common::build_scenario();
    let tr = s.state.tech_refs;
    let cr = s.state.civic_refs;
    let req = BuiltinDistrict::Spaceport.requirements(&tr, &cr);
    assert_eq!(req.required_tech, Some(tr.rocketry));
    assert!(req.requires_land);
    assert!(!req.requires_water);
}

#[test]
fn building_completion_in_advance_turn() {
    let mut s = common::build_scenario();

    // Grant Writing tech (Library requires it)
    let tr = s.state.tech_refs;
    grant_techs(&mut s.state, s.rome_id, &[tr.pottery, tr.writing]);

    // Register a Library building def
    let library_id = libciv::BuildingId::from_ulid(s.state.id_gen.next_ulid());
    s.state.building_defs.push(libciv::game::state::BuildingDef {
        id: library_id,
        name: "Library",
        cost: 90,
        maintenance: 1,
        yields: libciv::YieldBundle { science: 2, ..Default::default() },
        requires_district: Some("Campus"),
        required_tech: Some(tr.writing),
    });

    // Give the city a Campus district
    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    s.state.cities[rome_city_idx].districts.push(BuiltinDistrict::Campus);

    // Queue the Library with enough production
    s.state.cities[rome_city_idx].production_queue.push_back(
        ProductionItem::Building(library_id)
    );
    s.state.cities[rome_city_idx].production_stored = 90;

    common::advance_turn(&mut s);

    // Library should be completed
    assert!(s.state.cities[rome_city_idx].buildings.contains(&library_id),
        "Library should be in city.buildings after completion");
    assert!(s.state.cities[rome_city_idx].production_queue.is_empty(),
        "Production queue should be empty after completion");
}

#[test]
fn building_yields_contribute_to_compute_yields() {
    let mut s = common::build_scenario();
    let tr = s.state.tech_refs;
    grant_techs(&mut s.state, s.rome_id, &[tr.pottery, tr.writing]);

    // Register Library
    let library_id = libciv::BuildingId::from_ulid(s.state.id_gen.next_ulid());
    s.state.building_defs.push(libciv::game::state::BuildingDef {
        id: library_id,
        name: "Library",
        cost: 90,
        maintenance: 1,
        yields: libciv::YieldBundle { science: 2, ..Default::default() },
        requires_district: Some("Campus"),
        required_tech: Some(tr.writing),
    });

    let engine = libciv::DefaultRulesEngine;

    // Yields without the building
    let yields_before = libciv::RulesEngine::compute_yields(&engine, &s.state, s.rome_id);

    // Add the Library to the city
    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    s.state.cities[rome_city_idx].buildings.push(library_id);

    // Yields with the building
    let yields_after = libciv::RulesEngine::compute_yields(&engine, &s.state, s.rome_id);

    assert_eq!(yields_after.science, yields_before.science + 2,
        "Library should add +2 science to yields");
}
