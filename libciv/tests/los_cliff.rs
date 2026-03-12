/// Tests for elevation-based cliff LOS detection.
///
/// Cliffs are not explicitly placed — they emerge when adjacent tiles have an
/// elevation level difference > 1 (e.g., Ocean Level(0) directly bordering
/// flat inland Level(2)).

use libciv::game::WorldBoard;
use libciv::world::terrain::BuiltinTerrain;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Ocean (Level 0) directly adjacent to flat inland (Level 2) — elevation
/// diff = 2 > 1 — constitutes a cliff and must block LOS.
#[test]
fn ocean_adjacent_to_inland_blocks_los() {
    let mut board = WorldBoard::new(10, 10);

    let ocean_coord  = HexCoord::from_qr(4, 5);
    let inland_coord = HexCoord::from_qr(5, 5);  // adjacent

    // Set the ocean tile (default is Grassland; switch to Ocean).
    if let Some(t) = board.tile_mut(ocean_coord) {
        t.terrain = BuiltinTerrain::Ocean;
    }
    // inland_coord is already Grassland (Level 2) — no change needed.

    // Adjacent diff = |0 - 2| = 2 > 1 → cliff → LOS must be blocked.
    assert!(
        !board.has_los(ocean_coord, inland_coord),
        "Ocean (Level 0) directly adjacent to flat inland (Level 2) should block LOS"
    );
}

/// Ocean → Coast → Inland has a gradual elevation gradient (each step diff = 1).
/// No cliff → LOS must NOT be blocked.
#[test]
fn gradual_ocean_coast_inland_transition_does_not_block_los() {
    // Layout (same row): ocean(3,5) — coast(4,5) — inland(5,5)
    // Elevation:           0            1              2
    // Each consecutive diff = 1 → no cliff.
    let mut board = WorldBoard::new(10, 10);

    if let Some(t) = board.tile_mut(HexCoord::from_qr(3, 5)) {
        t.terrain = BuiltinTerrain::Ocean;
    }
    if let Some(t) = board.tile_mut(HexCoord::from_qr(4, 5)) {
        t.terrain = BuiltinTerrain::Coast;
    }
    // (5,5) is already Grassland (Level 2).

    // from=ocean, to=inland, Coast in between — no cliff on any step.
    assert!(
        board.has_los(HexCoord::from_qr(3, 5), HexCoord::from_qr(5, 5)),
        "Gradual Ocean→Coast→Inland transition should not block LOS"
    );
}
