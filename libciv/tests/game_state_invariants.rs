//! Game state invariant tests.
//!
//! These tests verify structural invariants that should hold after
//! turn processing — ownership consistency, visibility coverage,
//! terrain validity, and production progress.
mod common;

use libciv::game::recalculate_visibility;
use libciv::{DefaultRulesEngine, TurnEngine};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::MovementCost;

// ── tests ────────────────────────────────────────────────────────────────────

/// After advance_turn, units must not be silently removed from the state
/// (absent actual combat or defeat).
#[test]
fn advance_turn_does_not_lose_units() {
    let mut s = common::build_scenario();
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;

    let units_before = s.state.units.len();

    for _ in 0..5 {
        engine.process_turn(&mut s.state, &rules);
        for unit in &mut s.state.units {
            unit.movement_left = unit.max_movement;
        }
    }

    assert_eq!(
        s.state.units.len(),
        units_before,
        "units must not be silently dropped by advance_turn (no combat occurred)"
    );
}

/// Cities with a queued production item and a worked Plains tile should
/// accumulate production over turns.
#[test]
fn advance_turn_production_progress() {
    use libciv::civ::ProductionItem;
    use libciv::world::terrain::BuiltinTerrain;

    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;
    let engine = TurnEngine::new();

    // Set a tile adjacent to Rome's city to Plains (1 food + 1 production)
    // and add it to the city's worked tiles.
    let plains_coord = HexCoord::from_qr(4, 3);
    if let Some(tile) = s.state.board.tile_mut(plains_coord) {
        tile.terrain = BuiltinTerrain::Plains;
    }
    s.state
        .cities
        .iter_mut()
        .find(|c| c.id == s.rome_city)
        .unwrap()
        .worked_tiles
        .push(plains_coord);

    // Queue a warrior in Rome's city.
    s.state
        .cities
        .iter_mut()
        .find(|c| c.id == s.rome_city)
        .unwrap()
        .production_queue
        .push_back(ProductionItem::Unit(s.warrior_type));

    let stored_before = s.state.city(s.rome_city).unwrap().production_stored;

    // Run a few turns to accumulate production.
    for _ in 0..3 {
        engine.process_turn(&mut s.state, &rules);
        for unit in &mut s.state.units {
            unit.movement_left = unit.max_movement;
        }
    }

    let stored_after = s.state.city(s.rome_city).unwrap().production_stored;

    // Production should have progressed (even if the item completed and
    // storage was reset, the unit count should have grown).
    let unit_count_grew = s.state.units.len() > 2; // started with 2 warriors
    assert!(
        stored_after > stored_before || unit_count_grew,
        "production must progress: stored went from {} to {}, units={}",
        stored_before,
        stored_after,
        s.state.units.len()
    );
}

/// After recalculate_visibility, tiles adjacent to a city must be visible.
#[test]
fn visibility_covers_city_radius() {
    let mut s = common::build_scenario();
    recalculate_visibility(&mut s.state, s.rome_id);

    let rome_city = s.state.city(s.rome_city).unwrap();
    let city_coord = rome_city.coord;
    let civ = s.state.civ(s.rome_id).unwrap();

    // The city tile itself must be visible.
    assert!(
        civ.visible_tiles.contains(&city_coord),
        "city tile must be visible"
    );

    // All immediate neighbors that are on the board must be visible.
    for neighbor in city_coord.neighbors() {
        if s.state.board.normalize(neighbor).is_some() {
            assert!(
                civ.visible_tiles.contains(&neighbor),
                "neighbor {:?} of city must be visible",
                neighbor
            );
        }
    }
}

/// No unit should ever end up on impassable terrain (Mountain/Ocean) after
/// turn processing.
#[test]
fn all_units_on_passable_terrain() {
    let mut s = common::build_scenario();
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;

    for _ in 0..5 {
        engine.process_turn(&mut s.state, &rules);
        for unit in &mut s.state.units {
            unit.movement_left = unit.max_movement;
        }
    }

    for unit in &s.state.units {
        if let Some(tile) = s.state.board.tile(unit.coord) {
            use libhexgrid::board::HexTile;
            assert!(
                tile.movement_cost() != MovementCost::Impassable,
                "unit {:?} at {:?} is on impassable terrain ({:?})",
                unit.id,
                unit.coord,
                tile.terrain
            );
        }
    }
}

/// Every city's owner must appear in the civilizations list, and the
/// owning civ must list that city in its `cities` vec.
#[test]
fn city_owner_matches_civ_cities_list() {
    let mut s = common::build_scenario();
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;

    // Run a few turns so cities can potentially change state.
    for _ in 0..5 {
        engine.process_turn(&mut s.state, &rules);
    }

    for city in &s.state.cities {
        let civ = s.state
            .civ(city.owner)
            .unwrap_or_else(|| panic!("city {:?} owner {:?} not found in civs", city.id, city.owner));
        assert!(
            civ.cities.contains(&city.id),
            "civ {:?} does not list city {:?} in its cities vec",
            civ.id,
            city.id
        );
    }
}

/// Visibility should grow monotonically: explored_tiles never shrinks.
#[test]
fn explored_tiles_never_shrink() {
    let mut s = common::build_scenario();
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;

    let mut prev_rome_explored = s.state.civ(s.rome_id).unwrap().explored_tiles.len();

    for _ in 0..5 {
        engine.process_turn(&mut s.state, &rules);
        for unit in &mut s.state.units {
            unit.movement_left = unit.max_movement;
        }
        let civ_ids: Vec<_> = s.state.civilizations.iter().map(|c| c.id).collect();
        for cid in &civ_ids {
            recalculate_visibility(&mut s.state, *cid);
        }

        let current = s.state.civ(s.rome_id).unwrap().explored_tiles.len();
        assert!(
            current >= prev_rome_explored,
            "explored_tiles must never shrink: was {}, now {}",
            prev_rome_explored,
            current
        );
        prev_rome_explored = current;
    }
}

/// The board coordinate normalization must round-trip correctly: normalizing
/// a valid coord returns the same coord.
#[test]
fn board_normalize_round_trip() {
    let s = common::build_scenario();

    for coord in s.state.board.all_coords() {
        let normalized = s.state.board.normalize(coord);
        assert_eq!(
            normalized,
            Some(coord),
            "normalizing a valid board coord {:?} should return itself",
            coord
        );
    }
}

/// Turn counter must advance exactly once per process_turn call.
#[test]
fn turn_counter_advances_correctly() {
    let mut s = common::build_scenario();
    let engine = TurnEngine::new();
    let rules = DefaultRulesEngine;

    assert_eq!(s.state.turn, 0, "initial turn must be 0");

    for expected in 1..=10u32 {
        engine.process_turn(&mut s.state, &rules);
        assert_eq!(
            s.state.turn, expected,
            "turn counter should be {} after {} process_turn calls",
            expected, expected
        );
    }
}
