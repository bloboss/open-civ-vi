#![cfg(feature = "serde")]

//! Serialization round-trip tests for `GameStateDiff` and `StateDelta`.
//!
//! These tests verify that semantic diffs can be serialized to JSON and
//! deserialized back without loss, enabling action trace logs and replay.

mod common;

use libciv::game::diff::{AttackType, GameStateDiff, StateDelta};
use libciv::game::{ReplayRecorder, ReplayViewer, TurnEngine};
use libciv::{DefaultRulesEngine, RulesEngine};
use libhexgrid::coord::HexCoord;

/// Serialize a diff to JSON and deserialize it back.
///
/// Leaks the JSON string to `'static` because `StateDelta` fields use
/// `&'static str` (via `serde_static_str`). This is fine in tests.
fn round_trip(diff: &GameStateDiff) -> GameStateDiff {
    let json = serde_json::to_string(diff).expect("serialize diff");
    let leaked: &'static str = Box::leak(json.into_boxed_str());
    serde_json::from_str(leaked).expect("deserialize diff")
}

#[test]
fn empty_diff_round_trip() {
    let diff = GameStateDiff::new();
    let rt = round_trip(&diff);
    assert!(rt.is_empty());
}

#[test]
fn turn_advanced_round_trip() {
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TurnAdvanced { from: 0, to: 1 });

    let rt = round_trip(&diff);
    assert_eq!(rt.len(), 1);
    match &rt.deltas[0] {
        StateDelta::TurnAdvanced { from, to } => {
            assert_eq!(*from, 0);
            assert_eq!(*to, 1);
        }
        other => panic!("expected TurnAdvanced, got {other:?}"),
    }
}

#[test]
fn unit_moved_round_trip() {
    let s = common::build_scenario();
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::UnitMoved {
        unit: s.rome_warrior,
        from: HexCoord::from_qr(5, 3),
        to: HexCoord::from_qr(6, 3),
        cost: 100,
    });

    let rt = round_trip(&diff);
    assert_eq!(rt.len(), 1);
    match &rt.deltas[0] {
        StateDelta::UnitMoved { unit, from, to, cost } => {
            assert_eq!(*unit, s.rome_warrior);
            assert_eq!(from.q, 5);
            assert_eq!(to.q, 6);
            assert_eq!(*cost, 100);
        }
        other => panic!("expected UnitMoved, got {other:?}"),
    }
}

#[test]
fn static_str_fields_survive_round_trip() {
    let s = common::build_scenario();
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TechResearched {
        civ: s.rome_id,
        tech: "Pottery",
    });
    diff.push(StateDelta::BuildingUnlocked {
        civ: s.rome_id,
        building: "Library",
    });
    diff.push(StateDelta::CivicCompleted {
        civ: s.babylon_id,
        civic: "Code of Laws",
    });

    let rt = round_trip(&diff);
    assert_eq!(rt.len(), 3);

    match &rt.deltas[0] {
        StateDelta::TechResearched { tech, .. } => assert_eq!(*tech, "Pottery"),
        other => panic!("expected TechResearched, got {other:?}"),
    }
    match &rt.deltas[1] {
        StateDelta::BuildingUnlocked { building, .. } => assert_eq!(*building, "Library"),
        other => panic!("expected BuildingUnlocked, got {other:?}"),
    }
    match &rt.deltas[2] {
        StateDelta::CivicCompleted { civic, .. } => assert_eq!(*civic, "Code of Laws"),
        other => panic!("expected CivicCompleted, got {other:?}"),
    }
}

#[test]
fn combat_delta_round_trip() {
    let s = common::build_scenario();
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::UnitAttacked {
        attacker: s.rome_warrior,
        defender: s.babylon_warrior,
        attack_type: AttackType::Melee,
        attacker_damage: 15,
        defender_damage: 25,
    });
    diff.push(StateDelta::ExperienceGained {
        unit: s.rome_warrior,
        amount: 5,
        new_total: 5,
    });

    let rt = round_trip(&diff);
    assert_eq!(rt.len(), 2);
    match &rt.deltas[0] {
        StateDelta::UnitAttacked {
            attacker,
            defender,
            attack_type,
            attacker_damage,
            defender_damage,
        } => {
            assert_eq!(*attacker, s.rome_warrior);
            assert_eq!(*defender, s.babylon_warrior);
            assert_eq!(*attack_type, AttackType::Melee);
            assert_eq!(*attacker_damage, 15);
            assert_eq!(*defender_damage, 25);
        }
        other => panic!("expected UnitAttacked, got {other:?}"),
    }
}

#[test]
fn mixed_deltas_round_trip() {
    let s = common::build_scenario();
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TurnAdvanced { from: 0, to: 1 });
    diff.push(StateDelta::GoldChanged {
        civ: s.rome_id,
        delta: 42,
    });
    diff.push(StateDelta::PopulationGrew {
        city: s.rome_city,
        new_population: 3,
    });
    diff.push(StateDelta::CityFounded {
        city: s.babylon_city,
        coord: HexCoord::from_qr(10, 5),
        owner: s.babylon_id,
    });
    diff.push(StateDelta::UnitCreated {
        unit: s.rome_warrior,
        coord: HexCoord::from_qr(3, 3),
        owner: s.rome_id,
    });
    diff.push(StateDelta::UnitDestroyed {
        unit: s.babylon_warrior,
    });
    diff.push(StateDelta::TilesRevealed {
        civ: s.rome_id,
        coords: vec![HexCoord::from_qr(1, 1), HexCoord::from_qr(2, 2)],
    });

    let rt = round_trip(&diff);
    assert_eq!(rt.len(), 7);

    // Spot-check a few variants.
    match &rt.deltas[1] {
        StateDelta::GoldChanged { civ, delta } => {
            assert_eq!(*civ, s.rome_id);
            assert_eq!(*delta, 42);
        }
        other => panic!("expected GoldChanged, got {other:?}"),
    }
    match &rt.deltas[6] {
        StateDelta::TilesRevealed { coords, .. } => {
            assert_eq!(coords.len(), 2);
        }
        other => panic!("expected TilesRevealed, got {other:?}"),
    }
}

#[test]
fn advance_turn_diff_round_trips() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    let diff = rules.advance_turn(&mut s.state);

    let rt = round_trip(&diff);
    assert_eq!(rt.len(), diff.len());
}

#[test]
fn move_unit_diff_round_trips() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    let diff = rules
        .move_unit(&mut s.state, s.rome_warrior, HexCoord::from_qr(6, 3))
        .expect("move should succeed");

    let rt = round_trip(&diff);
    assert_eq!(rt.len(), diff.len());

    // Verify the UnitMoved delta survived.
    assert!(rt.deltas.iter().any(|d| matches!(d, StateDelta::UnitMoved { .. })));
}

#[test]
fn jsonl_log_format() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Simulate a multi-action log: each diff is one JSON line.
    let mut log_lines: Vec<&'static str> = Vec::new();

    let diff1 = rules
        .move_unit(&mut s.state, s.rome_warrior, HexCoord::from_qr(6, 3))
        .expect("move");
    log_lines.push(Box::leak(
        serde_json::to_string(&diff1).expect("serialize").into_boxed_str(),
    ));

    // Apply the move so state is consistent.
    libciv::apply_diff(&mut s.state, &diff1);

    let diff2 = rules.advance_turn(&mut s.state);
    log_lines.push(Box::leak(
        serde_json::to_string(&diff2).expect("serialize").into_boxed_str(),
    ));

    // Each line should be independently parseable.
    for (i, line) in log_lines.iter().enumerate() {
        let parsed: GameStateDiff =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line {i} failed: {e}"));
        assert!(!parsed.is_empty(), "line {i} should have deltas");
    }
}

#[test]
fn replay_recorder_diffs_serialize() {
    let mut s = common::build_scenario();
    let mut recorder = ReplayRecorder::new(&s.state).expect("recorder init");
    let rules = DefaultRulesEngine;

    // Record several turns with real game actions.
    for _ in 0..3 {
        let engine = TurnEngine::new();
        let diff = engine.process_turn(&mut s.state, &rules);
        recorder.record_turn(diff);
    }

    // Serialize each recorded diff individually.
    for (i, diff) in recorder.turn_diffs.iter().enumerate() {
        let rt = round_trip(diff);
        assert_eq!(rt.len(), diff.len(), "turn {i} delta count mismatch");
    }

    // Also verify the viewer still works after round-tripping.
    let mut viewer = ReplayViewer::from_recorder(&recorder).expect("viewer init");
    viewer.step_forward();
    let state = viewer.current_state().expect("current_state");
    assert_eq!(state.turn, 1);
}
