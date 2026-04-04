/// Integration tests for natural wonder discovery events emitted by
/// `recalculate_visibility` when a civ first explores a tile containing
/// a natural wonder.
mod common;

use libciv::game::recalculate_visibility;
use libciv::game::StateDelta;
use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::world::wonder::{BuiltinNaturalWonder, GrandMesa};
use libciv::NaturalWonderId;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

fn wonder_id() -> NaturalWonderId {
    NaturalWonderId::from_ulid(ulid::Ulid::new())
}

// ---------------------------------------------------------------------------
// Test 1: Discovering a natural wonder emits NaturalWonderDiscovered
// ---------------------------------------------------------------------------

/// When a civ's unit can see a tile with a natural wonder for the first time,
/// `recalculate_visibility` should emit a `NaturalWonderDiscovered` delta.
#[test]
fn discovering_natural_wonder_emits_delta() {
    let mut s = common::build_scenario();

    // Place Grand Mesa on a tile near Rome's warrior at (5,3), within vision range 2.
    let wonder_coord = HexCoord::from_qr(6, 3);
    if let Some(tile) = s.state.board.tile_mut(wonder_coord) {
        tile.natural_wonder = Some(BuiltinNaturalWonder::GrandMesa(GrandMesa {
            id: wonder_id(),
        }));
    }

    // Clear Rome's explored tiles so the wonder tile is unexplored.
    let rome_civ = s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap();
    rome_civ.explored_tiles.clear();
    rome_civ.visible_tiles.clear();

    // Recalculate visibility for Rome — the warrior at (5,3) should see (6,3).
    let diff = recalculate_visibility(&mut s.state, s.rome_id);

    let discovered = diff.deltas.iter().find(|d| matches!(d,
        StateDelta::NaturalWonderDiscovered { civ, wonder_name, coord }
            if *civ == s.rome_id && *wonder_name == "Grand Mesa" && *coord == wonder_coord
    ));

    assert!(
        discovered.is_some(),
        "Expected NaturalWonderDiscovered delta for Grand Mesa at {:?}; got deltas: {:?}",
        wonder_coord,
        diff.deltas,
    );
}

// ---------------------------------------------------------------------------
// Test 2: Re-exploring the same wonder does NOT emit a second delta
// ---------------------------------------------------------------------------

/// After a wonder has already been explored, subsequent recalculate_visibility
/// calls must not emit another `NaturalWonderDiscovered` for the same tile.
#[test]
fn rediscovering_natural_wonder_does_not_emit_again() {
    let mut s = common::build_scenario();

    // Place Grand Mesa near Rome's warrior.
    let wonder_coord = HexCoord::from_qr(6, 3);
    if let Some(tile) = s.state.board.tile_mut(wonder_coord) {
        tile.natural_wonder = Some(BuiltinNaturalWonder::GrandMesa(GrandMesa {
            id: wonder_id(),
        }));
    }

    // Clear explored tiles so first call discovers the wonder.
    let rome_civ = s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap();
    rome_civ.explored_tiles.clear();
    rome_civ.visible_tiles.clear();

    // First call — should discover.
    let diff1 = recalculate_visibility(&mut s.state, s.rome_id);
    let count1 = diff1.deltas.iter().filter(|d| matches!(d,
        StateDelta::NaturalWonderDiscovered { .. }
    )).count();
    assert_eq!(count1, 1, "First call should emit exactly one NaturalWonderDiscovered");

    // Second call — wonder tile is already explored, no new discovery.
    let diff2 = recalculate_visibility(&mut s.state, s.rome_id);
    let count2 = diff2.deltas.iter().filter(|d| matches!(d,
        StateDelta::NaturalWonderDiscovered { .. }
    )).count();
    assert_eq!(count2, 0, "Second call should NOT emit NaturalWonderDiscovered again");
}

// ---------------------------------------------------------------------------
// Test 3: Wonder discovery awards era score via advance_turn
// ---------------------------------------------------------------------------

/// When a wonder is discovered during a turn, the era score observer in
/// advance_turn should award era score for the "Natural Wonder Discovered"
/// historic moment.
#[test]
fn wonder_discovery_grants_era_score_via_advance_turn() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Record initial era score.
    let initial_score = s.state.civilizations.iter()
        .find(|c| c.id == s.rome_id).unwrap()
        .era_score;

    // Place Grand Mesa within vision range of Rome's warrior at (5,3).
    let wonder_coord = HexCoord::from_qr(6, 3);
    if let Some(tile) = s.state.board.tile_mut(wonder_coord) {
        tile.natural_wonder = Some(BuiltinNaturalWonder::GrandMesa(GrandMesa {
            id: wonder_id(),
        }));
    }

    // Clear explored tiles so the wonder will be "discovered" during the turn.
    let rome_civ = s.state.civilizations.iter_mut()
        .find(|c| c.id == s.rome_id).unwrap();
    rome_civ.explored_tiles.clear();
    rome_civ.visible_tiles.clear();

    // Advance a turn — visibility refresh + era score observer should fire.
    let diff = rules.advance_turn(&mut s.state);

    // Check that NaturalWonderDiscovered delta was emitted.
    let discovered = diff.deltas.iter().any(|d| matches!(d,
        StateDelta::NaturalWonderDiscovered { civ, .. } if *civ == s.rome_id
    ));
    assert!(discovered, "advance_turn should emit NaturalWonderDiscovered");

    // Check that era score was awarded (3 points for wonder discovery).
    let has_moment = diff.deltas.iter().any(|d| matches!(d,
        StateDelta::HistoricMomentEarned { civ, moment, .. }
            if *civ == s.rome_id && *moment == "Natural Wonder Discovered"
    ));
    assert!(has_moment, "advance_turn should emit HistoricMomentEarned for wonder discovery");

    let final_score = s.state.civilizations.iter()
        .find(|c| c.id == s.rome_id).unwrap()
        .era_score;
    assert!(final_score > initial_score, "era score should increase after wonder discovery");
}
