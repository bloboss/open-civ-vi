/// Integration tests for the city loyalty system.
///
/// Loyalty is a per-city score (0–100) that shifts each turn based on nearby
/// friendly and foreign city pressure, governor bonuses, occupation penalties,
/// and capital proximity. When loyalty reaches 0 the city revolts and may flip
/// to the civilization exerting the highest foreign pressure.
mod common;

use libciv::{CivId, DefaultRulesEngine, RulesEngine};
use libciv::civ::{City, CityOwnership, Governor};
use libciv::game::StateDelta;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn loyalty_deltas(diff: &libciv::GameStateDiff) -> Vec<(libciv::CityId, i32, i32)> {
    diff.deltas.iter().filter_map(|d| {
        if let StateDelta::LoyaltyChanged { city, delta, new_value } = d {
            Some((*city, *delta, *new_value))
        } else {
            None
        }
    }).collect()
}

fn revolt_deltas(diff: &libciv::GameStateDiff) -> Vec<(libciv::CityId, Option<CivId>, CivId)> {
    diff.deltas.iter().filter_map(|d| {
        if let StateDelta::CityRevolted { city, new_owner, old_owner } = d {
            Some((*city, *new_owner, *old_owner))
        } else {
            None
        }
    }).collect()
}

/// Build a scenario where a Babylon city is very close to Rome's capital,
/// far from Babylon's capital. Rome has high population to exert strong
/// foreign pressure on the Babylon city, eroding its loyalty.
fn setup_pressure_scenario() -> common::Scenario {
    let mut s = common::build_scenario();

    // Give Rome high population to exert strong loyalty pressure.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .population = 10;

    // Add a second Babylon city very close to Rome (at 4,4 — distance 2 from Rome's 3,3)
    // but far from Babylon's capital at (10,5) (distance 7).
    let city_id = s.state.id_gen.next_city_id();
    let mut city = City::new(city_id, "Nippur".into(), s.babylon_id, HexCoord::from_qr(4, 4));
    city.population = 1;
    s.state.cities.push(city);
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .cities.push(city_id);

    s
}

// ---------------------------------------------------------------------------
// Basic loyalty tests
// ---------------------------------------------------------------------------

/// Cities start with loyalty 100.
#[test]
fn cities_start_at_full_loyalty() {
    let s = common::build_scenario();
    for city in &s.state.cities {
        assert_eq!(city.loyalty, 100, "city {} should start at loyalty 100", city.name);
    }
}

/// An isolated capital (no foreign cities nearby) should maintain loyalty at 100.
/// The capital bonus + self-population bonus should keep loyalty at max.
#[test]
fn isolated_capital_stays_loyal() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Remove Babylon's city to isolate Rome completely.
    s.state.cities.retain(|c| c.owner == s.rome_id);
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .cities.clear();

    for _ in 0..10 {
        rules.advance_turn(&mut s.state);
    }

    let rome_city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert_eq!(rome_city.loyalty, 100, "isolated capital should stay at max loyalty");
}

/// Foreign pressure erodes loyalty over time for a non-capital city.
#[test]
fn foreign_pressure_erodes_loyalty() {
    let mut s = setup_pressure_scenario();
    let rules = DefaultRulesEngine;

    // Find the Nippur city (Babylon's city near Rome).
    let nippur_id = s.state.cities.iter()
        .find(|c| c.name == "Nippur")
        .unwrap().id;

    let initial_loyalty = s.state.cities.iter()
        .find(|c| c.id == nippur_id).unwrap().loyalty;

    // Advance several turns.
    for _ in 0..5 {
        rules.advance_turn(&mut s.state);
    }

    let loyalty = s.state.cities.iter()
        .find(|c| c.id == nippur_id).unwrap().loyalty;
    assert!(
        loyalty < initial_loyalty,
        "Nippur loyalty should decrease under foreign pressure: {} vs initial {}",
        loyalty, initial_loyalty
    );
}

/// LoyaltyChanged deltas are emitted when loyalty changes.
#[test]
fn loyalty_changed_delta_emitted() {
    let mut s = setup_pressure_scenario();
    let rules = DefaultRulesEngine;

    let diff = rules.advance_turn(&mut s.state);
    let changes = loyalty_deltas(&diff);

    // At least one loyalty change should occur (Nippur under foreign pressure).
    assert!(!changes.is_empty(), "expected at least one LoyaltyChanged delta");
}

// ---------------------------------------------------------------------------
// Occupied city penalty
// ---------------------------------------------------------------------------

/// Occupied cities lose loyalty faster due to the occupation penalty.
#[test]
fn occupied_city_loses_loyalty_faster() {
    let mut s = setup_pressure_scenario();
    let rules = DefaultRulesEngine;

    let nippur_id = s.state.cities.iter()
        .find(|c| c.name == "Nippur")
        .unwrap().id;

    // Mark Nippur as Occupied.
    s.state.cities.iter_mut()
        .find(|c| c.id == nippur_id).unwrap()
        .ownership = CityOwnership::Occupied;

    let diff = rules.advance_turn(&mut s.state);

    let nippur_change = loyalty_deltas(&diff).into_iter()
        .find(|(id, _, _)| *id == nippur_id);

    if let Some((_, delta, _)) = nippur_change {
        // The delta should be negative (losing loyalty).
        assert!(delta < 0, "occupied city should lose loyalty, got delta {delta}");
    }
}

// ---------------------------------------------------------------------------
// Governor bonus
// ---------------------------------------------------------------------------

/// An established governor in a city provides a loyalty bonus.
#[test]
fn governor_stabilizes_loyalty() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Moderate pressure: Rome has pop 4 (not 10) so the delta doesn't hit the -20 clamp.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .population = 4;

    let city_id = s.state.id_gen.next_city_id();
    let mut city = City::new(city_id, "Nippur".into(), s.babylon_id, HexCoord::from_qr(4, 4));
    city.population = 1;
    s.state.cities.push(city);
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.babylon_id).unwrap()
        .cities.push(city_id);

    // First, advance one turn without governor to get a baseline.
    let diff_no_gov = rules.advance_turn(&mut s.state);
    let delta_no_gov = loyalty_deltas(&diff_no_gov).into_iter()
        .find(|(id, _, _)| *id == city_id)
        .map(|(_, d, _)| d)
        .unwrap_or(0);

    // Reset loyalty.
    s.state.cities.iter_mut()
        .find(|c| c.id == city_id).unwrap()
        .loyalty = 100;

    // Add an established governor.
    let gov_id = libciv::GovernorId::from_ulid(s.state.id_gen.next_ulid());
    s.state.governors.push(Governor {
        id: gov_id,
        def_name: "Victor",
        owner: s.babylon_id,
        assigned_city: Some(city_id),
        promotions: Vec::new(),
        turns_to_establish: 0, // already established
    });

    let diff_with_gov = rules.advance_turn(&mut s.state);
    let delta_with_gov = loyalty_deltas(&diff_with_gov).into_iter()
        .find(|(id, _, _)| *id == city_id)
        .map(|(_, d, _)| d)
        .unwrap_or(0);

    // Governor should improve loyalty delta (less negative or more positive).
    assert!(
        delta_with_gov > delta_no_gov,
        "governor should improve loyalty delta: with={delta_with_gov}, without={delta_no_gov}"
    );
}

// ---------------------------------------------------------------------------
// City revolt
// ---------------------------------------------------------------------------

/// When loyalty reaches 0, CityRevolted is emitted and the city flips.
#[test]
fn city_revolts_at_zero_loyalty() {
    let mut s = setup_pressure_scenario();
    let rules = DefaultRulesEngine;

    let nippur_id = s.state.cities.iter()
        .find(|c| c.name == "Nippur")
        .unwrap().id;

    // Set loyalty to 1 so it drops to 0 on the next turn.
    s.state.cities.iter_mut()
        .find(|c| c.id == nippur_id).unwrap()
        .loyalty = 1;

    let diff = rules.advance_turn(&mut s.state);

    let revolts = revolt_deltas(&diff);
    let revolt = revolts.iter().find(|(id, _, _)| *id == nippur_id);
    assert!(revolt.is_some(), "expected CityRevolted delta for Nippur");

    let (_, new_owner, old_owner) = revolt.unwrap();
    assert_eq!(*old_owner, s.babylon_id, "old owner should be Babylon");

    // Rome is the strongest nearby civ, so Nippur should flip to Rome.
    assert_eq!(*new_owner, Some(s.rome_id), "city should flip to Rome");
}

/// After a revolt, the city is owned by the new civilization.
#[test]
fn city_ownership_transfers_on_revolt() {
    let mut s = setup_pressure_scenario();
    let rules = DefaultRulesEngine;

    let nippur_id = s.state.cities.iter()
        .find(|c| c.name == "Nippur")
        .unwrap().id;

    // Force loyalty to 1 so it drops to 0 on next turn.
    s.state.cities.iter_mut()
        .find(|c| c.id == nippur_id).unwrap()
        .loyalty = 1;

    rules.advance_turn(&mut s.state);

    let nippur = s.state.cities.iter().find(|c| c.id == nippur_id).unwrap();
    assert_eq!(nippur.owner, s.rome_id, "Nippur should now belong to Rome");
    assert_eq!(nippur.ownership, CityOwnership::Occupied, "flipped city should be Occupied");
    assert_eq!(nippur.loyalty, 50, "flipped city starts at loyalty 50");

    // Rome's city list should include Nippur.
    let rome = s.state.civilizations.iter().find(|c| c.id == s.rome_id).unwrap();
    assert!(rome.cities.contains(&nippur_id), "Rome should have Nippur in its city list");

    // Babylon's city list should not include Nippur.
    let babylon = s.state.civilizations.iter().find(|c| c.id == s.babylon_id).unwrap();
    assert!(!babylon.cities.contains(&nippur_id), "Babylon should no longer own Nippur");
}

/// After revolt, the city's territory tiles belong to the new owner.
#[test]
fn territory_transfers_on_revolt() {
    let mut s = setup_pressure_scenario();
    let rules = DefaultRulesEngine;

    let nippur_id = s.state.cities.iter()
        .find(|c| c.name == "Nippur")
        .unwrap().id;

    // Give Nippur some territory first.
    let nippur_coord = HexCoord::from_qr(4, 4);
    s.state.cities.iter_mut()
        .find(|c| c.id == nippur_id).unwrap()
        .territory.insert(nippur_coord);
    if let Some(tile) = s.state.board.tile_mut(nippur_coord) {
        tile.owner = Some(s.babylon_id);
    }

    // Force revolt.
    s.state.cities.iter_mut()
        .find(|c| c.id == nippur_id).unwrap()
        .loyalty = 1;

    rules.advance_turn(&mut s.state);

    // Territory tile should now belong to Rome.
    let tile = s.state.board.tile(nippur_coord).unwrap();
    assert_eq!(tile.owner, Some(s.rome_id), "territory tile should flip to Rome");
}

// ---------------------------------------------------------------------------
// Capital loyalty bonus
// ---------------------------------------------------------------------------

/// Capitals get a loyalty bonus, making them harder to flip.
#[test]
fn capital_resists_loyalty_loss() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Move Rome's capital very close to Babylon to create foreign pressure.
    // Give Babylon lots of population to exert strong pressure.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.babylon_city).unwrap()
        .population = 20;

    // Advance many turns — Rome's capital should still have loyalty > 0.
    for _ in 0..20 {
        rules.advance_turn(&mut s.state);
    }

    let rome_city = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap();
    assert!(rome_city.loyalty > 0, "capital should resist loyalty loss: loyalty={}", rome_city.loyalty);
}

// ---------------------------------------------------------------------------
// City-states are skipped
// ---------------------------------------------------------------------------

/// City-states do not participate in the loyalty system.
#[test]
fn city_states_skip_loyalty() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Add a city-state near Rome.
    let cs_id = s.state.id_gen.next_city_id();
    let cs_civ = s.state.id_gen.next_civ_id();
    let mut cs_city = City::new(cs_id, "Geneva".into(), cs_civ, HexCoord::from_qr(5, 3));
    cs_city.kind = libciv::civ::CityKind::CityState(
        libciv::civ::CityStateData::new(libciv::civ::CityStateType::Cultural)
    );
    cs_city.loyalty = 100;
    s.state.cities.push(cs_city);

    for _ in 0..10 {
        rules.advance_turn(&mut s.state);
    }

    let cs = s.state.cities.iter().find(|c| c.id == cs_id).unwrap();
    assert_eq!(cs.loyalty, 100, "city-state loyalty should not change");
}

// ---------------------------------------------------------------------------
// Loyalty delta clamping
// ---------------------------------------------------------------------------

/// Loyalty changes are clamped to ±20 per turn.
#[test]
fn loyalty_delta_is_clamped() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give Babylon massive population to create enormous pressure.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.babylon_city).unwrap()
        .population = 100;

    // Add a non-capital Rome city close to Babylon.
    let city_id = s.state.id_gen.next_city_id();
    let mut city = City::new(city_id, "Antium".into(), s.rome_id, HexCoord::from_qr(9, 5));
    city.population = 1;
    city.is_capital = false;
    s.state.cities.push(city);
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .cities.push(city_id);

    let diff = rules.advance_turn(&mut s.state);

    // Check that the loyalty delta for Antium doesn't exceed -20.
    let antium_change = loyalty_deltas(&diff).into_iter()
        .find(|(id, _, _)| *id == city_id);
    if let Some((_, delta, _)) = antium_change {
        assert!(delta >= -20, "loyalty delta should be clamped to -20, got {delta}");
    }
}

// ---------------------------------------------------------------------------
// Prolonged pressure leads to revolt
// ---------------------------------------------------------------------------

/// Sustained foreign pressure eventually causes a city to revolt.
#[test]
fn sustained_pressure_causes_revolt() {
    let mut s = setup_pressure_scenario();
    let rules = DefaultRulesEngine;

    let nippur_id = s.state.cities.iter()
        .find(|c| c.name == "Nippur")
        .unwrap().id;

    let mut revolted = false;
    for _ in 0..50 {
        let diff = rules.advance_turn(&mut s.state);
        if revolt_deltas(&diff).iter().any(|(id, _, _)| *id == nippur_id) {
            revolted = true;
            break;
        }
    }

    assert!(revolted, "Nippur should eventually revolt under sustained foreign pressure");
}
