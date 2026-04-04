/// Integration tests for city projects (GS-13).
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::civ::city::ProductionItem;
use libciv::game::StateDelta;

/// Find a project def by name and return its id + cost.
fn find_project(state: &libciv::GameState, name: &str) -> (libciv::ProjectId, u32) {
    let def = state.project_defs.iter().find(|d| d.name == name)
        .unwrap_or_else(|| panic!("project {name} not found in registry"));
    (def.id, def.production_cost)
}

#[test]
fn project_completes_after_production() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Rome's city the required district for "Campus Research Grants".
    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.districts.push(libciv::civ::district::BuiltinDistrict::Campus);

    let (proj_id, cost) = find_project(&s.state, "Campus Research Grants");

    // Queue the project.
    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.production_queue.push_back(ProductionItem::Project(proj_id));
    // Set stored production high enough to complete in one turn.
    city.production_stored = cost;

    let diff = rules.advance_turn(&mut s.state);

    let completed = diff.deltas.iter().any(|d| matches!(
        d, StateDelta::ProjectCompleted { project, .. } if *project == "Campus Research Grants"
    ));
    assert!(completed, "expected ProjectCompleted delta for Campus Research Grants");

    // Queue should be empty after completion.
    let city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert!(city.production_queue.is_empty(), "queue should be empty after project completes");
}

#[test]
fn science_project_increments_milestones() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Rome's city a Spaceport district.
    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.districts.push(libciv::civ::district::BuiltinDistrict::Spaceport);

    let (proj_id, cost) = find_project(&s.state, "Launch Earth Satellite");

    // Queue and give enough production.
    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.production_queue.push_back(ProductionItem::Project(proj_id));
    city.production_stored = cost;

    let initial = s.state.civ(s.rome_id).unwrap().science_milestones_completed;

    let diff = rules.advance_turn(&mut s.state);

    let new_milestones = s.state.civ(s.rome_id).unwrap().science_milestones_completed;
    assert_eq!(new_milestones, initial + 1, "science milestones should increment by 1");

    // Should also emit ScienceMilestoneCompleted delta.
    let has_milestone_delta = diff.deltas.iter().any(|d| matches!(
        d, StateDelta::ScienceMilestoneCompleted { milestone, .. } if *milestone == "Launch Earth Satellite"
    ));
    assert!(has_milestone_delta, "expected ScienceMilestoneCompleted delta");
}

#[test]
fn carbon_recapture_reduces_co2() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Set initial CO2.
    s.state.global_co2 = 200;

    // Give Rome's city an Industrial Zone district.
    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.districts.push(libciv::civ::district::BuiltinDistrict::IndustrialZone);

    let (proj_id, cost) = find_project(&s.state, "Carbon Recapture");

    // Queue and give enough production.
    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.production_queue.push_back(ProductionItem::Project(proj_id));
    city.production_stored = cost;

    rules.advance_turn(&mut s.state);

    assert_eq!(s.state.global_co2, 150, "Carbon Recapture should reduce CO2 by 50");
}

#[test]
fn non_repeatable_project_removed_from_defs_after_completion() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.districts.push(libciv::civ::district::BuiltinDistrict::Spaceport);

    let (proj_id, cost) = find_project(&s.state, "Launch Earth Satellite");

    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.production_queue.push_back(ProductionItem::Project(proj_id));
    city.production_stored = cost;

    rules.advance_turn(&mut s.state);

    // Non-repeatable project should be removed from defs.
    let still_exists = s.state.project_defs.iter().any(|d| d.name == "Launch Earth Satellite");
    assert!(!still_exists, "non-repeatable project should be removed from defs");
}

#[test]
fn repeatable_project_stays_in_defs() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.districts.push(libciv::civ::district::BuiltinDistrict::Campus);

    let (proj_id, cost) = find_project(&s.state, "Campus Research Grants");

    let city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    city.production_queue.push_back(ProductionItem::Project(proj_id));
    city.production_stored = cost;

    rules.advance_turn(&mut s.state);

    // Repeatable project should still be in defs.
    let still_exists = s.state.project_defs.iter().any(|d| d.name == "Campus Research Grants");
    assert!(still_exists, "repeatable project should remain in defs");
}
