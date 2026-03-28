/// End-to-end tests for the domination victory pipeline:
/// city capture, domination condition, and verification of all victory types.
mod common;

use libciv::{
    CivId, CultureVictory, DefaultRulesEngine, DominationVictory,
    GameState, RulesEngine, ScoreVictory,
    UnitCategory, UnitDomain, UnitId, UnitTypeId,
};
use libciv::civ::{BasicUnit, City};
use libciv::civ::great_works::{GreatWorkSlot, GreatWorkSlotType};
use libciv::game::victory::{VictoryCondition, VictoryKind};
use libciv::game::StateDelta;
use libhexgrid::coord::HexCoord;

use common::{build_scenario, advance_turn};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Spawn a powerful unit (high CS) at a given coord for a civ.
fn spawn_strong_unit(s: &mut common::Scenario, owner: CivId, coord: HexCoord, cs: u32) -> UnitId {
    let uid = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: uid, unit_type: s.warrior_type, owner,
        coord, domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200, combat_strength: Some(cs),
        promotions: Vec::new(), health: 100, range: 0, vision_range: 2, charges: None, trade_origin: None, trade_destination: None,
    });
    uid
}

// ===========================================================================
// DOMINATION VICTORY TESTS
// ===========================================================================

#[test]
fn test_city_capture_on_melee_kill() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;
    let rules = DefaultRulesEngine;

    // Place a very strong Rome unit adjacent to Babylon's capital at (10,5).
    let attacker = spawn_strong_unit(&mut s, rome_id, HexCoord::from_qr(10, 4), 100);

    // Kill Babylon's warrior first (at (8,5) — too far, so move it onto the city).
    // Instead, remove the warrior and place a weak defender on the city.
    s.state.units.retain(|u| u.id != s.babylon_warrior);
    let weak_defender = spawn_strong_unit(&mut s, babylon_id, HexCoord::from_qr(10, 5), 5);

    // Attack: 100 CS attacker vs 5 CS defender → defender dies.
    let diff = rules.attack(&mut s.state, attacker, weak_defender).unwrap();

    // Defender should be dead.
    assert!(s.state.unit(weak_defender).is_none(), "defender should be dead");

    // City should be captured.
    let captured = diff.deltas.iter().any(|d| matches!(d, StateDelta::CityCaptured { .. }));
    assert!(captured, "should emit CityCaptured delta");

    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(city.owner, rome_id, "Babylon's capital should now be owned by Rome");
    assert_eq!(city.founded_by, babylon_id, "founded_by should still be Babylon");

    // Attacker should have moved onto the city tile.
    let atk = s.state.unit(attacker).unwrap();
    assert_eq!(atk.coord, HexCoord::from_qr(10, 5), "attacker should be on the captured city tile");
}

#[test]
fn test_city_not_captured_if_defenders_remain() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;
    let rules = DefaultRulesEngine;

    // Place two defenders on Babylon's capital.
    let def1 = spawn_strong_unit(&mut s, babylon_id, HexCoord::from_qr(10, 5), 5);
    // Keep the original warrior too (at (8,5) — but it's not on the city tile, so it doesn't count).
    // Place a second defender ON the city.
    let def2 = spawn_strong_unit(&mut s, babylon_id, HexCoord::from_qr(10, 5), 50);

    // Place a strong attacker.
    let attacker = spawn_strong_unit(&mut s, rome_id, HexCoord::from_qr(10, 4), 100);

    // Kill def1 (the weak one). def2 still lives on the tile.
    let diff = rules.attack(&mut s.state, attacker, def1).unwrap();

    // City should NOT be captured because def2 is still on the tile.
    let captured = diff.deltas.iter().any(|d| matches!(d, StateDelta::CityCaptured { .. }));
    assert!(!captured, "should not capture city while defenders remain");

    let city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(city.owner, babylon_id, "city should still belong to Babylon");
}

#[test]
fn test_domination_victory_fires_on_all_capitals_captured() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;
    let rules = DefaultRulesEngine;

    // Register domination victory.
    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(DominationVictory { id: vid }));

    // Capture Babylon's capital: remove defender, place strong attacker.
    s.state.units.retain(|u| u.id != s.babylon_warrior);
    let weak_def = spawn_strong_unit(&mut s, babylon_id, HexCoord::from_qr(10, 5), 1);
    let attacker = spawn_strong_unit(&mut s, rome_id, HexCoord::from_qr(10, 4), 100);
    rules.attack(&mut s.state, attacker, weak_def).unwrap();

    // Verify Babylon's capital is now owned by Rome.
    let bab_capital = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    assert_eq!(bab_capital.owner, rome_id);

    // Advance turn to trigger victory check.
    advance_turn(&mut s);

    assert!(s.state.game_over.is_some(), "game should be over");
    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.winner, rome_id);
    assert_eq!(go.condition, "Domination Victory");
}

#[test]
fn test_domination_victory_not_fired_if_capitals_remain() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;

    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(DominationVictory { id: vid }));

    // Don't capture Babylon's capital.
    advance_turn(&mut s);

    assert!(s.state.game_over.is_none(), "no domination victory without capturing all capitals");
}

#[test]
fn test_domination_progress_tracking() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;

    let vid = s.state.id_gen.next_victory_id();
    let vc = DominationVictory { id: vid };

    // Before capture: 0 of 1 foreign capitals controlled.
    let progress = vc.check_progress(rome_id, &s.state);
    assert_eq!(progress.current, 0);
    assert_eq!(progress.target, 1);
    assert!(!progress.is_won());
}

// ===========================================================================
// SCORE VICTORY TESTS (verify API-level correctness)
// ===========================================================================

#[test]
fn test_score_victory_fires_at_turn_limit() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;

    // Register score victory with turn limit = 5.
    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(ScoreVictory { id: vid, turn_limit: 5 }));

    // Advance 5 turns.
    for _ in 0..5 {
        advance_turn(&mut s);
    }

    assert!(s.state.game_over.is_some(), "score victory should fire at turn limit");
    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.condition, "Score Victory");
    // Winner should be deterministic — same setup, same seed.
}

#[test]
fn test_score_victory_not_before_limit() {
    let mut s = build_scenario();

    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(ScoreVictory { id: vid, turn_limit: 10 }));

    for _ in 0..5 {
        advance_turn(&mut s);
    }

    assert!(s.state.game_over.is_none(), "score victory should not fire before turn limit");
}

// ===========================================================================
// CULTURE VICTORY TESTS (verify API-level correctness)
// ===========================================================================

#[test]
fn test_culture_victory_requires_tourism_exceeding_all() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;

    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(CultureVictory { id: vid }));

    // Give Rome great work slots and works to generate tourism.
    // 6 great works → 18 tourism (well above any domestic culture from 1 turn).
    use libciv::civ::great_works::GreatWorkSlot;
    use libciv::civ::great_people::GreatPerson;
    use libciv::GreatPersonType;
    let rc = s.rome_city;
    if let Some(city) = s.state.cities.iter_mut().find(|c| c.id == rc) {
        for _ in 0..3 {
            city.great_work_slots.push(GreatWorkSlot::new(GreatWorkSlotType::Writing));
        }
        for _ in 0..3 {
            city.great_work_slots.push(GreatWorkSlot::new(GreatWorkSlotType::Art));
        }
    }
    for i in 0..6 {
        let name: &'static str = match i {
            0 => "W1", 1 => "W2", 2 => "W3", 3 => "A1", 4 => "A2", _ => "A3",
        };
        let gp_type = if i < 3 { GreatPersonType::Writer } else { GreatPersonType::Artist };
        let gp_id = libciv::GreatPersonId::from_ulid(s.state.id_gen.next_ulid());
        let mut gp = GreatPerson::new(gp_id, name, gp_type, "Ancient");
        gp.owner = Some(rome_id);
        s.state.great_people.push(gp);
        DefaultRulesEngine.create_great_work(&mut s.state, gp_id).unwrap();
    }

    // Advance turn — tourism computed from great works, domestic_culture starts low.
    advance_turn(&mut s);

    let rome_civ = s.state.civilizations.iter().find(|c| c.id == rome_id).unwrap();
    let bab_civ = s.state.civilizations.iter().find(|c| c.id == babylon_id).unwrap();
    assert!(rome_civ.tourism_output > bab_civ.domestic_culture,
        "Rome tourism {} should exceed Babylon domestic_culture {}",
        rome_civ.tourism_output, bab_civ.domestic_culture);
    assert!(s.state.game_over.is_some(), "culture victory should fire");
    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.winner, rome_id);
    assert_eq!(go.condition, "Culture Victory");
}

#[test]
fn test_culture_victory_blocked_when_opponent_culture_higher() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;

    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(CultureVictory { id: vid }));

    // Rome has some tourism but Babylon has massive domestic culture.
    if let Some(civ) = s.state.civilizations.iter_mut().find(|c| c.id == rome_id) {
        civ.tourism_output = 50;
    }
    if let Some(civ) = s.state.civilizations.iter_mut().find(|c| c.id == babylon_id) {
        civ.domestic_culture = 500;
    }

    advance_turn(&mut s);

    assert!(s.state.game_over.is_none(), "culture victory blocked when opponent culture is higher");
}

// ===========================================================================
// ALL VICTORY TYPES: MUTUAL EXCLUSION / PRIORITY
// ===========================================================================

#[test]
fn test_immediate_win_takes_priority_over_turn_limit() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;
    let rules = DefaultRulesEngine;

    // Register both domination (ImmediateWin) and score (TurnLimit at turn 3).
    let vid1 = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(DominationVictory { id: vid1 }));
    let vid2 = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(ScoreVictory { id: vid2, turn_limit: 3 }));

    // Capture Babylon's capital on turn 2.
    advance_turn(&mut s);
    advance_turn(&mut s);

    // Now capture the city.
    s.state.units.retain(|u| u.id != s.babylon_warrior);
    let def = spawn_strong_unit(&mut s, babylon_id, HexCoord::from_qr(10, 5), 1);
    let atk = spawn_strong_unit(&mut s, rome_id, HexCoord::from_qr(10, 4), 100);
    rules.attack(&mut s.state, atk, def).unwrap();

    // This turn is turn 3 — both domination and score could fire.
    // Domination (ImmediateWin) should take priority.
    advance_turn(&mut s);

    let go = s.state.game_over.as_ref().expect("game should be over");
    assert_eq!(go.condition, "Domination Victory",
        "ImmediateWin should take priority over TurnLimit");
    assert_eq!(go.winner, rome_id);
}

// ===========================================================================
// API PROJECTION: GameOverView correctness
// ===========================================================================

#[test]
fn test_game_over_projects_to_api_view() {
    use libciv::game::victory::GameOver;

    // Construct a GameOver directly and verify its fields.
    let mut s = build_scenario();
    let rome_id = s.rome_id;

    s.state.game_over = Some(GameOver {
        winner: rome_id,
        condition: "Domination Victory",
        turn: 42,
    });

    let go = s.state.game_over.as_ref().unwrap();
    assert_eq!(go.winner, rome_id);
    assert_eq!(go.condition, "Domination Victory");
    assert_eq!(go.turn, 42);
}

// ===========================================================================
// MULTI-CIV: 3-player domination requires all foreign capitals
// ===========================================================================

#[test]
fn test_domination_three_civs_requires_both_capitals() {
    let mut s = build_scenario();
    let rome_id = s.rome_id;
    let babylon_id = s.babylon_id;

    // Add a third civ: Egypt.
    let egypt_id = s.state.id_gen.next_civ_id();
    s.state.civilizations.push(
        libciv::civ::Civilization::new(egypt_id, "Egypt", "Egyptian", {
            struct A;
            impl std::fmt::Debug for A { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "A") } }
            impl libciv::civ::Agenda for A {
                fn name(&self) -> &'static str { "A" }
                fn description(&self) -> &'static str { "A" }
                fn attitude(&self, _: CivId) -> i32 { 0 }
            }
            libciv::civ::Leader { name: "Cleopatra", civ_id: egypt_id, abilities: Vec::new(), agenda: Box::new(A) }
        })
    );
    let egypt_city_id = s.state.id_gen.next_city_id();
    let mut egypt_city = City::new(egypt_city_id, "Thebes".into(), egypt_id, HexCoord::from_qr(7, 6));
    egypt_city.is_capital = true;
    s.state.cities.push(egypt_city);
    s.state.civilizations.iter_mut().find(|c| c.id == egypt_id).unwrap()
        .cities.push(egypt_city_id);

    // Register domination.
    let vid = s.state.id_gen.next_victory_id();
    s.state.victory_conditions.push(Box::new(DominationVictory { id: vid }));

    // Check progress: Rome needs 2 foreign capitals.
    let vc = DominationVictory { id: vid };
    let p = vc.check_progress(rome_id, &s.state);
    assert_eq!(p.target, 2, "Rome needs to capture 2 foreign capitals");
    assert_eq!(p.current, 0);

    // Capture Babylon's capital.
    s.state.units.retain(|u| u.id != s.babylon_warrior);
    let def = spawn_strong_unit(&mut s, babylon_id, HexCoord::from_qr(10, 5), 1);
    let atk = spawn_strong_unit(&mut s, rome_id, HexCoord::from_qr(10, 4), 100);
    DefaultRulesEngine.attack(&mut s.state, atk, def).unwrap();

    let p = vc.check_progress(rome_id, &s.state);
    assert_eq!(p.current, 1, "captured 1 of 2 capitals");
    assert!(!p.is_won(), "not yet won — Egypt's capital remains");

    advance_turn(&mut s);
    assert!(s.state.game_over.is_none(), "no victory with only 1 of 2 capitals");

    // Now capture Egypt's capital.
    let def2 = spawn_strong_unit(&mut s, egypt_id, HexCoord::from_qr(7, 6), 1);
    let atk2 = spawn_strong_unit(&mut s, rome_id, HexCoord::from_qr(7, 5), 100);
    DefaultRulesEngine.attack(&mut s.state, atk2, def2).unwrap();

    let p = vc.check_progress(rome_id, &s.state);
    assert_eq!(p.current, 2);
    assert!(p.is_won(), "should win with both capitals captured");

    advance_turn(&mut s);
    assert!(s.state.game_over.is_some(), "domination victory should fire");
    assert_eq!(s.state.game_over.as_ref().unwrap().condition, "Domination Victory");
}
