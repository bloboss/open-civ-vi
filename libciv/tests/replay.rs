#![cfg(feature = "serde")]

//! Integration tests for the replay recorder and viewer.

mod common;

use libciv::game::{ReplayRecorder, ReplayViewer, TurnEngine};
use libciv::DefaultRulesEngine;

/// Helper: advance one turn and return the diff (without the movement reset /
/// visibility refresh that `common::advance_turn` does, since we only need the
/// rules-engine diff for replay purposes).
fn advance_and_record(s: &mut common::Scenario) -> libciv::GameStateDiff {
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;
    engine.process_turn(&mut s.state, &rules)
}

#[test]
fn record_and_replay_turns() {
    let mut s = common::build_scenario();
    let mut recorder = ReplayRecorder::new(&s.state).expect("recorder init");

    // Record 5 turns.
    for _ in 0..5 {
        let diff = advance_and_record(&mut s);
        recorder.record_turn(diff);
    }

    assert_eq!(recorder.turn_count(), 5);

    let mut viewer = ReplayViewer::from_recorder(&recorder).expect("viewer init");
    assert_eq!(viewer.total_turns(), 5);
    assert_eq!(viewer.turn(), 0);

    // Step through all turns.
    for expected in 1..=5 {
        assert!(viewer.step_forward());
        assert_eq!(viewer.turn(), expected);
    }

    // Cannot step past the end.
    assert!(!viewer.step_forward());
    assert_eq!(viewer.turn(), 5);
}

#[test]
fn replay_state_matches_original() {
    let mut s = common::build_scenario();
    let mut recorder = ReplayRecorder::new(&s.state).expect("recorder init");

    // Record 3 turns, tracking the turn counter after each.
    let mut expected_turns = Vec::new();
    for _ in 0..3 {
        let diff = advance_and_record(&mut s);
        recorder.record_turn(diff);
        expected_turns.push(s.state.turn);
    }

    let mut viewer = ReplayViewer::from_recorder(&recorder).expect("viewer init");

    // Jump to turn 3 and verify the state's turn field.
    assert!(viewer.jump_to_turn(3));
    let state = viewer.current_state().expect("current_state");
    assert_eq!(state.turn, expected_turns[2]);

    // Also verify turn 1.
    assert!(viewer.jump_to_turn(1));
    let state = viewer.current_state().expect("current_state");
    assert_eq!(state.turn, expected_turns[0]);

    // Turn 0 should be the initial state (turn 0).
    assert!(viewer.jump_to_turn(0));
    let state = viewer.current_state().expect("current_state");
    assert_eq!(state.turn, 0);
}

#[test]
fn step_forward_and_backward() {
    let mut s = common::build_scenario();
    let mut recorder = ReplayRecorder::new(&s.state).expect("recorder init");

    for _ in 0..4 {
        let diff = advance_and_record(&mut s);
        recorder.record_turn(diff);
    }

    let mut viewer = ReplayViewer::from_recorder(&recorder).expect("viewer init");

    // Step forward 3 times.
    assert!(viewer.step_forward());
    assert!(viewer.step_forward());
    assert!(viewer.step_forward());
    assert_eq!(viewer.turn(), 3);

    // Step backward twice.
    assert!(viewer.step_backward());
    assert!(viewer.step_backward());
    assert_eq!(viewer.turn(), 1);

    // Step backward to 0.
    assert!(viewer.step_backward());
    assert_eq!(viewer.turn(), 0);

    // Cannot go below 0.
    assert!(!viewer.step_backward());
    assert_eq!(viewer.turn(), 0);
}

#[test]
fn jump_to_out_of_range_returns_false() {
    let mut s = common::build_scenario();
    let mut recorder = ReplayRecorder::new(&s.state).expect("recorder init");

    let diff = advance_and_record(&mut s);
    recorder.record_turn(diff);

    let mut viewer = ReplayViewer::from_recorder(&recorder).expect("viewer init");

    // Valid jumps.
    assert!(viewer.jump_to_turn(0));
    assert!(viewer.jump_to_turn(1));

    // Out of range.
    assert!(!viewer.jump_to_turn(2));
    assert!(!viewer.jump_to_turn(100));

    // Viewer state unchanged after failed jump.
    assert_eq!(viewer.turn(), 1);
}
