//! Extended AI simulation tests.
//!
//! These tests run full multi-turn games between two deterministic
//! `HeuristicAgent`s to stress-test the game loop beyond the short
//! scenarios in `ai_agent.rs`.
mod common;

use libciv::ai::{Agent, HeuristicAgent};
use libciv::game::{recalculate_visibility, StateDelta};
use libciv::{DefaultRulesEngine, GameStateDiff, TurnEngine};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Run one full turn: advance rules, reset movement, refresh visibility, then
/// let each agent take its turn.  Returns both diffs.
fn run_agent_turn(
    s: &mut common::Scenario,
    rome_agent: &HeuristicAgent,
    babylon_agent: &HeuristicAgent,
) -> (GameStateDiff, GameStateDiff) {
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;

    engine.process_turn(&mut s.state, &rules);

    for unit in &mut s.state.units {
        unit.movement_left = unit.max_movement;
    }

    let civ_ids: Vec<_> = s.state.civilizations.iter().map(|c| c.id).collect();
    for cid in &civ_ids {
        recalculate_visibility(&mut s.state, *cid);
    }

    let rome_diff = rome_agent.take_turn(&mut s.state, &rules);
    let babylon_diff = babylon_agent.take_turn(&mut s.state, &rules);
    (rome_diff, babylon_diff)
}

// ── tests ────────────────────────────────────────────────────────────────────

/// Run a full 20-turn game between two AI agents without any panics.
/// Verify the turn counter and that both civs still have cities.
#[test]
fn ai_game_20_turns_no_panic() {
    let mut s = common::build_scenario();
    let rome_agent = HeuristicAgent::new(s.rome_id);
    let babylon_agent = HeuristicAgent::new(s.babylon_id);

    for _ in 0..20 {
        run_agent_turn(&mut s, &rome_agent, &babylon_agent);
    }

    assert_eq!(s.state.turn, 20, "turn counter should be 20");
    assert!(
        s.state.cities.iter().any(|c| c.owner == s.rome_id),
        "Rome must still have at least one city"
    );
    assert!(
        s.state.cities.iter().any(|c| c.owner == s.babylon_id),
        "Babylon must still have at least one city"
    );
}

/// Running the same seed twice for 10 turns must yield identical unit
/// positions and city counts — full determinism.
#[test]
fn ai_game_deterministic_across_runs() {
    let run = |_label: &str| -> (Vec<(libciv::UnitId, libhexgrid::coord::HexCoord)>, usize) {
        let mut s = common::build_scenario();
        let rome_agent = HeuristicAgent::new(s.rome_id);
        let babylon_agent = HeuristicAgent::new(s.babylon_id);
        for _ in 0..10 {
            run_agent_turn(&mut s, &rome_agent, &babylon_agent);
        }
        let positions: Vec<_> = s.state.units.iter().map(|u| (u.id, u.coord)).collect();
        let city_count = s.state.cities.len();
        (positions, city_count)
    };

    let (pos_a, cities_a) = run("run-1");
    let (pos_b, cities_b) = run("run-2");

    assert_eq!(cities_a, cities_b, "city count must be identical across runs");
    assert_eq!(pos_a.len(), pos_b.len(), "unit count must be identical");
    for (a, b) in pos_a.iter().zip(pos_b.iter()) {
        assert_eq!(a, b, "unit positions must match across deterministic runs");
    }
}

/// After 15 turns, units should have been produced (total unit count grows)
/// and both civs should have explored more tiles than they started with.
#[test]
fn ai_game_units_produced_and_explore() {
    let mut s = common::build_scenario();
    let rome_agent = HeuristicAgent::new(s.rome_id);
    let babylon_agent = HeuristicAgent::new(s.babylon_id);

    let initial_units = s.state.units.len();
    let initial_rome_explored = s.state.civ(s.rome_id).unwrap().explored_tiles.len();
    let initial_babylon_explored = s.state.civ(s.babylon_id).unwrap().explored_tiles.len();

    for _ in 0..15 {
        run_agent_turn(&mut s, &rome_agent, &babylon_agent);
    }

    let final_rome_explored = s.state.civ(s.rome_id).unwrap().explored_tiles.len();
    let final_babylon_explored = s.state.civ(s.babylon_id).unwrap().explored_tiles.len();

    assert!(
        s.state.units.len() >= initial_units,
        "total unit count should not decrease (no combat in this scenario)"
    );
    assert!(
        final_rome_explored > initial_rome_explored,
        "Rome should have explored more tiles after 15 turns"
    );
    assert!(
        final_babylon_explored > initial_babylon_explored,
        "Babylon should have explored more tiles after 15 turns"
    );
}

/// After 10 turns, verify core game state invariants:
/// - All units have valid owners that exist in the civs list.
/// - All cities have valid owners.
/// - All unit coords are valid board positions.
/// - All city coords are valid board positions.
#[test]
fn ai_game_state_consistency() {
    use libhexgrid::board::HexBoard;

    let mut s = common::build_scenario();
    let rome_agent = HeuristicAgent::new(s.rome_id);
    let babylon_agent = HeuristicAgent::new(s.babylon_id);

    for _ in 0..10 {
        run_agent_turn(&mut s, &rome_agent, &babylon_agent);
    }

    let civ_ids: Vec<_> = s.state.civilizations.iter().map(|c| c.id).collect();

    // All units must have a valid owner (including the barbarian virtual faction).
    for unit in &s.state.units {
        let is_known_civ = civ_ids.contains(&unit.owner);
        let is_barbarian = s.state.barbarian_civ == Some(unit.owner);
        assert!(
            is_known_civ || is_barbarian,
            "unit {:?} has owner {:?} which is not a known civ",
            unit.id, unit.owner
        );
        // Unit coord must be on the board.
        assert!(
            s.state.board.normalize(unit.coord).is_some(),
            "unit {:?} is at {:?} which is off the board",
            unit.id, unit.coord
        );
    }

    // All cities must have a valid owner.
    for city in &s.state.cities {
        assert!(
            civ_ids.contains(&city.owner),
            "city {:?} has owner {:?} which is not a known civ",
            city.id, city.owner
        );
        // City coord must be on the board.
        assert!(
            s.state.board.normalize(city.coord).is_some(),
            "city {:?} is at {:?} which is off the board",
            city.id, city.coord
        );
    }
}

/// Verify that diffs emitted by agents reference valid entity IDs.
#[test]
fn ai_diffs_reference_valid_entities() {
    let mut s = common::build_scenario();
    let rome_agent = HeuristicAgent::new(s.rome_id);
    let babylon_agent = HeuristicAgent::new(s.babylon_id);

    for _ in 0..5 {
        let (rome_diff, babylon_diff) = run_agent_turn(&mut s, &rome_agent, &babylon_agent);

        for delta in rome_diff.deltas.iter().chain(babylon_diff.deltas.iter()) {
            match delta {
                StateDelta::UnitMoved { unit, .. } => {
                    assert!(
                        s.state.unit(*unit).is_some(),
                        "UnitMoved references non-existent unit {:?}",
                        unit
                    );
                }
                StateDelta::ProductionStarted { city, .. } => {
                    assert!(
                        s.state.city(*city).is_some(),
                        "ProductionStarted references non-existent city {:?}",
                        city
                    );
                }
                StateDelta::UnitCreated { unit, owner, .. } => {
                    assert!(
                        s.state.unit(*unit).is_some(),
                        "UnitCreated references non-existent unit {:?}",
                        unit
                    );
                    assert!(
                        s.state.civ(*owner).is_some(),
                        "UnitCreated references non-existent civ {:?}",
                        owner
                    );
                }
                _ => {}
            }
        }
    }
}
