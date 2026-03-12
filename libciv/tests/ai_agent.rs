//! Integration tests for [`HeuristicAgent`].
//!
//! These tests drive the agent against the shared two-civ scenario from
//! `common` and verify end-to-end behaviour: production, movement,
//! determinism, and multi-turn stability.
mod common;

use libciv::ai::{Agent, HeuristicAgent};
use libciv::civ::ProductionItem;
use libciv::game::{recalculate_visibility, StateDelta};
use libciv::{DefaultRulesEngine, GameStateDiff, TurnEngine};
use libhexgrid::coord::HexCoord;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Full turn: advance rules, reset movement, refresh visibility, then let each
/// agent take its turn.
fn run_agent_turn(
    s: &mut common::Scenario,
    rome_agent: &HeuristicAgent,
    babylon_agent: &HeuristicAgent,
) -> (GameStateDiff, GameStateDiff) {
    let engine = TurnEngine::new();
    let rules  = DefaultRulesEngine;

    // End-of-turn processing.
    engine.process_turn(&mut s.state, &rules);

    // Reset movement for all units.
    for unit in &mut s.state.units {
        unit.movement_left = unit.max_movement;
    }

    // Refresh visibility.
    let civ_ids: Vec<_> = s.state.civilizations.iter().map(|c| c.id).collect();
    for cid in &civ_ids {
        recalculate_visibility(&mut s.state, *cid);
    }

    // Agent decisions.
    let rome_diff     = rome_agent.take_turn(&mut s.state, &rules);
    let babylon_diff  = babylon_agent.take_turn(&mut s.state, &rules);
    (rome_diff, babylon_diff)
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// The agent queues a combat unit in each city that has an empty production
/// queue on the very first call to `take_turn`.
#[test]
fn agent_queues_production_on_first_turn() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_agent = HeuristicAgent::new(s.rome_id);
    let diff = rome_agent.take_turn(&mut s.state, &rules);

    // A ProductionStarted delta must appear.
    let has_production_started = diff.deltas.iter().any(|d| {
        matches!(d, StateDelta::ProductionStarted { city, .. } if *city == s.rome_city)
    });
    assert!(has_production_started, "expected ProductionStarted for Rome's city");

    // The city's queue must be non-empty.
    let city = s.state.city(s.rome_city).unwrap();
    assert!(!city.production_queue.is_empty(), "production queue should be non-empty");

    // Must be a unit (the only registered type is warrior/settler; warrior is Combat).
    assert!(
        matches!(city.production_queue.front(), Some(ProductionItem::Unit(_))),
        "first item in queue must be a unit"
    );
}

/// Agent does not re-queue production when the queue is already populated.
#[test]
fn agent_does_not_double_queue_production() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_agent = HeuristicAgent::new(s.rome_id);

    // First call fills the queue.
    rome_agent.take_turn(&mut s.state, &rules);
    let queue_len_after_first = s.state.city(s.rome_city).unwrap().production_queue.len();

    // Second call must not add more items.
    rome_agent.take_turn(&mut s.state, &rules);
    let queue_len_after_second = s.state.city(s.rome_city).unwrap().production_queue.len();

    assert_eq!(
        queue_len_after_first, queue_len_after_second,
        "agent must not re-queue production when the queue is already populated"
    );
}

/// The agent moves its warrior unit on the first turn (there are unexplored
/// tiles beyond its initial visibility radius).
#[test]
fn agent_moves_warrior_toward_unexplored_territory() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_agent = HeuristicAgent::new(s.rome_id);
    let initial_coord = s.state.unit(s.rome_warrior).unwrap().coord;

    let diff = rome_agent.take_turn(&mut s.state, &rules);

    let unit_after = s.state.unit(s.rome_warrior).unwrap();

    // The unit should have moved (unexplored tiles have score 100 vs visible 10).
    assert_ne!(
        unit_after.coord, initial_coord,
        "agent should move the warrior toward unexplored territory"
    );

    // A UnitMoved delta must be present.
    let has_unit_moved = diff.deltas.iter().any(|d| {
        matches!(d, StateDelta::UnitMoved { unit, .. } if *unit == s.rome_warrior)
    });
    assert!(has_unit_moved, "diff must contain a UnitMoved delta for Rome's warrior");
}

/// The agent only moves units that it owns.
#[test]
fn agent_does_not_move_enemy_units() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let babylon_coord_before = s.state.unit(s.babylon_warrior).unwrap().coord;

    // Only Rome's agent acts.
    let rome_agent = HeuristicAgent::new(s.rome_id);
    rome_agent.take_turn(&mut s.state, &rules);

    let babylon_coord_after = s.state.unit(s.babylon_warrior).unwrap().coord;
    assert_eq!(
        babylon_coord_before, babylon_coord_after,
        "Rome's agent must not move Babylon's warrior"
    );
}

/// Both agents can take turns independently without panicking.
#[test]
fn both_agents_take_first_turn_without_panic() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_agent     = HeuristicAgent::new(s.rome_id);
    let babylon_agent  = HeuristicAgent::new(s.babylon_id);

    // Should not panic.
    let _rd = rome_agent.take_turn(&mut s.state, &rules);
    let _bd = babylon_agent.take_turn(&mut s.state, &rules);
}

/// The agent produces identical diffs when given two identical game states
/// built from the same seed (full determinism check).
#[test]
fn agent_is_deterministic_same_seed() {
    let mut s1 = common::build_scenario();
    let mut s2 = common::build_scenario();
    let rules  = DefaultRulesEngine;

    let diff1 = HeuristicAgent::new(s1.rome_id).take_turn(&mut s1.state, &rules);
    let diff2 = HeuristicAgent::new(s2.rome_id).take_turn(&mut s2.state, &rules);

    assert_eq!(
        diff1.len(), diff2.len(),
        "agent must produce the same number of diff events for identical states"
    );

    // Warrior positions must match.
    assert_eq!(
        s1.state.unit(s1.rome_warrior).unwrap().coord,
        s2.state.unit(s2.rome_warrior).unwrap().coord,
        "unit positions must be identical across two deterministic runs"
    );
}

/// The agent runs for several turns without panicking or leaving an
/// inconsistent state (production queue grows, units keep moving).
#[test]
fn agent_runs_multiple_turns_without_panic() {
    let mut s = common::build_scenario();

    let rome_agent    = HeuristicAgent::new(s.rome_id);
    let babylon_agent = HeuristicAgent::new(s.babylon_id);

    for _ in 0..5 {
        run_agent_turn(&mut s, &rome_agent, &babylon_agent);
    }

    assert_eq!(s.state.turn, 5, "turn counter should be 5 after 5 agent turns");
}

/// After running for several turns, Rome's warrior must have moved away from
/// its starting position (it should not get permanently stuck).
#[test]
fn warrior_explores_over_multiple_turns() {
    let initial_coord = HexCoord::from_qr(5, 3); // Rome's warrior starts here

    let mut s = common::build_scenario();
    let rome_agent    = HeuristicAgent::new(s.rome_id);
    let babylon_agent = HeuristicAgent::new(s.babylon_id);

    for _ in 0..3 {
        run_agent_turn(&mut s, &rome_agent, &babylon_agent);
    }

    let final_coord = s.state.unit(s.rome_warrior).unwrap().coord;
    assert_ne!(
        final_coord, initial_coord,
        "warrior should have moved away from its starting tile after 3 turns"
    );
}

/// The production-diff emitted by the agent contains the right city ID.
#[test]
fn production_diff_contains_correct_city_id() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let diff = HeuristicAgent::new(s.rome_id).take_turn(&mut s.state, &rules);

    let started: Vec<_> = diff.deltas.iter().filter_map(|d| {
        if let StateDelta::ProductionStarted { city, item } = d {
            Some((*city, *item))
        } else {
            None
        }
    }).collect();

    assert_eq!(started.len(), 1, "exactly one ProductionStarted expected");
    assert_eq!(started[0].0, s.rome_city, "ProductionStarted must reference Rome's city");
}
