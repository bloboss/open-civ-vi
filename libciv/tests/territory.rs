/// Integration tests for tile claiming and territory expansion.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::civ::City;
use libciv::game::{RulesError, StateDelta};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libciv::{UnitCategory, UnitDomain};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn count_tile_claimed(diff: &libciv::GameStateDiff) -> usize {
    diff.deltas.iter().filter(|d| matches!(d, StateDelta::TileClaimed { .. })).count()
}

// ---------------------------------------------------------------------------
// claim_tile — basic cases
// ---------------------------------------------------------------------------

/// Claiming an unclaimed tile within range succeeds and emits TileClaimed.
#[test]
fn claim_unclaimed_tile_succeeds() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    // Rome city at (3,3); target at (4,3) — distance 1 (within 1–3 range).
    let target = HexCoord::from_qr(4, 3);

    let diff = rules.claim_tile(&mut s.state, s.rome_city, target, false).unwrap();

    assert_eq!(diff.deltas.len(), 1);
    match diff.deltas[0] {
        StateDelta::TileClaimed { civ, city, coord } => {
            assert_eq!(civ, s.rome_id);
            assert_eq!(city, s.rome_city);
            assert_eq!(coord, target);
        }
        ref other => panic!("expected TileClaimed, got {other:?}"),
    }
    assert_eq!(s.state.board.tile(target).unwrap().owner, Some(s.rome_id));
}

/// Claiming the same tile again (same civ) is idempotent — empty diff returned.
#[test]
fn claim_already_owned_tile_is_idempotent() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    let target = HexCoord::from_qr(4, 3);

    // First claim.
    rules.claim_tile(&mut s.state, s.rome_city, target, false).unwrap();
    // Second claim — should succeed with empty diff.
    let diff = rules.claim_tile(&mut s.state, s.rome_city, target, false).unwrap();
    assert!(diff.is_empty(), "expected empty diff on idempotent claim");
}

/// Claiming a tile owned by a different civ returns TileOwnedByEnemy when force=false.
#[test]
fn claim_enemy_tile_returns_error() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    // Mark (5,3) — distance 2 from Rome — as Babylon-owned.
    let contested = HexCoord::from_qr(5, 3);
    if let Some(t) = s.state.board.tile_mut(contested) {
        t.owner = Some(s.babylon_id);
    }

    let err = rules.claim_tile(&mut s.state, s.rome_city, contested, false).unwrap_err();
    assert!(matches!(err, RulesError::TileOwnedByEnemy), "got {err:?}");
}

/// Claiming a tile more than 3 tiles from the city center returns TileNotInCityRange.
#[test]
fn claim_tile_out_of_range_returns_error() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    // (7,3) is distance 4 from Rome at (3,3): max(4,0,4)=4 > 3.
    let far_tile = HexCoord::from_qr(7, 3);

    let err = rules.claim_tile(&mut s.state, s.rome_city, far_tile, false).unwrap_err();
    assert!(matches!(err, RulesError::TileNotInCityRange), "got {err:?}");
}

// ---------------------------------------------------------------------------
// claim_tile — force flag (culture flip)
// ---------------------------------------------------------------------------

/// force=true allows claiming a tile currently owned by an enemy civilization.
/// The new owner is set in state and TileClaimed is emitted.
#[test]
fn force_claim_enemy_tile_succeeds() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    // (5,3) is distance 2 from Rome — pre-assign to Babylon.
    let contested = HexCoord::from_qr(5, 3);
    if let Some(t) = s.state.board.tile_mut(contested) {
        t.owner = Some(s.babylon_id);
    }

    let diff = rules.claim_tile(&mut s.state, s.rome_city, contested, true).unwrap();

    // A TileClaimed delta must be emitted for Rome.
    match diff.deltas.as_slice() {
        [StateDelta::TileClaimed { civ, city, coord }] => {
            assert_eq!(*civ, s.rome_id);
            assert_eq!(*city, s.rome_city);
            assert_eq!(*coord, contested);
        }
        other => panic!("expected exactly one TileClaimed, got {other:?}"),
    }
    // Ownership transferred in state.
    assert_eq!(
        s.state.board.tile(contested).unwrap().owner,
        Some(s.rome_id),
        "tile should now belong to Rome"
    );
}

/// force=true on a tile already owned by the same civ is still idempotent.
#[test]
fn force_claim_own_tile_is_idempotent() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    let target = HexCoord::from_qr(4, 3);

    rules.claim_tile(&mut s.state, s.rome_city, target, false).unwrap();
    let diff = rules.claim_tile(&mut s.state, s.rome_city, target, true).unwrap();

    assert!(diff.is_empty(), "force=true on own tile should be idempotent");
}

// ---------------------------------------------------------------------------
// reassign_tile — intra-civ city reassignment
// ---------------------------------------------------------------------------

/// Helper: add a second city for Rome at `coord`.
fn add_rome_city(s: &mut common::Scenario, coord: HexCoord) -> libciv::CityId {
    let city_id = s.state.id_gen.next_city_id();
    s.state.cities.push(City::new(city_id, "Antium".into(), s.rome_id, coord));
    s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap()
        .cities.push(city_id);
    city_id
}

/// A tile owned by Rome can be reassigned from one Rome city to another.
/// TileReassigned is emitted; tile.owner stays Rome.
#[test]
fn reassign_tile_within_same_civ_succeeds() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Antium at (7,0). Target at (4,2):
    //   distance from Roma  (3,3) = max(1,1,2) = 2  ≤ 3 ✓
    //   distance from Antium(7,0) = max(3,2,1) = 3  ≤ 3 ✓
    let antium_coord = HexCoord::from_qr(7, 0);
    let antium = add_rome_city(&mut s, antium_coord);

    let target = HexCoord::from_qr(4, 2);
    // Mark tile as owned by Rome (claimed by Roma).
    if let Some(t) = s.state.board.tile_mut(target) {
        t.owner = Some(s.rome_id);
    }

    let diff = rules.reassign_tile(&mut s.state, s.rome_city, antium, target).unwrap();

    match diff.deltas.as_slice() {
        [StateDelta::TileReassigned { civ, from_city, to_city, coord }] => {
            assert_eq!(*civ, s.rome_id);
            assert_eq!(*from_city, s.rome_city);
            assert_eq!(*to_city, antium);
            assert_eq!(*coord, target);
        }
        other => panic!("expected exactly one TileReassigned, got {other:?}"),
    }
    // Tile still belongs to Rome (civ unchanged).
    assert_eq!(s.state.board.tile(target).unwrap().owner, Some(s.rome_id));
}

/// Reassigning a tile when from_city == to_city is idempotent — empty diff.
#[test]
fn reassign_tile_same_city_is_idempotent() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let target = HexCoord::from_qr(4, 3);
    if let Some(t) = s.state.board.tile_mut(target) {
        t.owner = Some(s.rome_id);
    }

    let diff = rules.reassign_tile(&mut s.state, s.rome_city, s.rome_city, target).unwrap();
    assert!(diff.is_empty(), "same-city reassign should be idempotent");
}

/// Reassigning a tile to a city of a different civilization fails with CitiesNotSameCiv.
#[test]
fn reassign_tile_cross_civ_fails() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let target = HexCoord::from_qr(4, 3);
    if let Some(t) = s.state.board.tile_mut(target) {
        t.owner = Some(s.rome_id);
    }

    // from_city = Rome's city, to_city = Babylon's city — different civs.
    let err = rules.reassign_tile(&mut s.state, s.rome_city, s.babylon_city, target).unwrap_err();
    assert!(matches!(err, RulesError::CitiesNotSameCiv), "got {err:?}");
}

/// Reassigning an unclaimed tile fails with TileNotOwned.
#[test]
fn reassign_tile_unclaimed_fails() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Antium at (7,0); target at (4,2) in range of both.
    let antium = add_rome_city(&mut s, HexCoord::from_qr(7, 0));
    let target = HexCoord::from_qr(4, 2);
    // Leave tile unclaimed (owner = None).

    let err = rules.reassign_tile(&mut s.state, s.rome_city, antium, target).unwrap_err();
    assert!(matches!(err, RulesError::TileNotOwned), "got {err:?}");
}

/// Reassigning to a city that is out of range of the tile fails with TileNotInCityRange.
#[test]
fn reassign_tile_to_out_of_range_city_fails() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Antium far away at (0, 0). Target at (4,3) — distance 4 from (0,0).
    let antium = add_rome_city(&mut s, HexCoord::from_qr(0, 0));
    let target = HexCoord::from_qr(4, 3);
    if let Some(t) = s.state.board.tile_mut(target) {
        t.owner = Some(s.rome_id);
    }

    let err = rules.reassign_tile(&mut s.state, s.rome_city, antium, target).unwrap_err();
    assert!(matches!(err, RulesError::TileNotInCityRange), "got {err:?}");
}

// ---------------------------------------------------------------------------
// found_city emits TileClaimed
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Cultural border expansion (advance_turn Phase 3b)
// ---------------------------------------------------------------------------

/// Helper: cost in shadow-culture to claim a tile at `distance` from the city center.
/// Mirrors `tile_border_cost` in rules.rs.
fn border_cost(distance: u32) -> u32 {
    (10.0 + (6.0 * distance as f64).powf(1.3)) as u32
}

/// Every city contributes 1 base culture/turn to its shadow border accumulator,
/// even when no worked tile yields culture.
#[test]
fn border_accumulates_base_culture_each_turn() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    assert_eq!(s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().culture_border, 0);

    // After N turns, culture_border should equal N (base 1/turn, no tile culture,
    // ring-2 cost ~35 so no tile is claimed yet).
    for n in 1..=5u32 {
        rules.advance_turn(&mut s.state);
        let border = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().culture_border;
        assert_eq!(border, n, "after {n} turn(s), culture_border should be {n}");
    }
}

/// Seeding a city with enough culture causes the expansion phase to claim a ring-2 tile.
/// The TileClaimed delta for that tile must appear in the advance_turn diff.
#[test]
fn border_expands_when_sufficient_culture() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Seed culture_border with exactly enough to claim one ring-2 tile.
    let ring2_cost = border_cost(2);
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .culture_border = ring2_cost;

    let diff = rules.advance_turn(&mut s.state);

    // At least one TileClaimed delta must be present.
    let claimed: Vec<_> = diff.deltas.iter().filter_map(|d| {
        if let libciv::game::StateDelta::TileClaimed { civ, city, coord } = d {
            Some((*civ, *city, *coord))
        } else {
            None
        }
    }).collect();
    assert!(!claimed.is_empty(), "expected at least one TileClaimed when culture is sufficient");

    // All claimed tiles must belong to Rome's city.
    let city_coord = HexCoord::from_qr(3, 3);
    for (civ, city, coord) in &claimed {
        assert_eq!(*civ, s.rome_id, "claimed tile should belong to Rome");
        assert_eq!(*city, s.rome_city);
        let dist = city_coord.distance(coord);
        assert!((2..=5).contains(&dist), "claimed tile at dist {dist} must be 2–5 from city");
    }
}

/// With exactly ring-2 cost available, the expansion claims a ring-2 tile (not ring-3).
/// After the claim, culture_border drops to 0 (ring-2 cost is higher than 0 remaining).
#[test]
fn border_prefers_ring2_over_ring3() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let ring2_cost = border_cost(2);
    let ring3_cost = border_cost(3);
    assert!(ring2_cost < ring3_cost, "ring-2 should be cheaper than ring-3");

    // Set budget to exactly the ring-2 cost (not enough for ring-3).
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .culture_border = ring2_cost;

    let diff = rules.advance_turn(&mut s.state);

    let city_coord = HexCoord::from_qr(3, 3);

    // Exactly one TileClaimed should have been emitted.
    let expansion_claims: Vec<_> = diff.deltas.iter().filter_map(|d| {
        if let libciv::game::StateDelta::TileClaimed { coord, city, civ } = d {
            if *city == s.rome_city {
                Some((*civ, *city, *coord))
            } else { None }
        } else { None }
    }).collect();
    assert_eq!(expansion_claims.len(), 1, "exactly one tile should be claimed");

    // The claimed tile must be at ring-2 distance.
    let (_, _, coord) = expansion_claims[0];
    let dist = city_coord.distance(&coord);
    assert_eq!(dist, 2, "claimed tile should be at ring-2 (cheapest), got dist {dist}");

    // advance_turn adds 1 base culture before spending, so after claiming the
    // ring-2 tile the remainder is 1 (not enough for ring-3 at ~52).
    let remaining = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().culture_border;
    assert_eq!(remaining, 1);
    assert!(remaining < ring3_cost, "remaining {remaining} must be below ring-3 cost {ring3_cost}");
}

/// Cultural expansion never claims a tile beyond radius 5 from the city center.
#[test]
fn border_stops_at_radius_5() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Give the city a huge culture budget — enough to claim many tiles.
    s.state.cities.iter_mut()
        .find(|c| c.id == s.rome_city).unwrap()
        .culture_border = 100_000;

    let diff = rules.advance_turn(&mut s.state);

    let city_coord = HexCoord::from_qr(3, 3);
    for delta in &diff.deltas {
        if let libciv::game::StateDelta::TileClaimed { city, coord, .. } = delta {
            if *city == s.rome_city {
                let dist = city_coord.distance(coord);
                assert!(
                    dist <= 5,
                    "claimed tile at dist {dist} exceeds maximum expansion radius of 5"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// found_city emits TileClaimed
// ---------------------------------------------------------------------------

/// `found_city` diff contains TileClaimed deltas for the city center and its
/// ring-1 neighbours.
#[test]
fn found_city_emits_tile_claimed_deltas() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Place a settler for Rome at (7,0): distance to Rome (3,3) = max(4,3,3)=4 > 3.
    // Distance to Babylon (10,5) = max(3,5,8)=8 > 3. Valid founding site.
    let settler_coord = HexCoord::from_qr(7, 0);
    let settler_id = s.state.id_gen.next_unit_id();
    s.state.units.push(libciv::civ::BasicUnit {
        id: settler_id,
        unit_type: s.settler_type,
        owner: s.rome_id,
        coord: settler_coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Civilian,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    let diff = rules.found_city(&mut s.state, settler_id, "Antium".into()).unwrap();

    // Diff must contain CityFounded.
    assert!(
        diff.deltas.iter().any(|d| matches!(d, StateDelta::CityFounded { .. })),
        "expected CityFounded delta"
    );

    // Extract founded city_id.
    let city_id = diff.deltas.iter().find_map(|d| {
        if let StateDelta::CityFounded { city, .. } = d { Some(*city) } else { None }
    }).expect("CityFounded not found");

    // There must be at least one TileClaimed (city center at minimum).
    let claimed_count = count_tile_claimed(&diff);
    assert!(claimed_count >= 1, "expected at least 1 TileClaimed, got {claimed_count}");

    // The city center itself must be claimed.
    assert!(
        diff.deltas.iter().any(|d| matches!(d,
            StateDelta::TileClaimed { city, coord, .. }
            if *city == city_id && *coord == settler_coord
        )),
        "city center tile not claimed"
    );

    // All TileClaimed deltas belong to Rome and the new city.
    for delta in &diff.deltas {
        if let StateDelta::TileClaimed { civ, city, .. } = delta {
            assert_eq!(*civ, s.rome_id);
            assert_eq!(*city, city_id);
        }
    }

    // Tile ownership is set in state.
    assert_eq!(
        s.state.board.tile(settler_coord).unwrap().owner,
        Some(s.rome_id),
        "city center tile not owned by Rome after founding"
    );
}
