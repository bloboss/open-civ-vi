/// Integration tests for the tourism mechanics:
/// - Lifetime culture accumulation
/// - Tourism generation from wonders
/// - Cultural dominance detection
/// - CultureVictory condition
mod common;

use libciv::{BuiltinVictoryCondition, CivId, DefaultRulesEngine, GameStateDiff};
use libciv::civ::tourism::{
    compute_tourism, domestic_tourists, has_cultural_dominance, WonderTourism,
};
use libciv::game::{RulesEngine, StateDelta};

// ---------------------------------------------------------------------------
// Lifetime culture accumulation
// ---------------------------------------------------------------------------

#[test]
fn lifetime_culture_accumulates_each_turn() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Both civs start with 0 lifetime culture.
    assert_eq!(s.state.civ(s.rome_id).unwrap().lifetime_culture, 0);

    // Advance one turn — each city produces at least 1 base culture.
    rules.advance_turn(&mut s.state);

    let rome_lc = s.state.civ(s.rome_id).unwrap().lifetime_culture;
    assert!(rome_lc > 0, "Rome should have accumulated lifetime culture, got {rome_lc}");
}

#[test]
fn lifetime_culture_grows_over_multiple_turns() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    rules.advance_turn(&mut s.state);
    let after_1 = s.state.civ(s.rome_id).unwrap().lifetime_culture;

    rules.advance_turn(&mut s.state);
    let after_2 = s.state.civ(s.rome_id).unwrap().lifetime_culture;

    assert!(after_2 > after_1, "lifetime culture should grow: {after_1} -> {after_2}");
}

// ---------------------------------------------------------------------------
// Tourism computation
// ---------------------------------------------------------------------------

#[test]
fn compute_tourism_zero_without_wonders() {
    let s = common::build_scenario();
    assert_eq!(compute_tourism(&s.state, s.rome_id), 0);
}

#[test]
fn compute_tourism_includes_wonder_tourism() {
    let mut s = common::build_scenario();

    // Register a dummy wonder that generates 5 tourism/turn for Rome.
    s.state.wonder_tourism.push(WonderTourism {
        name: "Colosseum",
        owner: s.rome_id,
        tourism_per_turn: 5,
    });

    assert_eq!(compute_tourism(&s.state, s.rome_id), 5);
    // Babylon has no wonders → 0 tourism.
    assert_eq!(compute_tourism(&s.state, s.babylon_id), 0);
}

#[test]
fn multiple_wonders_sum_tourism() {
    let mut s = common::build_scenario();

    s.state.wonder_tourism.push(WonderTourism {
        name: "Colosseum",
        owner: s.rome_id,
        tourism_per_turn: 5,
    });
    s.state.wonder_tourism.push(WonderTourism {
        name: "Great Library",
        owner: s.rome_id,
        tourism_per_turn: 3,
    });

    assert_eq!(compute_tourism(&s.state, s.rome_id), 8);
}

// ---------------------------------------------------------------------------
// Tourism distribution in advance_turn
// ---------------------------------------------------------------------------

#[test]
fn tourism_accumulated_grows_with_wonders() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Rome a wonder generating 10 tourism/turn.
    s.state.wonder_tourism.push(WonderTourism {
        name: "Colosseum",
        owner: s.rome_id,
        tourism_per_turn: 10,
    });

    rules.advance_turn(&mut s.state);

    let rome = s.state.civ(s.rome_id).unwrap();
    let tourism_toward_babylon = rome.tourism_accumulated
        .get(&s.babylon_id).copied().unwrap_or(0);
    assert_eq!(tourism_toward_babylon, 10,
        "Rome should have pushed 10 tourism toward Babylon");

    // Babylon has no wonders, so no tourism toward Rome.
    let babylon = s.state.civ(s.babylon_id).unwrap();
    let tourism_toward_rome = babylon.tourism_accumulated
        .get(&s.rome_id).copied().unwrap_or(0);
    assert_eq!(tourism_toward_rome, 0);
}

#[test]
fn tourism_generated_delta_emitted() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    s.state.wonder_tourism.push(WonderTourism {
        name: "Colosseum",
        owner: s.rome_id,
        tourism_per_turn: 7,
    });

    let diff = rules.advance_turn(&mut s.state);

    let tourism_deltas: Vec<_> = diff.deltas.iter().filter(|d| {
        matches!(d, StateDelta::TourismGenerated { civ, tourism, .. }
            if *civ == s.rome_id && *tourism == 7)
    }).collect();

    assert_eq!(tourism_deltas.len(), 1, "should emit TourismGenerated for Rome");
}

// ---------------------------------------------------------------------------
// Domestic tourists
// ---------------------------------------------------------------------------

#[test]
fn domestic_tourists_formula() {
    let mut s = common::build_scenario();

    // 0 culture → 0 domestic tourists.
    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(domestic_tourists(rome), 0);

    // Manually set lifetime culture.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .lifetime_culture = 350;

    let rome = s.state.civ(s.rome_id).unwrap();
    assert_eq!(domestic_tourists(rome), 3);
}

// ---------------------------------------------------------------------------
// Cultural dominance
// ---------------------------------------------------------------------------

#[test]
fn cultural_dominance_not_achieved_without_tourism() {
    let s = common::build_scenario();
    assert!(!has_cultural_dominance(&s.state, s.rome_id));
}

#[test]
fn cultural_dominance_achieved_when_tourism_exceeds_culture() {
    let mut s = common::build_scenario();

    // Babylon has 200 lifetime culture → 2 domestic tourists.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .lifetime_culture = 200;

    // Rome sends 3 tourism toward Babylon (> 2 domestic tourists).
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .tourism_accumulated.insert(s.babylon_id, 3);

    assert!(has_cultural_dominance(&s.state, s.rome_id));
}

#[test]
fn cultural_dominance_fails_if_tied() {
    let mut s = common::build_scenario();

    // Babylon: 200 culture → 2 domestic tourists.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .lifetime_culture = 200;

    // Rome sends exactly 2 — tie, not enough.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .tourism_accumulated.insert(s.babylon_id, 2);

    assert!(!has_cultural_dominance(&s.state, s.rome_id));
}

// ---------------------------------------------------------------------------
// CultureVictory condition
// ---------------------------------------------------------------------------

#[test]
fn culture_victory_not_won_without_dominance() {
    let mut s = common::build_scenario();
    let vid = s.state.id_gen.next_victory_id();
    let cv = BuiltinVictoryCondition::Culture { id: vid };

    let progress = cv.check_progress(s.rome_id, &s.state);
    assert!(!progress.is_won());
    assert_eq!(progress.current, 0);
    assert_eq!(progress.target, 1); // 1 other civ
}

#[test]
fn culture_victory_won_with_dominance() {
    let mut s = common::build_scenario();

    // Give Babylon some culture so the victory is meaningful.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .lifetime_culture = 100; // → 1 domestic tourist

    // Rome pushes 2 tourism toward Babylon (> 1).
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .tourism_accumulated.insert(s.babylon_id, 2);

    let vid = s.state.id_gen.next_victory_id();
    let cv = BuiltinVictoryCondition::Culture { id: vid };

    let progress = cv.check_progress(s.rome_id, &s.state);
    assert!(progress.is_won());
    assert_eq!(progress.current, 1);
    assert_eq!(progress.target, 1);
}

#[test]
fn culture_victory_triggers_game_over_in_advance_turn() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Set up dominance: Babylon has 100 culture (1 domestic tourist),
    // Rome has 2 tourism accumulated toward Babylon.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .lifetime_culture = 100;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .tourism_accumulated.insert(s.babylon_id, 2);

    // Register a CultureVictory condition.
    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Culture { id: vid });

    let diff = rules.advance_turn(&mut s.state);

    assert!(s.state.game_over.is_some(), "game should be over");
    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.winner, s.rome_id);
    assert_eq!(go.condition, "Culture Victory");

    let victory_delta = diff.deltas.iter().any(|d| {
        matches!(d, StateDelta::VictoryAchieved { civ, condition }
            if *civ == s.rome_id && *condition == "Culture Victory")
    });
    assert!(victory_delta, "VictoryAchieved delta should be emitted");
}

#[test]
fn culture_victory_end_to_end_via_tourism_accumulation() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Rome a very high tourism wonder.
    s.state.wonder_tourism.push(WonderTourism {
        name: "Eiffel Tower",
        owner: s.rome_id,
        tourism_per_turn: 200,
    });

    // Register culture victory.
    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(BuiltinVictoryCondition::Culture { id: vid });

    // Advance several turns — Babylon accumulates culture each turn (defense),
    // but Rome pushes 200 tourism/turn which should overwhelm it.
    for _ in 0..20 {
        if s.state.game_over.is_some() { break; }
        rules.advance_turn(&mut s.state);
    }

    assert!(s.state.game_over.is_some(),
        "Rome should have won culture victory within 20 turns");
    assert_eq!(s.state.game_over.as_ref().unwrap().winner, s.rome_id);
    assert_eq!(s.state.game_over.as_ref().unwrap().condition, "Culture Victory");
}
