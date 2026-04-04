/// Integration tests for the victory condition infrastructure.
mod common;

use libciv::{
    all_scores, compute_score,
    BuiltinVictoryCondition, DefaultRulesEngine, RulesEngine,
};
use libciv::game::StateDelta;

// ── Score computation ─────────────────────────────────────────────────────────

#[test]
fn score_increases_with_cities_and_techs() {
    let s = common::build_scenario();
    let initial = compute_score(&s.state, s.rome_id);
    // Rome starts with 1 city (pop 1) + territory tiles. Score > 0.
    assert!(initial > 0, "expected non-zero initial score, got {initial}");
}

#[test]
fn all_scores_returns_both_civs_sorted() {
    let s = common::build_scenario();
    let scores = all_scores(&s.state);
    assert_eq!(scores.len(), 2, "expected 2 civs in leaderboard");
    // Scores must be in descending order.
    assert!(scores[0].1 >= scores[1].1);
}

#[test]
fn scores_cover_both_civ_ids() {
    let s = common::build_scenario();
    let scores = all_scores(&s.state);
    let ids: Vec<_> = scores.iter().map(|(id, _)| *id).collect();
    assert!(ids.contains(&s.rome_id));
    assert!(ids.contains(&s.babylon_id));
}

// ── ScoreVictory — turn limit ─────────────────────────────────────────────────

#[test]
fn score_victory_fires_at_turn_limit() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    // Register a ScoreVictory that expires at turn 3.
    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Score { id: vc_id, turn_limit: 3 });

    // Advance 2 turns — game should still be running.
    rules.advance_turn(&mut s.state);
    rules.advance_turn(&mut s.state);
    assert!(s.state.game_over.is_none(), "game should not be over before turn limit");

    // Advance 1 more — turn counter reaches 3, victory should fire.
    let diff = rules.advance_turn(&mut s.state);
    assert!(s.state.game_over.is_some(), "game should be over at turn limit");

    let victory_delta = diff.deltas.iter().any(|d| matches!(d, StateDelta::VictoryAchieved { .. }));
    assert!(victory_delta, "expected VictoryAchieved delta in diff");
}

#[test]
fn score_victory_winner_is_highest_scorer() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Score { id: vc_id, turn_limit: 3 });

    rules.advance_turn(&mut s.state);
    rules.advance_turn(&mut s.state);
    rules.advance_turn(&mut s.state);

    let go = s.state.game_over.as_ref().expect("game should be over");
    // The winner should be whoever has the highest score.
    let scores = all_scores(&s.state);
    let expected_winner = scores[0].0;
    assert_eq!(go.winner, expected_winner);
    assert_eq!(go.condition, "Score Victory");
}

#[test]
fn score_victory_condition_name_in_delta() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Score { id: vc_id, turn_limit: 1 });

    let diff = rules.advance_turn(&mut s.state);
    let delta = diff.deltas.iter().find_map(|d| {
        if let StateDelta::VictoryAchieved { condition, .. } = d { Some(*condition) } else { None }
    });
    assert_eq!(delta, Some("Score Victory"));
}

#[test]
fn no_victory_conditions_game_never_ends() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    // No victory conditions registered.
    for _ in 0..10 {
        rules.advance_turn(&mut s.state);
    }
    assert!(s.state.game_over.is_none(), "no conditions means no game over");
}

#[test]
fn game_over_blocks_further_victory_evaluation() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Score { id: vc_id, turn_limit: 1 });

    // First evaluation fires the victory.
    rules.advance_turn(&mut s.state);
    let winner_after_first = s.state.game_over.as_ref().map(|g| g.winner);

    // Second advance should not change the winner.
    rules.advance_turn(&mut s.state);
    let winner_after_second = s.state.game_over.as_ref().map(|g| g.winner);

    assert_eq!(winner_after_first, winner_after_second, "winner should not change once game is over");
}
