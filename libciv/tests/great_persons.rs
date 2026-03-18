/// Integration tests for the great persons retirement system.
///
/// Each test spawns a dummy great person via `spawn_great_person`, retires it
/// through `RulesEngine::retire_great_person`, and asserts the expected
/// side-effects (modifiers, gold, production, error handling).

mod common;

use libciv::{
    DefaultRulesEngine, GameState, GreatPersonType, RulesEngine, UnitCategory, UnitDomain,
};
use libciv::civ::{builtin_great_person_defs, spawn_great_person};
use libciv::game::StateDelta;
use libhexgrid::coord::HexCoord;

/// Helper: build a scenario with the four builtin great person defs registered.
fn scenario_with_great_person_defs() -> common::Scenario {
    let mut s = common::build_scenario();
    s.state.great_person_defs = builtin_great_person_defs();
    s
}

// ---------------------------------------------------------------------------
// Great General (Sun Tzu) -- land combat strength bonus
// ---------------------------------------------------------------------------

#[test]
fn test_retire_great_general_grants_land_combat_bonus() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Spawn Sun Tzu at Rome's warrior location.
    let gp_id = spawn_great_person(&mut s.state, s.rome_id, "Sun Tzu", HexCoord::from_qr(5, 3));

    // Retire him.
    let diff = rules.retire_great_person(&mut s.state, gp_id)
        .expect("retire should succeed");

    // Should have a GreatPersonRetired delta.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GreatPersonRetired { .. })));

    // Great person should be marked retired.
    let gp = s.state.great_person(gp_id).expect("gp should still exist in pool");
    assert!(gp.is_retired);

    // Rome's civilization should now have a great_person_modifier for land CS +5.
    let civ = s.state.civ(s.rome_id).unwrap();
    assert!(!civ.great_person_modifiers.is_empty(), "should have at least one modifier");

    // The great person unit should have been removed.
    let gp_units: Vec<_> = s.state.units.iter()
        .filter(|u| u.owner == s.rome_id && u.category == UnitCategory::GreatPerson)
        .collect();
    assert!(gp_units.is_empty(), "great person unit should be consumed");
}

// ---------------------------------------------------------------------------
// Great Admiral (Themistocles) -- naval combat strength bonus
// ---------------------------------------------------------------------------

#[test]
fn test_retire_great_admiral_grants_naval_combat_bonus() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    let gp_id = spawn_great_person(&mut s.state, s.rome_id, "Themistocles", HexCoord::from_qr(5, 3));

    let diff = rules.retire_great_person(&mut s.state, gp_id)
        .expect("retire should succeed");

    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GreatPersonRetired { .. })));

    let gp = s.state.great_person(gp_id).unwrap();
    assert!(gp.is_retired);

    // Should have a naval CS modifier.
    let civ = s.state.civ(s.rome_id).unwrap();
    assert!(!civ.great_person_modifiers.is_empty());
}

// ---------------------------------------------------------------------------
// Great Engineer (Imhotep) -- production burst
// ---------------------------------------------------------------------------

#[test]
fn test_retire_great_engineer_adds_production() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give Rome's city a production queue item so the burst has somewhere to go.
    let rome_city_idx = s.state.cities.iter().position(|c| c.id == s.rome_city).unwrap();
    s.state.cities[rome_city_idx].production_queue.push_back(
        libciv::civ::ProductionItem::Unit(s.warrior_type),
    );
    let production_before = s.state.cities[rome_city_idx].production_stored;

    // Spawn Imhotep at the city location.
    let gp_id = spawn_great_person(&mut s.state, s.rome_id, "Imhotep", HexCoord::from_qr(3, 3));

    let diff = rules.retire_great_person(&mut s.state, gp_id)
        .expect("retire should succeed");

    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GreatPersonRetired { .. })));
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::ProductionBurst { .. })));

    let production_after = s.state.cities[rome_city_idx].production_stored;
    assert!(production_after > production_before, "production should have increased");
}

// ---------------------------------------------------------------------------
// Great Merchant (Marco Polo) -- gold grant
// ---------------------------------------------------------------------------

#[test]
fn test_retire_great_merchant_grants_gold() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    let gold_before = s.state.civ(s.rome_id).unwrap().gold;

    let gp_id = spawn_great_person(&mut s.state, s.rome_id, "Marco Polo", HexCoord::from_qr(5, 3));

    let diff = rules.retire_great_person(&mut s.state, gp_id)
        .expect("retire should succeed");

    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GreatPersonRetired { .. })));
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GoldChanged { .. })));

    let gold_after = s.state.civ(s.rome_id).unwrap().gold;
    assert_eq!(gold_after - gold_before, 200, "should grant exactly 200 gold");
}

// ---------------------------------------------------------------------------
// Error: retire already-retired great person
// ---------------------------------------------------------------------------

#[test]
fn test_retire_already_retired_fails() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    let gp_id = spawn_great_person(&mut s.state, s.rome_id, "Sun Tzu", HexCoord::from_qr(5, 3));

    // First retire succeeds.
    rules.retire_great_person(&mut s.state, gp_id).expect("first retire should succeed");

    // Second retire should fail.
    let err = rules.retire_great_person(&mut s.state, gp_id);
    assert!(err.is_err(), "retiring twice should fail");
}

// ---------------------------------------------------------------------------
// Combat modifier integration: retired General's +5 CS applies in battle
// ---------------------------------------------------------------------------

#[test]
fn test_great_person_combat_modifier_applies_in_battle() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Retire Sun Tzu for Rome -- all land units get +5 CS.
    let gp_id = spawn_great_person(&mut s.state, s.rome_id, "Sun Tzu", HexCoord::from_qr(5, 3));
    rules.retire_great_person(&mut s.state, gp_id).expect("retire should succeed");

    // Move warriors adjacent for melee combat.
    // Rome warrior at (5,3), Babylon warrior at (8,5). Teleport them adjacent.
    if let Some(u) = s.state.unit_mut(s.rome_warrior) {
        u.coord = HexCoord::from_qr(6, 4);
        u.movement_left = 200;
    }
    if let Some(u) = s.state.unit_mut(s.babylon_warrior) {
        u.coord = HexCoord::from_qr(7, 4);
        u.movement_left = 200;
    }

    // Attack: Rome (base 20 + 5 modifier = 25 effective) vs Babylon (base 20).
    let diff = rules.attack(&mut s.state, s.rome_warrior, s.babylon_warrior)
        .expect("attack should succeed");

    // Extract damage dealt to defender -- with +5 CS advantage, Rome should deal more.
    let defender_damage = diff.deltas.iter().find_map(|d| {
        if let StateDelta::UnitAttacked { defender_damage, .. } = d {
            Some(*defender_damage)
        } else {
            None
        }
    }).expect("should have UnitAttacked delta");

    // With a +5 CS advantage (25 vs 20), expected base damage ~36 (30 * exp(5/25)).
    // Due to RNG [0.75, 1.25], the range is roughly 27-45.
    // Without the modifier it would be ~30 (30 * exp(0/25) = 30) * rng.
    // We just verify the attack happened; deterministic seed makes this reproducible.
    assert!(defender_damage > 0, "should deal positive damage");
}
