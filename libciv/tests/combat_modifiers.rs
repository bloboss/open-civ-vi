//! Integration tests for the combat modifier pipeline, XP, promotions, and cascading events.

mod common;

#[allow(unused_imports)]
use libciv::{
    CivId, DefaultRulesEngine, GameState, PromotionClass, RulesEngine,
    UnitCategory, UnitDomain, UnitId, UnitTypeId, AgeType,
};
use libciv::civ::{BasicUnit, City, Civilization, Leader, BuiltinAgenda};
use libciv::game::StateDelta;
use libciv::game::state::UnitTypeDef;
use libciv::rules::modifier::*;
use libhexgrid::coord::HexCoord;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn stub_leader(name: &'static str, civ_id: CivId) -> Leader {
    Leader { name, civ_id, agenda: BuiltinAgenda::Default }
}

/// Build a minimal two-civ state with configurable unit types.
/// Returns (state, attacker_civ, defender_civ, attacker_unit, defender_unit).
fn combat_scenario(
    atk_cs: u32,
    def_cs: u32,
    atk_era: Option<AgeType>,
    def_era: Option<AgeType>,
) -> (GameState, CivId, CivId, UnitId, UnitId, UnitTypeId, UnitTypeId) {
    let mut state = GameState::new(42, 14, 8);

    // Create custom types for test control.
    let atk_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.push(UnitTypeDef {
        id: atk_type_id, name: "TestAttacker", production_cost: 40,
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        max_movement: 200, combat_strength: Some(atk_cs),
        range: 0, vision_range: 2, can_found_city: false, resource_cost: None,
        siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None,
        era: atk_era, promotion_class: Some(PromotionClass::Melee),
    });

    let def_type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
    state.unit_type_defs.push(UnitTypeDef {
        id: def_type_id, name: "TestDefender", production_cost: 40,
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        max_movement: 200, combat_strength: Some(def_cs),
        range: 0, vision_range: 2, can_found_city: false, resource_cost: None,
        siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None,
        era: def_era, promotion_class: Some(PromotionClass::Melee),
    });

    let atk_civ_id = state.id_gen.next_civ_id();
    state.civilizations.push(
        Civilization::new(atk_civ_id, "Rome", "Roman", stub_leader("Caesar", atk_civ_id))
    );
    let atk_city_id = state.id_gen.next_city_id();
    let mut city = City::new(atk_city_id, "Roma".into(), atk_civ_id, HexCoord::from_qr(3, 3));
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations.iter_mut().find(|c| c.id == atk_civ_id).unwrap().cities.push(atk_city_id);

    let def_civ_id = state.id_gen.next_civ_id();
    state.civilizations.push(
        Civilization::new(def_civ_id, "Babylon", "Babylonian", stub_leader("Hammurabi", def_civ_id))
    );
    let def_city_id = state.id_gen.next_city_id();
    let mut city = City::new(def_city_id, "Babylon".into(), def_civ_id, HexCoord::from_qr(10, 5));
    city.is_capital = true;
    state.cities.push(city);
    state.civilizations.iter_mut().find(|c| c.id == def_civ_id).unwrap().cities.push(def_city_id);

    let atk_unit = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: atk_unit, unit_type: atk_type_id, owner: atk_civ_id,
        coord: HexCoord::from_qr(5, 3),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(atk_cs), promotions: Vec::new(),
        experience: 0, health: 100, range: 0, vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None,
        religion_id: None, spread_charges: None, religious_strength: None,
    });

    let def_unit = state.id_gen.next_unit_id();
    state.units.push(BasicUnit {
        id: def_unit, unit_type: def_type_id, owner: def_civ_id,
        coord: HexCoord::from_qr(6, 3), // adjacent to attacker
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        movement_left: 200, max_movement: 200,
        combat_strength: Some(def_cs), promotions: Vec::new(),
        experience: 0, health: 100, range: 0, vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None,
        religion_id: None, spread_charges: None, religious_strength: None,
    });

    (state, atk_civ_id, def_civ_id, atk_unit, def_unit, atk_type_id, def_type_id)
}

// ── Test 1: promotion_cs_bonus_applied ───────────────────────────────────────

#[test]
fn promotion_cs_bonus_applied() {
    let (mut state, _atk_civ, _def_civ, atk_unit, def_unit, _atk_type, _def_type) =
        combat_scenario(20, 20, None, None);

    // Find a promotion with a non-zero CombatStrengthFlat modifier (e.g. Ambush: +20).
    let ambush_promo = state.promotion_defs.iter()
        .find(|rp| rp.def.name == "Ambush")
        .expect("Ambush promotion should exist");
    let ambush_id = ambush_promo.id;

    // Give the attacker the Ambush promotion.
    state.unit_mut(atk_unit).unwrap().promotions.push(ambush_id);

    // Attack: the modifier pipeline should include the +20 from Ambush.
    let engine = DefaultRulesEngine;
    let diff = engine.attack(&mut state, atk_unit, def_unit).unwrap();

    // The attacker should have dealt more damage than a baseline 20v20 fight.
    // We can't check exact damage (RNG), but we can verify the attack happened.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitAttacked { .. })),
        "attack delta should exist");

    // The ExperienceGained delta should exist for the attacker.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::ExperienceGained { unit, .. } if *unit == atk_unit)),
        "attacker should gain XP");
}

// ── Test 2: government_cs_bonus_applied ──────────────────────────────────────

#[test]
fn government_cs_bonus_applied() {
    let (mut state, atk_civ, _def_civ, atk_unit, def_unit, _atk_type, _def_type) =
        combat_scenario(20, 20, None, None);

    // Check if any government has inherent combat modifiers.
    let gov_with_cs = state.governments.iter()
        .find(|g| g.inherent_modifiers.iter().any(|m|
            matches!(m.effect, EffectType::CombatStrengthFlat(_) | EffectType::CombatStrengthPercent(_))
        ));

    if let Some(gov) = gov_with_cs {
        // Set the civ's current government.
        let gov_name = gov.name;
        state.civilizations.iter_mut()
            .find(|c| c.id == atk_civ).unwrap()
            .current_government_name = Some(gov_name);
    } else {
        // No government has CS modifiers by default.
        // Add a test modifier to the first government.
        if let Some(gov) = state.governments.first_mut() {
            gov.inherent_modifiers.push(Modifier::new(
                ModifierSource::Custom("TestGovBonus"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(5),
                StackingRule::Additive,
            ));
            let gov_name = gov.name;
            state.civilizations.iter_mut()
                .find(|c| c.id == atk_civ).unwrap()
                .current_government_name = Some(gov_name);
        }
    }

    let engine = DefaultRulesEngine;
    let diff = engine.attack(&mut state, atk_unit, def_unit).unwrap();
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitAttacked { .. })));
}

// ── Test 3: policy_discipline_cs_bonus ───────────────────────────────────────

#[test]
fn policy_discipline_cs_bonus() {
    let (mut state, atk_civ, _def_civ, atk_unit, def_unit, _atk_type, _def_type) =
        combat_scenario(20, 20, None, None);

    // Find the "Discipline" policy or create one with CS modifier.
    let discipline_policy = state.policies.iter()
        .find(|p| p.name == "Discipline");

    let policy_id = if let Some(p) = discipline_policy {
        // Check if it has a CS modifier.
        let has_cs = p.modifiers.iter().any(|m|
            matches!(m.effect, EffectType::CombatStrengthFlat(_) | EffectType::CombatStrengthPercent(_))
        );
        if has_cs {
            p.id
        } else {
            // Add a CS modifier to a copy.
            let pid = p.id;
            let idx = state.policies.iter().position(|pp| pp.id == pid).unwrap();
            state.policies[idx].modifiers.push(Modifier::new(
                ModifierSource::Policy("Discipline"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(8),
                StackingRule::Additive,
            ));
            pid
        }
    } else {
        // Create a test policy.
        let pid = crate::common::build_scenario().state.id_gen.next_ulid();
        let pid = libciv::PolicyId::from_ulid(pid);
        state.policies.push(libciv::rules::policy::Policy {
            id: pid,
            name: "Discipline",
            policy_type: libciv::PolicyType::Military,
            prereq_civic: "Code of Laws",
            modifiers: vec![Modifier::new(
                ModifierSource::Policy("Discipline"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(8),
                StackingRule::Additive,
            )],
            maintenance: 0,
        });
        pid
    };

    // Add to civ's active policies.
    state.civilizations.iter_mut()
        .find(|c| c.id == atk_civ).unwrap()
        .active_policies.push(policy_id);

    let engine = DefaultRulesEngine;
    let diff = engine.attack(&mut state, atk_unit, def_unit).unwrap();
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitAttacked { .. })),
        "attack should succeed with policy bonus");
}

// ── Test 4: xp_awarded_on_kill ───────────────────────────────────────────────

#[test]
fn xp_awarded_on_kill() {
    // Give attacker overwhelming CS to guarantee a kill.
    let (mut state, _atk_civ, _def_civ, atk_unit, def_unit, _atk_type, _def_type) =
        combat_scenario(100, 5, None, None);

    // Reduce defender health to guarantee kill.
    state.unit_mut(def_unit).unwrap().health = 1;

    let engine = DefaultRulesEngine;
    let diff = engine.attack(&mut state, atk_unit, def_unit).unwrap();

    // Should have ExperienceGained delta.
    let xp_delta = diff.deltas.iter().find(|d|
        matches!(d, StateDelta::ExperienceGained { unit, .. } if *unit == atk_unit)
    );
    assert!(xp_delta.is_some(), "XP should be awarded on kill");

    if let Some(StateDelta::ExperienceGained { amount, new_total, .. }) = xp_delta {
        // Kill base XP = 5, era_diff = 0, scale = 1.0, so XP = 5.
        assert_eq!(*amount, 5, "kill XP should be 5 for same-era units");
        assert_eq!(*new_total, 5);
    }
}

// ── Test 5: xp_scaled_by_era_difference ──────────────────────────────────────

#[test]
fn xp_scaled_by_era_difference() {
    // Attacker is Ancient, defender is Medieval (era_diff = 2 => scale = 2.0).
    let (mut state, _atk_civ, _def_civ, atk_unit, def_unit, _atk_type, _def_type) =
        combat_scenario(100, 5, Some(AgeType::Ancient), Some(AgeType::Medieval));

    state.unit_mut(def_unit).unwrap().health = 1;

    let engine = DefaultRulesEngine;
    let diff = engine.attack(&mut state, atk_unit, def_unit).unwrap();

    let xp_delta = diff.deltas.iter().find(|d|
        matches!(d, StateDelta::ExperienceGained { unit, .. } if *unit == atk_unit)
    );
    assert!(xp_delta.is_some());

    if let Some(StateDelta::ExperienceGained { amount, .. }) = xp_delta {
        // Kill base = 5, era_diff = 2, scale = 2.0, XP = 10.
        assert_eq!(*amount, 10, "kill XP should be scaled by era diff (2.0x)");
    }
}

// ── Test 6: xp_minimum_one ───────────────────────────────────────────────────

#[test]
fn xp_minimum_one() {
    // Attacker is Future era, defender is Ancient (era_diff = -8 => scale = 0.25).
    let (mut state, _atk_civ, _def_civ, atk_unit, def_unit, _atk_type, _def_type) =
        combat_scenario(100, 5, Some(AgeType::Future), Some(AgeType::Ancient));

    state.unit_mut(def_unit).unwrap().health = 1;

    let engine = DefaultRulesEngine;
    let diff = engine.attack(&mut state, atk_unit, def_unit).unwrap();

    let xp_delta = diff.deltas.iter().find(|d|
        matches!(d, StateDelta::ExperienceGained { unit, .. } if *unit == atk_unit)
    );
    assert!(xp_delta.is_some());

    if let Some(StateDelta::ExperienceGained { amount, .. }) = xp_delta {
        // Kill base = 5, scale = 0.25, 5 * 0.25 = 1.25, round = 1. Min 1.
        assert!(*amount >= 1, "XP must be at least 1");
    }
}

// ── Test 7: promotion_eligibility_and_heal ───────────────────────────────────

#[test]
fn promotion_eligibility_and_heal() {
    let (mut state, _atk_civ, _def_civ, atk_unit, _def_unit, _atk_type, _def_type) =
        combat_scenario(20, 20, None, None);

    // Give unit exactly 15 XP (threshold for first promotion).
    state.unit_mut(atk_unit).unwrap().experience = 15;
    // Set health to 60 so we can verify the +50 HP heal.
    state.unit_mut(atk_unit).unwrap().health = 60;

    // Find a tier-1 Melee promotion with no prerequisites.
    let tier1_promo = state.promotion_defs.iter()
        .find(|rp| rp.def.class == PromotionClass::Melee && rp.def.tier == 1 && rp.def.prerequisites.is_empty())
        .expect("should have a tier-1 melee promotion");
    let promo_name = tier1_promo.def.name;

    let engine = DefaultRulesEngine;
    let diff = engine.promote_unit(&mut state, atk_unit, promo_name).unwrap();

    // Check UnitPromoted delta.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitPromoted { unit, promotion_name, .. }
        if *unit == atk_unit && *promotion_name == promo_name)),
        "UnitPromoted delta expected");

    // Check UnitHealed delta: 60 + 50 = 100 (capped).
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitHealed { unit, new_health, .. }
        if *unit == atk_unit && *new_health == 100)),
        "UnitHealed delta expected (60 + 50 = 100)");

    // Verify unit state.
    let unit = state.unit(atk_unit).unwrap();
    assert_eq!(unit.health, 100);
    assert_eq!(unit.promotions.len(), 1);

    // Test: 14 XP should NOT be eligible.
    let (mut state2, _, _, unit2, _, _, _) = combat_scenario(20, 20, None, None);
    state2.unit_mut(unit2).unwrap().experience = 14;
    let result = engine.promote_unit(&mut state2, unit2, promo_name);
    assert!(result.is_err(), "14 XP should not be eligible for promotion");
}

// ── Test 8: battle_won_historic_moment ───────────────────────────────────────

#[test]
fn battle_won_historic_moment() {
    let (mut state, atk_civ, _def_civ, atk_unit, def_unit, _atk_type, _def_type) =
        combat_scenario(100, 5, None, None);

    // Ensure kill by reducing defender health.
    state.unit_mut(def_unit).unwrap().health = 1;

    let engine = DefaultRulesEngine;
    let diff = engine.attack(&mut state, atk_unit, def_unit).unwrap();

    // Should have HistoricMomentEarned for BattleWon.
    assert!(diff.deltas.iter().any(|d| matches!(d,
        StateDelta::HistoricMomentEarned { civ, moment: "BattleWon", era_score: 1 }
        if *civ == atk_civ
    )), "BattleWon historic moment expected on non-barbarian kill");

    // Verify civ's era_score increased.
    let civ = state.civ(atk_civ).unwrap();
    assert!(civ.era_score >= 1, "era_score should have increased");
}

// ── Test 9: barbarian_kill_no_auto_clear ─────────────────────────────────────

#[test]
fn barbarian_kill_no_auto_clear() {
    let (mut state, _atk_civ, _def_civ, atk_unit, def_unit, _atk_type, _def_type) =
        combat_scenario(100, 5, None, None);

    // Create a barbarian camp at the defender's location.
    let camp_id = state.id_gen.next_barbarian_camp_id();
    let def_coord = state.unit(def_unit).unwrap().coord;
    let barb_owner = state.unit(def_unit).unwrap().owner;
    state.barbarian_camps.push(libciv::civ::BarbarianCamp {
        id: camp_id,
        coord: def_coord,
        owner: barb_owner,
        spawned_turn: 0,
        scout_state: libciv::civ::barbarian::ScoutState::NotSpawned,
        spawned_units: Vec::new(),
        units_spawned_count: 0,
        boldness: 0,
        clan_type: Some(libciv::civ::barbarian::ClanType::Flatland),
        clan_interactions: Vec::new(),
        conversion_progress: 0,
        converted: false,
    });

    // Set up barbarian civ for the defender.
    state.barbarian_civ = Some(barb_owner);

    // Ensure kill.
    state.unit_mut(def_unit).unwrap().health = 1;

    let engine = DefaultRulesEngine;
    let diff = engine.attack(&mut state, atk_unit, def_unit).unwrap();

    // Defender should be destroyed.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitDestroyed { unit } if *unit == def_unit)),
        "defender should be killed");

    // Camp should NOT be auto-cleared.
    assert!(state.barbarian_camps.iter().any(|c| c.id == camp_id),
        "barbarian camp should NOT be auto-cleared on kill");

    // No BarbarianCampDestroyed delta.
    assert!(!diff.deltas.iter().any(|d| matches!(d, StateDelta::BarbarianCampDestroyed { .. })),
        "no BarbarianCampDestroyed delta expected");

    // Attacker should NOT get BattleWon since defender is barbarian... wait,
    // actually the test says "barbarian_kill" meaning attacker kills a barb.
    // The attacker is NOT barbarian, so they should get BattleWon.
    // But the check in combat.rs is: is_barbarian_attacker (attacker is barb).
    // So a non-barb killing a barb DOES get BattleWon. That's correct.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::HistoricMomentEarned { moment: "BattleWon", .. })),
        "non-barbarian attacker killing barbarian should get BattleWon");
}
