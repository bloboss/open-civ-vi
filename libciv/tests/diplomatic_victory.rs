/// Integration tests for Diplomatic Victory.
mod common;

use libciv::{BuiltinVictoryCondition, DefaultRulesEngine, RulesEngine};
use libciv::game::StateDelta;

#[test]
fn diplomatic_favor_accumulates_each_turn() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    // No victory condition needed to test accumulation.
    let initial_favor = s.state.civ(s.rome_id).unwrap().diplomatic_favor;
    assert_eq!(initial_favor, 0, "favor should start at 0");

    let diff = rules.advance_turn(&mut s.state);

    let rome_favor = s.state.civ(s.rome_id).unwrap().diplomatic_favor;
    // Base +1, not at war +1 = at least 2 per turn.
    assert!(rome_favor >= 2, "expected at least 2 favor after 1 turn, got {rome_favor}");

    // Verify a DiplomaticFavorChanged delta was emitted.
    let has_favor_delta = diff.deltas.iter().any(|d| {
        matches!(d, StateDelta::DiplomaticFavorChanged { civ, .. } if *civ == s.rome_id)
    });
    assert!(has_favor_delta, "expected DiplomaticFavorChanged delta for Rome");
}

#[test]
fn diplomatic_victory_fires_at_threshold() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    let vc_id = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Diplomatic { id: vc_id, threshold: 10 });

    // Manually set Rome's diplomatic victory points to the threshold.
    s.state.world_congress.diplomatic_victory_points.insert(s.rome_id, 10);

    // Advance one turn so the victory check fires.
    rules.advance_turn(&mut s.state);

    assert!(s.state.game_over.is_some(), "game should be over once VP >= threshold");
    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.condition, "Diplomatic Victory");
}

#[test]
fn war_reduces_favor_gain() {
    let rules = DefaultRulesEngine;
    let mut s = common::build_scenario();

    // Put Rome and Babylon at war.
    rules.declare_war(&mut s.state, s.rome_id, s.babylon_id).unwrap();

    rules.advance_turn(&mut s.state);

    let rome_favor = s.state.civ(s.rome_id).unwrap().diplomatic_favor;
    // At war: base +1 only (no peace bonus). Could be higher if suzeraining city-states,
    // but in the default scenario there are none, so expect exactly 1.
    assert_eq!(rome_favor, 1, "expected 1 favor while at war, got {rome_favor}");
}
