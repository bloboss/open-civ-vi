/// Integration tests for Science Victory.
mod common;

use libciv::{BuiltinVictoryCondition, DefaultRulesEngine, RulesEngine, SCIENCE_MILESTONES};
use libciv::game::StateDelta;
use libciv::game::RulesError;

#[test]
fn complete_all_milestones_wins_science_victory() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    // Register a ScienceVictory condition.
    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Science { id: vc_id });

    // Complete all 3 milestones for Rome.
    for i in 0..SCIENCE_MILESTONES.len() {
        let diff = rules.complete_science_milestone(&mut s.state, s.rome_id)
            .expect("milestone should succeed");
        // Verify the delta contains the expected milestone name.
        let found = diff.deltas.iter().any(|d| {
            matches!(d, StateDelta::ScienceMilestoneCompleted { milestone, .. }
                if *milestone == SCIENCE_MILESTONES[i])
        });
        assert!(found, "expected ScienceMilestoneCompleted delta for {:?}", SCIENCE_MILESTONES[i]);
    }

    // Advance a turn so the victory condition is evaluated.
    let diff = rules.advance_turn(&mut s.state);
    assert!(s.state.game_over.is_some(), "game should be over after completing all milestones");
    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.winner, s.rome_id);
    assert_eq!(go.condition, "Science Victory");

    let has_victory_delta = diff.deltas.iter().any(|d| matches!(d, StateDelta::VictoryAchieved { .. }));
    assert!(has_victory_delta, "expected VictoryAchieved delta");
}

#[test]
fn partial_milestones_do_not_trigger_victory() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Science { id: vc_id });

    // Complete only 2 milestones.
    rules.complete_science_milestone(&mut s.state, s.rome_id).unwrap();
    rules.complete_science_milestone(&mut s.state, s.rome_id).unwrap();

    // Advance a turn -- victory should NOT fire.
    rules.advance_turn(&mut s.state);
    assert!(s.state.game_over.is_none(), "game should not be over with only 2 milestones");
}

#[test]
fn fourth_milestone_returns_error() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    // Complete all 3 milestones.
    for _ in 0..3 {
        rules.complete_science_milestone(&mut s.state, s.rome_id).unwrap();
    }

    // A fourth attempt should fail.
    let err = rules.complete_science_milestone(&mut s.state, s.rome_id);
    assert!(matches!(err, Err(RulesError::AllMilestonesCompleted)),
        "expected AllMilestonesCompleted error, got {:?}", err);
}
