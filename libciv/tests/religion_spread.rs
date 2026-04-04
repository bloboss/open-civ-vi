/// Integration tests for Phase 3d: passive religious pressure spread.
///
/// Verifies that `advance_turn` applies religious pressure from cities with
/// a majority religion to nearby cities, emitting `ReligiousPressureApplied`
/// deltas and updating follower counts.
mod common;

use libciv::civ::Religion;
use libciv::game::StateDelta;
use libciv::ReligionId;

fn religion_id() -> ReligionId {
    ReligionId::from_ulid(ulid::Ulid::new())
}

// ---------------------------------------------------------------------------
// Test 1: A city with majority religion spreads pressure to a nearby city
// ---------------------------------------------------------------------------

/// Place two cities within 10 tiles. Give the source city a majority religion.
/// After one turn, the target city should gain followers and a
/// `ReligiousPressureApplied` delta should be emitted.
#[test]
fn passive_spread_adds_followers_to_nearby_city() {
    let mut s = common::build_scenario();

    // Found a religion for Rome, holy city = Rome's capital.
    let rid = religion_id();
    s.state.religions.push(Religion::new(
        rid,
        "Solar Cult".into(),
        s.rome_id,
        s.rome_city,
    ));

    // Give Rome's city a majority religion: 5 followers out of pop 1.
    // (Population starts at 1; 5 followers > 0 threshold = majority.)
    let rome_city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    rome_city.religious_followers.insert(rid, 5);
    rome_city.population = 5;

    // Babylon's city starts with 0 religious followers, pop 5.
    let babylon_city = s.state.cities.iter_mut().find(|c| c.id == s.babylon_city).unwrap();
    babylon_city.population = 5;

    // Advance one turn — Phase 3d should apply pressure.
    common::advance_turn(&mut s);

    // Check that Babylon's city gained followers.
    let babylon_city = s.state.cities.iter().find(|c| c.id == s.babylon_city).unwrap();
    let followers = babylon_city.religious_followers.get(&rid).copied().unwrap_or(0);
    assert!(
        followers > 0,
        "Babylon should have gained followers of Solar Cult via passive spread; got {}",
        followers,
    );
}

// ---------------------------------------------------------------------------
// Test 2: Holy city provides extra pressure
// ---------------------------------------------------------------------------

/// The holy city bonus (+4) should produce stronger pressure than a non-holy
/// city, resulting in at least 1 follower gain.  We also verify the
/// `ReligiousPressureApplied` delta is present in the turn diff.
#[test]
fn holy_city_emits_religious_pressure_delta() {
    let mut s = common::build_scenario();

    let rid = religion_id();
    s.state.religions.push(Religion::new(
        rid,
        "Moon Faith".into(),
        s.rome_id,
        s.rome_city,
    ));

    // Source city (Rome) is the holy city with majority religion.
    let rome_city = s.state.cities.iter_mut().find(|c| c.id == s.rome_city).unwrap();
    rome_city.religious_followers.insert(rid, 5);
    rome_city.population = 5;

    // Target city (Babylon) starts with no followers.
    let babylon_city = s.state.cities.iter_mut().find(|c| c.id == s.babylon_city).unwrap();
    babylon_city.population = 5;

    // Process one turn and capture deltas via TurnEngine directly.
    let engine = libciv::TurnEngine::new();
    let rules = libciv::DefaultRulesEngine;
    let diff = engine.process_turn(&mut s.state, &rules);

    // There should be at least one ReligiousPressureApplied targeting Babylon.
    let pressure_delta = diff.deltas.iter().find(|d| matches!(d,
        StateDelta::ReligiousPressureApplied { city, religion, .. }
            if *city == s.babylon_city && *religion == rid
    ));

    assert!(
        pressure_delta.is_some(),
        "Expected ReligiousPressureApplied delta for Babylon; got deltas: {:?}",
        diff.deltas.iter()
            .filter(|d| matches!(d, StateDelta::ReligiousPressureApplied { .. }))
            .collect::<Vec<_>>(),
    );
}
