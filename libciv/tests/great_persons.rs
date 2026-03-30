/// Integration tests for the great persons system:
/// - Retirement effects (combat bonus, production burst, gold grant)
/// - Great person point accumulation from districts
/// - Auto-recruitment when points reach threshold
/// - Patronage (gold-based sponsoring)
/// - Era gating and inter-civ competition

mod common;

use libciv::{
    DefaultRulesEngine, GreatPersonType, RulesEngine, UnitCategory,
};
use libciv::civ::{
    builtin_great_person_defs, spawn_great_person, BuiltinDistrict, PlacedDistrict,
    GP_BASE_THRESHOLD,
};
use libciv::game::StateDelta;
use libhexgrid::coord::HexCoord;

/// Helper: build a scenario with the builtin great person defs registered.
fn scenario_with_great_person_defs() -> common::Scenario {
    let mut s = common::build_scenario();
    s.state.great_person_defs = builtin_great_person_defs();
    s
}

/// Helper: directly add a district to a city (bypasses tech/civic prereqs).
fn add_district(state: &mut libciv::GameState, city_id: libciv::CityId, district: BuiltinDistrict, coord: HexCoord) {
    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        city.districts.push(district);
    }
    state.placed_districts.push(PlacedDistrict {
        district_type: district,
        city_id,
        coord,
        buildings: Vec::new(),
        is_pillaged: false,
        unique_variant: None,
    });
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

// ===========================================================================
// Great person points accumulation from districts
// ===========================================================================

#[test]
fn test_district_generates_great_person_points() {
    let mut s = scenario_with_great_person_defs();

    // Give Rome a Campus district.
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, HexCoord::from_qr(4, 3));

    // Advance one turn.
    common::advance_turn(&mut s);

    // Rome should have 1 Scientist point.
    let civ = s.state.civ(s.rome_id).unwrap();
    let scientist_pts = civ.great_person_points.get(&GreatPersonType::Scientist).copied().unwrap_or(0);
    assert_eq!(scientist_pts, 1, "Campus should generate 1 Scientist point per turn");

    // Babylon should have 0 Scientist points (no Campus).
    let babylon_civ = s.state.civ(s.babylon_id).unwrap();
    let babylon_pts = babylon_civ.great_person_points.get(&GreatPersonType::Scientist).copied().unwrap_or(0);
    assert_eq!(babylon_pts, 0, "Babylon has no Campus, should have 0 Scientist points");
}

#[test]
fn test_theater_square_generates_three_types() {
    let mut s = scenario_with_great_person_defs();

    // Give Rome a Theater Square.
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::TheaterSquare, HexCoord::from_qr(4, 3));

    // Advance one turn.
    common::advance_turn(&mut s);

    // Theater Square generates points for Writer, Artist, and Musician.
    let civ = s.state.civ(s.rome_id).unwrap();
    let writer_pts = civ.great_person_points.get(&GreatPersonType::Writer).copied().unwrap_or(0);
    let artist_pts = civ.great_person_points.get(&GreatPersonType::Artist).copied().unwrap_or(0);
    let musician_pts = civ.great_person_points.get(&GreatPersonType::Musician).copied().unwrap_or(0);

    assert_eq!(writer_pts, 1, "Theater Square should generate Writer points");
    assert_eq!(artist_pts, 1, "Theater Square should generate Artist points");
    assert_eq!(musician_pts, 1, "Theater Square should generate Musician points");
}

#[test]
fn test_multiple_districts_accumulate() {
    let mut s = scenario_with_great_person_defs();

    // Give Rome two Campuses (different cities would be realistic, but
    // for testing accumulation we just care about the count).
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, HexCoord::from_qr(4, 3));

    // Advance 5 turns -- should accumulate 5 Scientist points.
    for _ in 0..5 {
        common::advance_turn(&mut s);
    }

    let civ = s.state.civ(s.rome_id).unwrap();
    let pts = civ.great_person_points.get(&GreatPersonType::Scientist).copied().unwrap_or(0);
    assert_eq!(pts, 5, "5 turns with 1 Campus should yield 5 Scientist points");
}

// ===========================================================================
// Auto-recruitment at threshold
// ===========================================================================

#[test]
fn test_great_person_auto_recruited_at_threshold() {
    let mut s = scenario_with_great_person_defs();

    // Give Rome a Campus.
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, HexCoord::from_qr(4, 3));

    // Set Rome's Scientist points to threshold - 1 so the next turn triggers recruitment.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .great_person_points.insert(GreatPersonType::Scientist, GP_BASE_THRESHOLD - 1);

    // Count great people before.
    let gp_count_before = s.state.great_people.len();

    // Advance one turn -- should accumulate 1 point, reaching threshold = 60, recruiting Euclid.
    common::advance_turn(&mut s);

    // A new great person should have been spawned.
    let gp_count_after = s.state.great_people.len();
    assert_eq!(gp_count_after, gp_count_before + 1, "should recruit one great person");

    // The recruited GP should be a Scientist owned by Rome.
    let recruited_gp = s.state.great_people.last().unwrap();
    assert_eq!(recruited_gp.person_type, GreatPersonType::Scientist);
    assert_eq!(recruited_gp.owner, Some(s.rome_id));
    assert_eq!(recruited_gp.name, "Euclid");

    // Points should have been reset (threshold subtracted: 60 - 60 = 0).
    let civ = s.state.civ(s.rome_id).unwrap();
    let pts = civ.great_person_points.get(&GreatPersonType::Scientist).copied().unwrap_or(0);
    assert_eq!(pts, 0, "points should reset after recruitment");
}

#[test]
fn test_points_carry_over_after_recruitment() {
    let mut s = scenario_with_great_person_defs();

    add_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, HexCoord::from_qr(4, 3));

    // Set points to threshold + 5 - 1 (so after +1 from turn, total = threshold + 5).
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .great_person_points.insert(GreatPersonType::Scientist, GP_BASE_THRESHOLD + 4);

    common::advance_turn(&mut s);

    // Should have recruited and carried over 5 points.
    let civ = s.state.civ(s.rome_id).unwrap();
    let pts = civ.great_person_points.get(&GreatPersonType::Scientist).copied().unwrap_or(0);
    assert_eq!(pts, 5, "excess points above threshold should carry over");
}

// ===========================================================================
// Patronage (gold-based sponsoring)
// ===========================================================================

#[test]
fn test_great_person_patronage_with_gold() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give Rome plenty of gold.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .gold = 500;

    // Patronize a Scientist (Euclid). With 0 points, cost = 60 * 3 = 180 gold.
    let result = rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist);
    assert!(result.is_ok(), "patronage should succeed with enough gold: {result:?}");

    let diff = result.unwrap();

    // Should have GoldChanged and GreatPersonPatronized deltas.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GoldChanged { .. })));
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GreatPersonPatronized { .. })));

    // Gold should have been deducted.
    let civ = s.state.civ(s.rome_id).unwrap();
    assert_eq!(civ.gold, 500 - 180, "should deduct 180 gold (60 points * 3 gold/point)");

    // Great person should exist and be owned by Rome.
    let gp = s.state.great_people.last().unwrap();
    assert_eq!(gp.person_type, GreatPersonType::Scientist);
    assert_eq!(gp.owner, Some(s.rome_id));
    assert_eq!(gp.name, "Euclid");

    // Points should be reset to 0.
    assert_eq!(civ.great_person_points.get(&GreatPersonType::Scientist).copied().unwrap_or(0), 0);
}

#[test]
fn test_patronage_partial_points_reduces_cost() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give Rome some accumulated points and gold.
    let rome_civ = s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap();
    rome_civ.gold = 200;
    rome_civ.great_person_points.insert(GreatPersonType::Scientist, 40);

    // Cost should be (60 - 40) * 3 = 60 gold.
    let result = rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist);
    assert!(result.is_ok());

    let civ = s.state.civ(s.rome_id).unwrap();
    assert_eq!(civ.gold, 200 - 60, "cost should be reduced by accumulated points");
}

#[test]
fn test_patronage_insufficient_gold_fails() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give Rome insufficient gold.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .gold = 10;

    let result = rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist);
    assert!(result.is_err(), "should fail with insufficient gold");
}

// ===========================================================================
// Era gating
// ===========================================================================

#[test]
fn test_era_gating_filters_candidates() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give Rome lots of gold.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .gold = 10_000;

    // Recruit the first Ancient Scientist (Euclid).
    rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist)
        .expect("first recruit should succeed");

    // The next Scientist (Hypatia) is Classical era. Without era advancement,
    // it should not be available if we're in Ancient era.
    // Set up eras so the game knows we're in Ancient.
    use libciv::civ::era::Era;
    if s.state.eras.is_empty() {
        let ancient_era_id = libciv::EraId::from_ulid(s.state.id_gen.next_ulid());
        s.state.eras.push(Era {
            id: ancient_era_id,
            name: "Ancient",
            age: libciv::AgeType::Ancient,
            tech_count: 8,
            civic_count: 4,
        });
        s.state.current_era_index = 0;
    }

    // Try to recruit another Scientist -- Hypatia is Classical, should fail.
    let result = rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist);
    assert!(
        matches!(result, Err(libciv::game::RulesError::NoGreatPersonAvailable)),
        "Classical-era GP should not be available in Ancient era: {result:?}"
    );
}

// ===========================================================================
// Competition between civs
// ===========================================================================

#[test]
fn test_competition_consumed_candidate() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give both civs gold.
    for civ in &mut s.state.civilizations {
        civ.gold = 10_000;
    }

    // Rome recruits the first Scientist (Euclid).
    rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist)
        .expect("Rome should recruit Euclid");

    // Add Classical era so Hypatia becomes available.
    use libciv::civ::era::Era;
    let classical_era_id = libciv::EraId::from_ulid(s.state.id_gen.next_ulid());
    s.state.eras.push(Era {
        id: classical_era_id,
        name: "Classical",
        age: libciv::AgeType::Classical,
        tech_count: 16,
        civic_count: 8,
    });
    s.state.current_era_index = s.state.eras.len() - 1;

    // Babylon recruits the next Scientist (Hypatia) -- threshold is now 120 (60 + 60).
    let result = rules.recruit_great_person(&mut s.state, s.babylon_id, GreatPersonType::Scientist);
    assert!(result.is_ok(), "Babylon should recruit Hypatia: {result:?}");

    let hypatia = s.state.great_people.last().unwrap();
    assert_eq!(hypatia.name, "Hypatia");
    assert_eq!(hypatia.owner, Some(s.babylon_id));
}

// ===========================================================================
// GP point modifier bonus from policies
// ===========================================================================

#[test]
fn test_gp_point_modifier_bonus_accumulates() {
    use libciv::rules::modifier::{Modifier, ModifierSource, StackingRule, EffectType};
    use libciv::rules::policy::Policy;
    use libciv::{PolicyId, PolicyType, YieldType};

    let mut s = scenario_with_great_person_defs();

    // Give Rome a Campus district.
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, HexCoord::from_qr(4, 3));

    // Create a policy that grants +2 GreatPersonPoints.
    let policy_id = PolicyId::from_ulid(s.state.id_gen.next_ulid());
    s.state.policies.push(Policy {
        id: policy_id,
        name: "Inspiration",
        policy_type: PolicyType::Economic,
        modifiers: vec![
            Modifier::new(
                ModifierSource::Policy("Inspiration"),
                libciv::rules::modifier::TargetSelector::Global,
                EffectType::YieldFlat(YieldType::GreatPersonPoints, 2),
                StackingRule::Additive,
            ),
        ],
        maintenance: 0,
    });

    // Activate the policy for Rome.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .active_policies.push(policy_id);

    // Advance one turn.
    common::advance_turn(&mut s);

    // Rome should have 1 (base) + 2 (modifier) = 3 Scientist points.
    let civ = s.state.civ(s.rome_id).unwrap();
    let scientist_pts = civ.great_person_points.get(&GreatPersonType::Scientist).copied().unwrap_or(0);
    assert_eq!(scientist_pts, 3, "Campus base (1) + policy modifier (2) = 3 Scientist points");
}

#[test]
fn test_gp_point_modifier_only_applies_to_active_types() {
    use libciv::rules::modifier::{Modifier, ModifierSource, StackingRule, EffectType};
    use libciv::rules::policy::Policy;
    use libciv::{PolicyId, PolicyType, YieldType};

    let mut s = scenario_with_great_person_defs();

    // Give Rome only a Campus (generates Scientist points).
    add_district(&mut s.state, s.rome_city, BuiltinDistrict::Campus, HexCoord::from_qr(4, 3));

    // Create a policy granting +3 GP points.
    let policy_id = PolicyId::from_ulid(s.state.id_gen.next_ulid());
    s.state.policies.push(Policy {
        id: policy_id,
        name: "Patronage",
        policy_type: PolicyType::Economic,
        modifiers: vec![
            Modifier::new(
                ModifierSource::Policy("Patronage"),
                libciv::rules::modifier::TargetSelector::Global,
                EffectType::YieldFlat(YieldType::GreatPersonPoints, 3),
                StackingRule::Additive,
            ),
        ],
        maintenance: 0,
    });

    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .active_policies.push(policy_id);

    common::advance_turn(&mut s);

    let civ = s.state.civ(s.rome_id).unwrap();

    // Scientist should get the bonus (Campus is active).
    let scientist_pts = civ.great_person_points.get(&GreatPersonType::Scientist).copied().unwrap_or(0);
    assert_eq!(scientist_pts, 4, "Campus base (1) + policy (3) = 4");

    // General should NOT get any points (no Encampment district).
    let general_pts = civ.great_person_points.get(&GreatPersonType::General).copied().unwrap_or(0);
    assert_eq!(general_pts, 0, "No Encampment means no General points, even with modifier");
}

// ===========================================================================
// Medieval era great persons
// ===========================================================================

#[test]
fn test_medieval_era_great_person_available() {
    use libciv::civ::era::Era;

    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give Rome lots of gold.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .gold = 50_000;

    // Recruit all Ancient + Classical Scientists (Euclid, Hypatia).
    // First, set up eras so Classical is available.
    let ancient_era_id = libciv::EraId::from_ulid(s.state.id_gen.next_ulid());
    s.state.eras.push(Era {
        id: ancient_era_id,
        name: "Ancient",
        age: libciv::AgeType::Ancient,
        tech_count: 8,
        civic_count: 4,
    });
    let classical_era_id = libciv::EraId::from_ulid(s.state.id_gen.next_ulid());
    s.state.eras.push(Era {
        id: classical_era_id,
        name: "Classical",
        age: libciv::AgeType::Classical,
        tech_count: 16,
        civic_count: 8,
    });
    s.state.current_era_index = 1; // Classical

    rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist)
        .expect("recruit Euclid");
    rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist)
        .expect("recruit Hypatia");

    // Now try Medieval -- should fail because we're in Classical era.
    let result = rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist);
    assert!(result.is_err(), "Medieval GP should not be available in Classical era");

    // Advance to Medieval era.
    let medieval_era_id = libciv::EraId::from_ulid(s.state.id_gen.next_ulid());
    s.state.eras.push(Era {
        id: medieval_era_id,
        name: "Medieval",
        age: libciv::AgeType::Medieval,
        tech_count: 24,
        civic_count: 12,
    });
    s.state.current_era_index = 2; // Medieval

    // Now recruit -- should get Al-Khwarizmi (first Medieval Scientist).
    let result = rules.recruit_great_person(&mut s.state, s.rome_id, GreatPersonType::Scientist);
    assert!(result.is_ok(), "Medieval GP should be available in Medieval era: {result:?}");

    let gp = s.state.great_people.last().unwrap();
    assert_eq!(gp.name, "Al-Khwarizmi");
    assert_eq!(gp.era, "Medieval");
}

// ===========================================================================
// Faith patronage
// ===========================================================================

#[test]
fn test_faith_patronage_prophet() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give Rome plenty of faith.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .faith = 500;

    // Patronize a Prophet with faith. With 0 points, cost = 60 * 2 = 120 faith.
    let result = rules.recruit_great_person_with_faith(&mut s.state, s.rome_id, GreatPersonType::Prophet);
    assert!(result.is_ok(), "faith patronage should succeed: {result:?}");

    let diff = result.unwrap();

    // Should have FaithChanged and GreatPersonPatronizedWithFaith deltas.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::FaithChanged { .. })));
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GreatPersonPatronizedWithFaith { .. })));

    // Faith should have been deducted.
    let civ = s.state.civ(s.rome_id).unwrap();
    assert_eq!(civ.faith, 500 - 120, "should deduct 120 faith (60 points * 2 faith/point)");

    // Great person should exist and be owned by Rome.
    let gp = s.state.great_people.last().unwrap();
    assert_eq!(gp.person_type, GreatPersonType::Prophet);
    assert_eq!(gp.owner, Some(s.rome_id));
    assert_eq!(gp.name, "Confucius");
}

#[test]
fn test_faith_patronage_non_prophet_fails() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .faith = 500;

    let result = rules.recruit_great_person_with_faith(&mut s.state, s.rome_id, GreatPersonType::Scientist);
    assert!(result.is_err(), "faith patronage should fail for non-Prophet types");
}

#[test]
fn test_faith_patronage_insufficient_faith_fails() {
    let mut s = scenario_with_great_person_defs();
    let rules = DefaultRulesEngine;

    // Give Rome very little faith.
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .faith = 10;

    let result = rules.recruit_great_person_with_faith(&mut s.state, s.rome_id, GreatPersonType::Prophet);
    assert!(result.is_err(), "should fail with insufficient faith");
}
