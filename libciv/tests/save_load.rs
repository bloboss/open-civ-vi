#![cfg(feature = "serde")]

//! Save/load round-trip tests.

mod common;

use libciv::game::save_load::{save_game, load_game};

#[test]
fn round_trip_empty_game() {
    let state = libciv::GameState::new(42, 14, 8);
    let json = save_game(&state).expect("save should succeed");
    let loaded = load_game(&json).expect("load should succeed");

    assert_eq!(loaded.turn, state.turn);
    assert_eq!(loaded.seed, state.seed);
}

#[test]
fn round_trip_with_civs() {
    let s = common::build_scenario();

    let json = save_game(&s.state).expect("save should succeed");
    let loaded = load_game(&json).expect("load should succeed");

    assert_eq!(loaded.turn, s.state.turn);
    assert_eq!(loaded.seed, s.state.seed);
    assert_eq!(loaded.civilizations.len(), s.state.civilizations.len());
    assert_eq!(loaded.cities.len(), s.state.cities.len());
    assert_eq!(loaded.units.len(), s.state.units.len());

    // Verify civ IDs survived the round trip.
    let loaded_civ_ids: Vec<_> = loaded.civilizations.iter().map(|c| c.id).collect();
    assert!(loaded_civ_ids.contains(&s.rome_id));
    assert!(loaded_civ_ids.contains(&s.babylon_id));
}

#[test]
fn round_trip_after_turns() {
    let mut s = common::build_scenario();

    // Advance a few turns.
    common::advance_turn(&mut s);
    common::advance_turn(&mut s);
    common::advance_turn(&mut s);

    let json = save_game(&s.state).expect("save should succeed");
    let loaded = load_game(&json).expect("load should succeed");

    assert_eq!(loaded.turn, s.state.turn);
    assert_eq!(loaded.seed, s.state.seed);
    // Tech tree should be rebuilt (has nodes from seed initialization).
    assert!(!loaded.tech_tree.nodes.is_empty(), "tech tree should be rebuilt from seed");
}
