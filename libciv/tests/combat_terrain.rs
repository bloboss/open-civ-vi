/// Tests verifying that terrain grants or penalizes combat defense bonuses.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::game::StateDelta;
use libciv::world::feature::BuiltinFeature;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Bring two warriors adjacent, execute attack, and return the damage dealt
/// to the defender. Uses a fixed RNG seed for determinism.
fn defender_damage_at(
    s: &mut common::Scenario,
    def_coord: HexCoord,
    terrain_setup: impl FnOnce(&mut common::Scenario, HexCoord),
) -> u32 {
    terrain_setup(s, def_coord);

    // Teleport both units so they are adjacent: attacker at (7,5), defender at def_coord.
    if let Some(u) = s.state.unit_mut(s.rome_warrior) {
        u.coord = HexCoord::from_qr(7, 5);
    }
    if let Some(u) = s.state.unit_mut(s.babylon_warrior) {
        u.coord = def_coord;
    }

    // Perform the attack.
    let rules = DefaultRulesEngine;
    let diff = rules.attack(&mut s.state, s.rome_warrior, s.babylon_warrior).unwrap();

    // Extract defender_damage from the diff.
    diff.deltas.iter().find_map(|d| {
        if let StateDelta::UnitAttacked { defender_damage, .. } = d {
            Some(*defender_damage)
        } else {
            None
        }
    }).expect("UnitAttacked delta missing")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A unit on hills takes less damage than the same unit on flat ground.
/// (Higher effective combat strength for the defender → attacker's formula yields less damage.)
#[test]
fn hills_defender_takes_less_damage() {
    // Flat-ground reference.
    let mut s_flat = common::build_scenario();
    let damage_flat = defender_damage_at(&mut s_flat, HexCoord::from_qr(8, 5), |_, _| {});

    // Hills: set Babylon's warrior tile to hills=true.
    let mut s_hills = common::build_scenario();
    let damage_hills = defender_damage_at(&mut s_hills, HexCoord::from_qr(8, 5), |s, coord| {
        if let Some(t) = s.state.board.tile_mut(coord) {
            t.hills = true;
        }
    });

    assert!(
        damage_hills < damage_flat,
        "hills defender should take less damage ({damage_hills}) than flat ({damage_flat})"
    );
}

/// A unit in forest takes less damage than on flat ground.
#[test]
fn forest_defender_takes_less_damage() {
    let mut s_flat = common::build_scenario();
    let damage_flat = defender_damage_at(&mut s_flat, HexCoord::from_qr(8, 5), |_, _| {});

    let mut s_forest = common::build_scenario();
    let damage_forest = defender_damage_at(&mut s_forest, HexCoord::from_qr(8, 5), |s, coord| {
        if let Some(t) = s.state.board.tile_mut(coord) {
            t.feature = Some(BuiltinFeature::Forest);
        }
    });

    assert!(
        damage_forest < damage_flat,
        "forest defender should take less damage ({damage_forest}) than flat ({damage_flat})"
    );
}

/// A unit in marsh takes MORE damage than on flat ground.
#[test]
fn marsh_defender_takes_more_damage() {
    let mut s_flat = common::build_scenario();
    let damage_flat = defender_damage_at(&mut s_flat, HexCoord::from_qr(8, 5), |_, _| {});

    let mut s_marsh = common::build_scenario();
    let damage_marsh = defender_damage_at(&mut s_marsh, HexCoord::from_qr(8, 5), |s, coord| {
        if let Some(t) = s.state.board.tile_mut(coord) {
            t.feature = Some(BuiltinFeature::Marsh);
        }
    });

    assert!(
        damage_marsh > damage_flat,
        "marsh defender should take more damage ({damage_marsh}) than flat ({damage_flat})"
    );
}
