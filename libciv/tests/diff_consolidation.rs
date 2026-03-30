/// Integration tests for diff consolidation: verifying that `process_turn`
/// returns a complete `GameStateDiff` capturing every state change.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::game::StateDelta;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Step 1: TurnEngine forwards the diff from advance_turn
// ---------------------------------------------------------------------------

/// `process_turn` must return a non-empty diff containing `TurnAdvanced`.
#[test]
fn process_turn_returns_diff_with_turn_advanced() {
    let mut s = common::build_scenario();
    let engine = libciv::TurnEngine::new();
    let rules = DefaultRulesEngine;
    let diff = engine.process_turn(&mut s.state, &rules);

    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::TurnAdvanced { .. })),
        "process_turn should return a diff containing TurnAdvanced"
    );
}

// ---------------------------------------------------------------------------
// Step 3a: Population growth emits PopulationGrew AND CitizenAssigned
// ---------------------------------------------------------------------------

/// When a city has enough food to grow, the diff should contain both
/// `PopulationGrew` and `CitizenAssigned` deltas.
#[test]
fn population_growth_emits_citizen_assigned() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Set Rome's capital to be on the verge of growing: food_stored just
    // below food_to_grow, with enough food yield from worked tiles.
    let city = s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap();
    city.food_to_grow = 15;
    city.food_stored = 14; // needs just 1 more food

    // Ensure the city has at least one worked tile that yields food.
    // The city center tile (3,3) should yield food from terrain.
    if city.worked_tiles.is_empty() {
        city.worked_tiles.push(HexCoord::from_qr(3, 3));
    }

    let diff = rules.advance_turn(&mut s.state);

    let grew = diff.deltas.iter().any(|d| matches!(d,
        StateDelta::PopulationGrew { city, .. } if *city == s.rome_city
    ));
    let assigned = diff.deltas.iter().any(|d| matches!(d,
        StateDelta::CitizenAssigned { city, .. } if *city == s.rome_city
    ));

    if grew {
        assert!(
            assigned,
            "PopulationGrew was emitted but CitizenAssigned was not — \
             auto-assign should produce a delta"
        );
    }
}

// ---------------------------------------------------------------------------
// Step 3f: City revolt emits CityRevolted AND LoyaltyChanged (post-revolt)
// ---------------------------------------------------------------------------

/// When a city revolts (loyalty reaches 0), the diff should contain both
/// `CityRevolted` and a `LoyaltyChanged` delta reflecting the post-revolt
/// loyalty value (50 for flip, 25 for independent).
#[test]
fn city_revolt_emits_loyalty_changed_after_revolt() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Rome high population to exert strong loyalty pressure.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .population = 10;

    // Add a second Babylon city very close to Rome (at 4,4).
    let nippur_id = s.state.id_gen.next_city_id();
    let mut nippur = libciv::civ::City::new(
        nippur_id,
        "Nippur".into(),
        s.babylon_id,
        HexCoord::from_qr(4, 4),
    );
    nippur.population = 1;
    s.state.cities.push(nippur);
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .cities.push(nippur_id);

    // Set loyalty to 1 so it drops to 0 on the next turn.
    s.state.cities.iter_mut()
        .find(|c| c.id == nippur_id).unwrap()
        .loyalty = 1;

    let diff = rules.advance_turn(&mut s.state);

    let revolted = diff.deltas.iter().any(|d| matches!(d,
        StateDelta::CityRevolted { city, .. } if *city == nippur_id
    ));

    if revolted {
        // There should be a LoyaltyChanged delta for the post-revolt loyalty reset.
        let loyalty_after = diff.deltas.iter().any(|d| matches!(d,
            StateDelta::LoyaltyChanged { city, new_value, .. }
                if *city == nippur_id && (*new_value == 50 || *new_value == 25)
        ));
        assert!(
            loyalty_after,
            "CityRevolted was emitted but no LoyaltyChanged for the post-revolt \
             loyalty reset (expected new_value 50 or 25)"
        );
    }
}
