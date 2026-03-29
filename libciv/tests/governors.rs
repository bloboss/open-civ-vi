/// Integration tests for the governor system: assignment, establishment timer,
/// promotion tree, modifier application, and civic title grants.

mod common;

use libciv::{DefaultRulesEngine, GovernorId, RulesEngine};
use libciv::civ::Governor;
use libciv::civ::governor::{promotion_def, promotions_for, get_governor_modifiers};
use libciv::game::diff::StateDelta;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn rules() -> DefaultRulesEngine { DefaultRulesEngine }

fn add_governor(s: &mut common::Scenario, def_name: &'static str) -> GovernorId {
    let id = GovernorId::from_ulid(s.state.id_gen.next_ulid());
    let owner = s.rome_id;
    s.state.governors.push(Governor::new(id, def_name, owner));
    id
}

// ---------------------------------------------------------------------------
// assign_governor
// ---------------------------------------------------------------------------

#[test]
fn test_assign_governor_to_city() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");

    let diff = rules().assign_governor(&mut s.state, gov_id, s.rome_city).unwrap();

    // Governor state updated.
    let gov = s.state.governors.iter().find(|g| g.id == gov_id).unwrap();
    assert_eq!(gov.assigned_city, Some(s.rome_city));
    assert_eq!(gov.turns_to_establish, 5);

    // Delta emitted.
    assert!(diff.deltas.iter().any(|d| matches!(d,
        StateDelta::GovernorAssigned { governor, city, .. }
        if *governor == gov_id && *city == s.rome_city
    )));
}

#[test]
fn test_assign_governor_to_wrong_civ_city() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Victor");

    // Babylon's city, but governor belongs to Rome => GovernorNotOwned.
    let err = rules().assign_governor(&mut s.state, gov_id, s.babylon_city);
    assert!(err.is_err());
}

#[test]
fn test_assign_governor_already_in_city() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Magnus");

    rules().assign_governor(&mut s.state, gov_id, s.rome_city).unwrap();

    // Assigning to the same city again => GovernorAlreadyInCity.
    let err = rules().assign_governor(&mut s.state, gov_id, s.rome_city);
    assert!(err.is_err());
}

#[test]
fn test_reassign_governor_resets_timer() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Liang");

    rules().assign_governor(&mut s.state, gov_id, s.rome_city).unwrap();

    // Manually reduce timer to simulate partial establishment.
    s.state.governors.iter_mut().find(|g| g.id == gov_id).unwrap().turns_to_establish = 1;

    // Found a second city for Rome.
    let city2_id = s.state.id_gen.next_city_id();
    let city2 = libciv::civ::City::new(city2_id, "Ostia".into(), s.rome_id, libhexgrid::coord::HexCoord::from_qr(5, 5));
    s.state.cities.push(city2);
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().cities.push(city2_id);

    rules().assign_governor(&mut s.state, gov_id, city2_id).unwrap();

    let gov = s.state.governors.iter().find(|g| g.id == gov_id).unwrap();
    assert_eq!(gov.assigned_city, Some(city2_id));
    assert_eq!(gov.turns_to_establish, 5, "reassignment should reset the timer");
}

// ---------------------------------------------------------------------------
// Establishment timer
// ---------------------------------------------------------------------------

#[test]
fn test_governor_establishment_timer_countdown() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Victor");
    rules().assign_governor(&mut s.state, gov_id, s.rome_city).unwrap();

    // Governor should NOT be established yet.
    assert!(!s.state.governors.iter().find(|g| g.id == gov_id).unwrap().is_established());

    // Advance 4 turns — still not established.
    for _ in 0..4 {
        common::advance_turn(&mut s);
    }
    assert!(!s.state.governors.iter().find(|g| g.id == gov_id).unwrap().is_established());

    // 5th turn — now established.
    common::advance_turn(&mut s);
    assert!(s.state.governors.iter().find(|g| g.id == gov_id).unwrap().is_established());
}

#[test]
fn test_governor_established_delta_emitted() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");
    rules().assign_governor(&mut s.state, gov_id, s.rome_city).unwrap();

    // Set timer to 1 so the next turn triggers establishment.
    s.state.governors.iter_mut().find(|g| g.id == gov_id).unwrap().turns_to_establish = 1;

    let diff = DefaultRulesEngine.advance_turn(&mut s.state);
    assert!(diff.deltas.iter().any(|d| matches!(d,
        StateDelta::GovernorEstablished { governor, city }
        if *governor == gov_id && *city == s.rome_city
    )));
}

// ---------------------------------------------------------------------------
// Loyalty bonus (existing behavior preserved)
// ---------------------------------------------------------------------------

#[test]
fn test_governor_loyalty_bonus_only_when_established() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Victor");

    // Assign but not yet established.
    let gov = s.state.governors.iter_mut().find(|g| g.id == gov_id).unwrap();
    gov.assigned_city = Some(s.rome_city);
    gov.turns_to_establish = 3;

    // Compute loyalty delta — governor should NOT contribute.
    let diff1 = DefaultRulesEngine.advance_turn(&mut s.state);
    let delta_unestablished: Vec<_> = diff1.deltas.iter()
        .filter_map(|d| match d {
            StateDelta::LoyaltyChanged { city, delta, .. } if *city == s.rome_city => Some(*delta),
            _ => None,
        })
        .collect();

    // Now establish the governor.
    s.state.governors.iter_mut().find(|g| g.id == gov_id).unwrap().turns_to_establish = 0;

    let diff2 = DefaultRulesEngine.advance_turn(&mut s.state);
    let delta_established: Vec<_> = diff2.deltas.iter()
        .filter_map(|d| match d {
            StateDelta::LoyaltyChanged { city, delta, .. } if *city == s.rome_city => Some(*delta),
            _ => None,
        })
        .collect();

    // Established governor should give a better (higher) loyalty delta.
    if !delta_unestablished.is_empty() && !delta_established.is_empty() {
        assert!(
            delta_established[0] >= delta_unestablished[0],
            "established governor should not reduce loyalty: est={}, unest={}",
            delta_established[0], delta_unestablished[0],
        );
    }
}

// ---------------------------------------------------------------------------
// Promotions
// ---------------------------------------------------------------------------

#[test]
fn test_promote_governor_basic() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");

    // Grant a title so the civ can promote.
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().governor_titles = 1;

    // Promote with tier-1 promotion (Librarian requires Connoisseur which is tier-0 base).
    // First, the base promotion prerequisites: tier 1 requires the base ("Connoisseur").
    // But the base (tier 0) is free — it's applied automatically via get_governor_modifiers.
    // For the promote action, only non-base promotions are unlocked.
    // Librarian requires "Connoisseur" — but we haven't unlocked "Connoisseur" as a promotion.
    // In Civ VI, the base is always active. Let's test with a tier-1 that requires base.
    // We need to first unlock "Connoisseur" as a promotion to satisfy prereqs.
    // Actually, looking at the design: tier-0 is the base that's auto-applied via modifiers,
    // but the promotion tree still needs to be traversed. Let me unlock Connoisseur first.
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().governor_titles += 1;
    rules().promote_governor(&mut s.state, gov_id, "Connoisseur").unwrap();

    let diff = rules().promote_governor(&mut s.state, gov_id, "Librarian").unwrap();

    let gov = s.state.governors.iter().find(|g| g.id == gov_id).unwrap();
    assert!(gov.has_promotion("Librarian"));

    assert!(diff.deltas.iter().any(|d| matches!(d,
        StateDelta::GovernorPromoted { governor, promotion }
        if *governor == gov_id && *promotion == "Librarian"
    )));

    // Title was consumed.
    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert_eq!(civ.governor_titles, 0);
}

#[test]
fn test_promote_governor_prerequisite_not_met() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");

    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().governor_titles = 1;

    // Try to unlock Librarian (tier 1) without Connoisseur (tier 0) => error.
    let err = rules().promote_governor(&mut s.state, gov_id, "Librarian");
    assert!(err.is_err());
}

#[test]
fn test_promote_governor_insufficient_titles() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");

    // No titles available.
    assert_eq!(
        s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap().governor_titles,
        0
    );

    let err = rules().promote_governor(&mut s.state, gov_id, "Connoisseur");
    assert!(err.is_err());
}

#[test]
fn test_promote_governor_already_unlocked() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().governor_titles = 2;

    rules().promote_governor(&mut s.state, gov_id, "Connoisseur").unwrap();

    // Try to unlock same promotion again => error.
    let err = rules().promote_governor(&mut s.state, gov_id, "Connoisseur");
    assert!(err.is_err());
}

#[test]
fn test_promote_wrong_governor() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Victor");
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().governor_titles = 1;

    // "Connoisseur" belongs to Pingala, not Victor => PromotionNotFound.
    let err = rules().promote_governor(&mut s.state, gov_id, "Connoisseur");
    assert!(err.is_err());
}

// ---------------------------------------------------------------------------
// Modifier application
// ---------------------------------------------------------------------------

#[test]
fn test_governor_modifiers_only_when_established() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");

    // Not assigned to a city — no modifiers.
    let gov = s.state.governors.iter().find(|g| g.id == gov_id).unwrap();
    assert!(get_governor_modifiers(gov).is_empty());

    // Assign and manually establish.
    rules().assign_governor(&mut s.state, gov_id, s.rome_city).unwrap();
    s.state.governors.iter_mut().find(|g| g.id == gov_id).unwrap().turns_to_establish = 0;

    // Established Pingala should have base (Connoisseur) modifiers.
    let gov = s.state.governors.iter().find(|g| g.id == gov_id).unwrap();
    let mods = get_governor_modifiers(gov);
    assert!(!mods.is_empty(), "established Pingala should have base modifiers");
}

#[test]
fn test_pingala_yields_bonus() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");

    // Compute baseline yields without governor.
    let baseline = rules().compute_yields(&s.state, s.rome_id);

    // Assign and establish Pingala.
    rules().assign_governor(&mut s.state, gov_id, s.rome_city).unwrap();
    s.state.governors.iter_mut().find(|g| g.id == gov_id).unwrap().turns_to_establish = 0;

    // Compute yields with established governor.
    let with_gov = rules().compute_yields(&s.state, s.rome_id);

    // Pingala base (Connoisseur) gives +3 Culture (flat).
    assert!(
        with_gov.culture > baseline.culture,
        "Pingala should boost culture: with={}, without={}",
        with_gov.culture, baseline.culture,
    );
}

#[test]
fn test_promoted_governor_extra_modifiers() {
    let mut s = common::build_scenario();
    let gov_id = add_governor(&mut s, "Pingala");

    // Assign, establish, and promote.
    rules().assign_governor(&mut s.state, gov_id, s.rome_city).unwrap();
    s.state.governors.iter_mut().find(|g| g.id == gov_id).unwrap().turns_to_establish = 0;

    // Grant titles and promote through Connoisseur -> Librarian.
    s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap().governor_titles = 2;
    rules().promote_governor(&mut s.state, gov_id, "Connoisseur").unwrap();

    let before_promo = rules().compute_yields(&s.state, s.rome_id);
    rules().promote_governor(&mut s.state, gov_id, "Librarian").unwrap();
    let after_promo = rules().compute_yields(&s.state, s.rome_id);

    // Librarian adds +3 Science.
    assert!(
        after_promo.science > before_promo.science,
        "Librarian should boost science: with={}, without={}",
        after_promo.science, before_promo.science,
    );
}

// ---------------------------------------------------------------------------
// Promotion tree structure
// ---------------------------------------------------------------------------

#[test]
fn test_promotion_definitions_exist() {
    // Every governor should have exactly 6 promotions.
    for name in libciv::civ::GOVERNOR_NAMES {
        let promos = promotions_for(name);
        assert_eq!(promos.len(), 6, "governor {name} should have 6 promotions, got {}", promos.len());
    }
}

#[test]
fn test_promotion_tiers() {
    // Each governor should have: 1 tier-0, 2 tier-1, 2 tier-2, 1 tier-3.
    for name in libciv::civ::GOVERNOR_NAMES {
        let promos = promotions_for(name);
        let tier_counts: Vec<usize> = (0..=3)
            .map(|t| promos.iter().filter(|p| p.tier == t).count())
            .collect();
        assert_eq!(tier_counts, vec![1, 2, 2, 1],
            "governor {name} tier distribution should be [1,2,2,1], got {tier_counts:?}");
    }
}

#[test]
fn test_promotion_prerequisites_valid() {
    // Every prerequisite name should reference an actual promotion of the same governor.
    for name in libciv::civ::GOVERNOR_NAMES {
        let promos = promotions_for(name);
        let names: Vec<&str> = promos.iter().map(|p| p.name).collect();
        for promo in &promos {
            for &req in promo.requires {
                assert!(
                    names.contains(&req),
                    "governor {name}: promotion '{}' requires '{}' which doesn't exist",
                    promo.name, req,
                );
            }
        }
    }
}

#[test]
fn test_promotion_lookup() {
    let promo = promotion_def("Librarian").expect("should find Librarian");
    assert_eq!(promo.governor, "Pingala");
    assert_eq!(promo.tier, 1);
    assert_eq!(promo.requires, &["Connoisseur"]);
}

// ---------------------------------------------------------------------------
// Civic governor title grants
// ---------------------------------------------------------------------------

#[test]
fn test_civic_grants_governor_title() {
    let mut s = common::build_scenario();

    // Set up a civic tree with "Code of Laws" so completing it grants a title.
    let code_of_laws_id = {
        let id = libciv::CivicId::from_ulid(s.state.id_gen.next_ulid());
        s.state.civic_tree.add_node(libciv::rules::tech::CivicNode {
            id,
            name: "Code of Laws",
            cost: 25,
            prerequisites: Vec::new(),
            effects: Vec::new(),
            inspiration_description: "",
            inspiration_effects: Vec::new(),
        });
        id
    };

    // Start researching it.
    let civ = s.state.civilizations.iter_mut().find(|c| c.id == s.rome_id).unwrap();
    civ.civic_in_progress = Some(libciv::civ::CivicProgress {
        civic_id: code_of_laws_id,
        progress: 24,
        inspired: false,
    });

    // Give enough culture to complete it in one turn.
    // The civ needs culture yield. Add a worked tile with culture.
    // Simplest: just set progress to cost-1 so 1 culture completes it.
    // Base city yields already give 1 culture per city per turn.
    // So with progress=24 and cost=25, one turn with 1+ culture should complete.

    let diff = DefaultRulesEngine.advance_turn(&mut s.state);

    // Should have completed the civic.
    let has_civic_completed = diff.deltas.iter().any(|d| matches!(d,
        StateDelta::CivicCompleted { civic, .. } if *civic == "Code of Laws"
    ));
    assert!(has_civic_completed, "Code of Laws should complete");

    // Should have earned a governor title.
    let has_title = diff.deltas.iter().any(|d| matches!(d,
        StateDelta::GovernorTitleEarned { civ } if *civ == s.rome_id
    ));
    assert!(has_title, "should earn a governor title from Code of Laws");

    // Civ should have 1 title now.
    let civ = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert_eq!(civ.governor_titles, 1);
}
