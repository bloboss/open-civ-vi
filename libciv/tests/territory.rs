/// Integration tests for tile claiming and territory expansion.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
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
// claim_tile tests
// ---------------------------------------------------------------------------

/// Claiming an unclaimed tile within range succeeds and emits TileClaimed.
#[test]
fn claim_unclaimed_tile_succeeds() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    // Rome city at (3,3); target at (4,3) — distance 1 (within 1–3 range).
    let target = HexCoord::from_qr(4, 3);

    let diff = rules.claim_tile(&mut s.state, s.rome_city, target).unwrap();

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
    rules.claim_tile(&mut s.state, s.rome_city, target).unwrap();
    // Second claim — should succeed with empty diff.
    let diff = rules.claim_tile(&mut s.state, s.rome_city, target).unwrap();
    assert!(diff.is_empty(), "expected empty diff on idempotent claim");
}

/// Claiming a tile owned by a different civ returns TileOwnedByEnemy.
#[test]
fn claim_enemy_tile_returns_error() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    // Mark (5,3) — distance 2 from Rome — as Babylon-owned.
    let contested = HexCoord::from_qr(5, 3);
    if let Some(t) = s.state.board.tile_mut(contested) {
        t.owner = Some(s.babylon_id);
    }

    let err = rules.claim_tile(&mut s.state, s.rome_city, contested).unwrap_err();
    assert!(matches!(err, RulesError::TileOwnedByEnemy), "got {err:?}");
}

/// Claiming a tile more than 3 tiles from the city center returns TileNotInCityRange.
#[test]
fn claim_tile_out_of_range_returns_error() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    // (7,3) is distance 4 from Rome at (3,3): max(4,0,4)=4 > 3.
    let far_tile = HexCoord::from_qr(7, 3);

    let err = rules.claim_tile(&mut s.state, s.rome_city, far_tile).unwrap_err();
    assert!(matches!(err, RulesError::TileNotInCityRange), "got {err:?}");
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
        health: 100,
        range: 0,
        vision_range: 2,
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
