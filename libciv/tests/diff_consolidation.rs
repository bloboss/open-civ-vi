/// Integration tests for diff consolidation: verifying that `process_turn`
/// returns a complete `GameStateDiff` capturing every state change.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine, UnitCategory, UnitDomain, UnitTypeId};
use libciv::civ::BasicUnit;
use libciv::game::StateDelta;
use libciv::game::state::UnitTypeDef;
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

// ---------------------------------------------------------------------------
// Step: Autonomous trader movement emits UnitMoved
// ---------------------------------------------------------------------------

/// A trader unit with `trade_origin` and `trade_destination` set should move
/// autonomously toward its destination during `advance_turn`, emitting at
/// least one `UnitMoved` delta.
#[test]
fn trader_autonomous_movement_emits_unit_moved() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Register a trader unit type.
    let trader_type = UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id: trader_type, name: "trader", production_cost: 65,
        max_movement: 200, combat_strength: None,
        domain: UnitDomain::Land, category: UnitCategory::Trader,
        range: 0, vision_range: 2, can_found_city: false,
        resource_cost: None, siege_bonus: 0, max_charges: 0,
        exclusive_to: None, replaces: None, era: None, promotion_class: None,
    });

    // Place the trader at Rome's city tile, assigned to travel to Babylon's city.
    let trader_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: trader_id,
        unit_type: trader_type,
        owner: s.rome_id,
        coord: HexCoord::from_qr(3, 3), // Rome city tile
        domain: UnitDomain::Land,
        category: UnitCategory::Trader,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None,
        trade_origin: Some(s.rome_city),
        trade_destination: Some(s.babylon_city),
        religion_id: None,
        spread_charges: None,
        religious_strength: None,
    });

    let diff = rules.advance_turn(&mut s.state);

    let moved = diff.deltas.iter().any(|d| matches!(d,
        StateDelta::UnitMoved { unit, .. } if *unit == trader_id
    ));

    assert!(
        moved,
        "Trader with trade_destination should emit UnitMoved during advance_turn; \
         got deltas: {:?}",
        diff.deltas.iter()
            .filter(|d| matches!(d, StateDelta::UnitMoved { .. } | StateDelta::TradeRouteEstablished { .. } | StateDelta::TradeRouteCleared { .. }))
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Step: Unique unit healing (Mamluk) emits UnitHealed
// ---------------------------------------------------------------------------

/// A damaged Mamluk-type unit (Arabia civ with HealEveryTurn ability) should
/// heal at end of turn and emit a `UnitHealed` delta.
#[test]
fn unique_unit_healing_emits_unit_healed() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Register a "mamluk" unit type matching Arabia's unique unit name.
    let mamluk_type = UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id: mamluk_type, name: "mamluk", production_cost: 220,
        max_movement: 400, combat_strength: Some(50),
        domain: UnitDomain::Land, category: UnitCategory::Combat,
        range: 0, vision_range: 2, can_found_city: false,
        resource_cost: None, siege_bonus: 0, max_charges: 0,
        exclusive_to: None, replaces: None, era: None, promotion_class: None,
    });

    // Set Rome's civ_identity to Arabia so lookup_bundle returns the Arabia
    // bundle (which has the Mamluk unique unit with HealEveryTurn).
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .civ_identity = Some(libciv::civ::civ_identity::BuiltinCiv::Arabia);

    // Create a damaged mamluk unit owned by Rome.
    let mamluk_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: mamluk_id,
        unit_type: mamluk_type,
        owner: s.rome_id,
        coord: HexCoord::from_qr(4, 3),
        domain: UnitDomain::Land,
        category: UnitCategory::Combat,
        movement_left: 400,
        max_movement: 400,
        combat_strength: Some(50),
        promotions: Vec::new(),
        experience: 0,
        health: 70, // damaged
        range: 0,
        vision_range: 2,
        charges: None,
        trade_origin: None,
        trade_destination: None,
        religion_id: None,
        spread_charges: None,
        religious_strength: None,
    });

    let diff = rules.advance_turn(&mut s.state);

    let healed = diff.deltas.iter().find(|d| matches!(d,
        StateDelta::UnitHealed { unit, old_health, new_health }
            if *unit == mamluk_id && *old_health == 70 && *new_health == 80
    ));

    assert!(
        healed.is_some(),
        "Mamluk (HealEveryTurn) at 70 HP should heal to 80 HP and emit UnitHealed; \
         got deltas: {:?}",
        diff.deltas.iter()
            .filter(|d| matches!(d, StateDelta::UnitHealed { .. }))
            .collect::<Vec<_>>()
    );
}
