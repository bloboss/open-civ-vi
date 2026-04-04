//! Integration tests for the Rise & Fall alliance type system.

mod common;

use libciv::DefaultRulesEngine;
use libciv::game::rules::RulesEngine;
use libciv::civ::diplomacy::AllianceType;
use libciv::civ::DiplomaticStatus;

#[test]
fn form_alliance_sets_status() {
    let s = common::build_scenario();
    let mut state = s.state;
    let rules = DefaultRulesEngine;

    let diff = rules.form_alliance(&mut state, s.rome_id, s.babylon_id, AllianceType::Research)
        .expect("form_alliance should succeed");

    // Status should be Alliance
    let rel = state.diplomatic_relations.iter()
        .find(|r| {
            (r.civ_a == s.rome_id && r.civ_b == s.babylon_id) ||
            (r.civ_a == s.babylon_id && r.civ_b == s.rome_id)
        })
        .expect("relation should exist");
    assert_eq!(rel.status, DiplomaticStatus::Alliance);
    assert_eq!(rel.alliance_type, Some(AllianceType::Research));
    assert_eq!(rel.alliance_level, 1);
    assert_eq!(rel.alliance_turns, 0);

    // Diff should contain AllianceFormed and DiplomacyChanged
    assert!(diff.deltas.iter().any(|d| matches!(d,
        libciv::game::diff::StateDelta::AllianceFormed { alliance_type: AllianceType::Research, .. }
    )));
    assert!(diff.deltas.iter().any(|d| matches!(d,
        libciv::game::diff::StateDelta::DiplomacyChanged { new_status: DiplomaticStatus::Alliance, .. }
    )));
}

#[test]
fn alliance_levels_up_over_time() {
    let s = common::build_scenario();
    let mut state = s.state;
    let rules = DefaultRulesEngine;

    rules.form_alliance(&mut state, s.rome_id, s.babylon_id, AllianceType::Military)
        .expect("form_alliance should succeed");

    // Advance 30 turns to reach level 2
    for _ in 0..30 {
        rules.advance_turn(&mut state);
    }

    let rel = state.diplomatic_relations.iter()
        .find(|r| {
            (r.civ_a == s.rome_id && r.civ_b == s.babylon_id) ||
            (r.civ_a == s.babylon_id && r.civ_b == s.rome_id)
        })
        .expect("relation should exist");
    assert_eq!(rel.alliance_level, 2, "alliance should be level 2 after 30 turns");
    assert_eq!(rel.alliance_turns, 30);

    // Advance 30 more turns to reach level 3
    for _ in 0..30 {
        rules.advance_turn(&mut state);
    }

    let rel = state.diplomatic_relations.iter()
        .find(|r| {
            (r.civ_a == s.rome_id && r.civ_b == s.babylon_id) ||
            (r.civ_a == s.babylon_id && r.civ_b == s.rome_id)
        })
        .expect("relation should exist");
    assert_eq!(rel.alliance_level, 3, "alliance should be level 3 after 60 turns");
    assert_eq!(rel.alliance_turns, 60);
}

#[test]
fn cannot_ally_while_at_war() {
    let s = common::build_scenario();
    let mut state = s.state;
    let rules = DefaultRulesEngine;

    // Declare war first
    rules.declare_war(&mut state, s.rome_id, s.babylon_id)
        .expect("declare_war should succeed");

    // Attempt to form alliance should fail
    let result = rules.form_alliance(&mut state, s.rome_id, s.babylon_id, AllianceType::Economic);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, libciv::game::rules::RulesError::CannotAllyAtWar),
        "expected CannotAllyAtWar, got: {err:?}"
    );
}
