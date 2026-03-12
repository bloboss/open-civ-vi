/// Tests for resource concealment by terrain features (Forest / Rainforest).
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::world::feature::BuiltinFeature;
use libciv::world::resource::{BuiltinResource, Wine};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Setup helper
// ---------------------------------------------------------------------------

/// Place Wine (Food+Gold, no reveal tech) under Forest on the Rome city center.
/// Returns the scenario with the city worked_tiles pointing at just that tile.
fn setup_wine_under_forest() -> common::Scenario {
    let mut s = common::build_scenario();
    let coord = HexCoord::from_qr(3, 3);  // Rome's city center (set in build_scenario)

    if let Some(tile) = s.state.board.tile_mut(coord) {
        tile.resource = Some(BuiltinResource::Wine(Wine));
        tile.feature  = Some(BuiltinFeature::Forest);
    }

    // Work only the center tile for a predictable yield.
    if let Some(city) = s.state.cities.iter_mut().find(|c| c.id == s.rome_city) {
        city.worked_tiles = vec![coord];
    }

    s
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Wine under Forest contributes no yields — the resource is concealed.
///
/// Expected: Grassland base (2 food) + Forest modifier (+1 prod). Wine's
/// Food (+1) and Gold (+1) are suppressed because Forest conceals resources.
#[test]
fn wine_under_forest_yields_are_concealed() {
    let s = setup_wine_under_forest();
    let engine = DefaultRulesEngine;
    let yields = engine.compute_yields(&s.state, s.rome_id);

    assert_eq!(yields.food, 2,   "Wine Food should be concealed by Forest");
    assert_eq!(yields.gold, 0,   "Wine Gold should be concealed by Forest");
    assert_eq!(yields.production, 1, "Forest +1 production still applies");
}

/// Wine on a tile where the Forest has been cleared appears in yields.
///
/// Expected: Grassland (2 food) + Wine (1 food, 1 gold) = 3 food, 1 gold.
#[test]
fn wine_with_forest_cleared_appears_in_yields() {
    let mut s = setup_wine_under_forest();

    // Clear the forest feature.
    let coord = HexCoord::from_qr(3, 3);
    if let Some(tile) = s.state.board.tile_mut(coord) {
        tile.feature = None;
    }

    let engine = DefaultRulesEngine;
    let yields = engine.compute_yields(&s.state, s.rome_id);

    assert_eq!(yields.food, 3,   "Grassland 2 + Wine 1 = 3 food after clearing Forest");
    assert_eq!(yields.gold, 1,   "Wine +1 gold should now appear");
}
