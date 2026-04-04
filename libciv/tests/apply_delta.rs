mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::game::{apply_delta, apply_diff, StateDelta, GameStateDiff};
use libciv::civ::unit::Unit;
use libhexgrid::coord::HexCoord;

// ── Unit movement ────────────────────────────────────────────────────────────

#[test]
fn apply_unit_moved_updates_coord_and_movement() {
    let mut s = common::build_scenario();
    let uid = s.rome_warrior;
    let before = s.state.unit(uid).unwrap().coord();
    let target = HexCoord::from_qr(4, 3);

    let delta = StateDelta::UnitMoved {
        unit: uid,
        from: before,
        to: target,
        cost: 100,
    };
    apply_delta(&mut s.state, &delta);

    let unit = s.state.unit(uid).unwrap();
    assert_eq!(unit.coord(), target, "unit should be at new coord");
    assert_eq!(unit.movement_left(), 100, "movement should be reduced by cost");
}

// ── Unit destruction ─────────────────────────────────────────────────────────

#[test]
fn apply_unit_destroyed_removes_unit() {
    let mut s = common::build_scenario();
    let uid = s.rome_warrior;
    assert!(s.state.unit(uid).is_some());

    apply_delta(&mut s.state, &StateDelta::UnitDestroyed { unit: uid });

    assert!(s.state.unit(uid).is_none(), "unit should be removed from state");
}

// ── Gold changes ─────────────────────────────────────────────────────────────

#[test]
fn apply_gold_changed_adjusts_treasury() {
    let mut s = common::build_scenario();
    let civ_id = s.rome_id;
    let before = s.state.civilizations.iter().find(|c| c.id == civ_id).unwrap().gold;

    apply_delta(&mut s.state, &StateDelta::GoldChanged { civ: civ_id, delta: 42 });

    let after = s.state.civilizations.iter().find(|c| c.id == civ_id).unwrap().gold;
    assert_eq!(after, before + 42);
}

// ── Turn advancement ─────────────────────────────────────────────────────────

#[test]
fn apply_turn_advanced_sets_turn() {
    let mut s = common::build_scenario();
    assert_eq!(s.state.turn, 0);

    apply_delta(&mut s.state, &StateDelta::TurnAdvanced { from: 0, to: 1 });

    assert_eq!(s.state.turn, 1);
}

// ── Victory ──────────────────────────────────────────────────────────────────

#[test]
fn apply_victory_achieved_sets_game_over() {
    let mut s = common::build_scenario();
    assert!(s.state.game_over.is_none());

    apply_delta(&mut s.state, &StateDelta::VictoryAchieved {
        civ: s.rome_id,
        condition: "Score",
    });

    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.winner, s.rome_id);
    assert_eq!(go.condition, "Score");
}

// ── Improvement placement ────────────────────────────────────────────────────

#[test]
fn apply_improvement_placed_sets_tile() {
    use libciv::world::improvement::BuiltinImprovement;
    use libhexgrid::board::HexBoard;

    let mut s = common::build_scenario();
    let coord = s.state.cities[0].coord;

    apply_delta(&mut s.state, &StateDelta::ImprovementPlaced {
        coord,
        improvement: BuiltinImprovement::Farm,
    });

    let tile = s.state.board.tile(coord).unwrap();
    assert_eq!(tile.improvement, Some(BuiltinImprovement::Farm));
}

// ── apply_diff batch ─────────────────────────────────────────────────────────

#[test]
fn apply_diff_processes_multiple_deltas() {
    let mut s = common::build_scenario();

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::GoldChanged { civ: s.rome_id, delta: 10 });
    diff.push(StateDelta::GoldChanged { civ: s.rome_id, delta: 20 });
    diff.push(StateDelta::TurnAdvanced { from: 0, to: 1 });

    apply_diff(&mut s.state, &diff);

    let gold = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap().gold;
    assert_eq!(gold, 30, "both gold deltas should have been applied");
    assert_eq!(s.state.turn, 1);
}

// ── Round-trip: advance turns, collect diffs, verify key fields ──────────────

#[test]
fn round_trip_advance_and_apply_matches() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    // Advance 3 turns, collecting diffs.
    let mut diffs = Vec::new();
    for _ in 0..3 {
        let diff = rules.advance_turn(&mut s.state);
        diffs.push(diff);
        // Reset movement for next turn.
        for u in &mut s.state.units {
            u.movement_left = u.max_movement;
        }
    }

    let final_turn = s.state.turn;
    let final_gold = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap().gold;

    // Create a fresh scenario and apply all diffs.
    let mut s2 = common::build_scenario();
    for diff in &diffs {
        apply_diff(&mut s2.state, diff);
        for u in &mut s2.state.units {
            u.movement_left = u.max_movement;
        }
    }

    assert_eq!(s2.state.turn, final_turn, "turn should match after applying diffs");
    let gold2 = s2.state.civilizations.iter().find(|c| c.id == s2.rome_id).unwrap().gold;
    assert_eq!(gold2, final_gold, "gold should match after applying diffs");
}
