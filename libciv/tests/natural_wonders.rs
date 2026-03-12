/// Tests for natural wonder yield bonuses and appeal.
use libciv::game::WorldBoard;
use libciv::world::terrain::BuiltinTerrain;
use libciv::world::wonder::{
    BuiltinNaturalWonder, GrandMesa, Krakatoa, NaturalWonder, UluruAyersRock,
};
use libciv::NaturalWonderId;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::MovementCost;
use libhexgrid::HexTile;

fn wonder_id() -> NaturalWonderId {
    NaturalWonderId::from_ulid(ulid::Ulid::new())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A natural wonder tile's yield = terrain base + wonder yield_bonus.
/// Feature and improvement bonuses are suppressed.
#[test]
fn wonder_tile_yields_terrain_plus_wonder_bonus() {
    let mut board = WorldBoard::new(5, 5);
    let coord = HexCoord::from_qr(2, 2);

    // Place Grand Mesa on a Grassland tile.
    // Grand Mesa: +2 Production, +1 Food.
    // Grassland base: +2 Food.
    // Expected total: 3 Food, 2 Production.
    if let Some(t) = board.tile_mut(coord) {
        t.terrain = BuiltinTerrain::Grassland;
        t.natural_wonder = Some(BuiltinNaturalWonder::GrandMesa(GrandMesa { id: wonder_id() }));
    }

    let tile = board.tile(coord).unwrap();
    let yields = tile.total_yields();

    assert_eq!(yields.food,       3, "Grassland 2 + GrandMesa 1 = 3 food");
    assert_eq!(yields.production, 2, "GrandMesa grants +2 production");
}

/// An impassable wonder (Krakatoa) has Impassable movement cost.
#[test]
fn impassable_wonder_has_impassable_movement_cost() {
    let mut board = WorldBoard::new(5, 5);
    let coord = HexCoord::from_qr(2, 2);

    if let Some(t) = board.tile_mut(coord) {
        t.natural_wonder = Some(BuiltinNaturalWonder::Krakatoa(Krakatoa { id: wonder_id() }));
    }

    let tile = board.tile(coord).unwrap();
    assert_eq!(
        tile.movement_cost(),
        MovementCost::Impassable,
        "Krakatoa should be impassable"
    );
}

/// Appeal bonus reported by a wonder definition.
#[test]
fn wonder_appeal_bonus_is_correct() {
    let wonder = UluruAyersRock { id: wonder_id() };
    assert_eq!(wonder.appeal_bonus(), 4);
    assert_eq!(wonder.yield_bonus().faith, 3, "Uluru grants +3 faith");
    assert!(wonder.impassable(), "Uluru is impassable");
}
