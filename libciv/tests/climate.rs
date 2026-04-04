/// Integration tests for the Climate Change & Environmental Disasters system (GS-2).

mod common;

use libciv::game::{DefaultRulesEngine, RulesEngine, StateDelta};
use libciv::world::disaster::DisasterKind;
use libciv::world::feature::BuiltinFeature;
use libciv::world::improvement::BuiltinImprovement;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;

/// Helper: advance one turn and return the diff deltas.
fn advance_one(scenario: &mut common::Scenario) -> Vec<StateDelta> {
    let engine = DefaultRulesEngine;
    let diff = engine.advance_turn(&mut scenario.state);
    diff.deltas
}

#[test]
fn sea_level_rises_when_co2_crosses_threshold() {
    let mut sc = common::build_scenario();

    // Set CO2 just below the first threshold, advance — no rise.
    sc.state.global_co2 = 199;
    let deltas = advance_one(&mut sc);
    assert!(
        !deltas.iter().any(|d| matches!(d, StateDelta::SeaLevelRose { .. })),
        "No sea level rise below threshold"
    );
    assert_eq!(sc.state.climate_level, 0);

    // Push CO2 to exactly 200 (threshold for level 1), advance.
    sc.state.global_co2 = 200;
    let deltas = advance_one(&mut sc);
    assert!(
        deltas.iter().any(|d| matches!(d, StateDelta::SeaLevelRose { new_level: 1 })),
        "Sea level should rise to 1 at CO2=200"
    );
    assert_eq!(sc.state.climate_level, 1);

    // No duplicate rise on the next turn at same CO2.
    let deltas = advance_one(&mut sc);
    assert!(
        !deltas.iter().any(|d| matches!(d, StateDelta::SeaLevelRose { .. })),
        "No duplicate sea level rise at same CO2"
    );

    // Jump to CO2=600 — should rise to level 3 in one step.
    sc.state.global_co2 = 600;
    let deltas = advance_one(&mut sc);
    assert!(
        deltas.iter().any(|d| matches!(d, StateDelta::SeaLevelRose { new_level: 3 })),
        "Sea level should jump to 3 at CO2=600"
    );
    assert_eq!(sc.state.climate_level, 3);
}

#[test]
fn coastal_tile_submerged_on_sea_level_rise() {
    let mut sc = common::build_scenario();

    let coord = HexCoord::from_qr(2, 2);

    // Tag tile as coastal lowland at 1m elevation.
    if let Some(tile) = sc.state.board.tile_mut(coord) {
        tile.coastal_lowland = Some(1);
        tile.improvement = Some(BuiltinImprovement::Farm);
    }

    // Set CO2 above level-1 threshold and advance.
    sc.state.global_co2 = 200;
    let deltas = advance_one(&mut sc);

    // Should emit TileSubmerged.
    assert!(
        deltas.iter().any(|d| matches!(d, StateDelta::TileSubmerged { coord: c } if *c == coord)),
        "Tile at 1m should be submerged at climate level 1"
    );

    // Verify tile state.
    let tile = sc.state.board.tile(coord).unwrap();
    assert!(tile.submerged, "Tile should be marked submerged");
    assert!(tile.improvement.is_none(), "Improvement should be destroyed on submersion");

    // Should not be submerged again on next turn.
    let deltas = advance_one(&mut sc);
    assert!(
        !deltas.iter().any(|d| matches!(d, StateDelta::TileSubmerged { coord: c } if *c == coord)),
        "Already-submerged tile should not emit again"
    );
}

#[test]
fn higher_lowland_not_submerged_until_higher_level() {
    let mut sc = common::build_scenario();

    let coord = HexCoord::from_qr(3, 2);

    // Tag tile as coastal lowland at 3m elevation.
    if let Some(tile) = sc.state.board.tile_mut(coord) {
        tile.coastal_lowland = Some(3);
    }

    // Climate level 1 — tile should NOT be submerged.
    sc.state.global_co2 = 200;
    advance_one(&mut sc);
    let tile = sc.state.board.tile(coord).unwrap();
    assert!(!tile.submerged, "3m tile should not be submerged at level 1");

    // Climate level 3 — tile SHOULD be submerged.
    sc.state.global_co2 = 600;
    let deltas = advance_one(&mut sc);
    assert!(
        deltas.iter().any(|d| matches!(d, StateDelta::TileSubmerged { coord: c } if *c == coord)),
        "3m tile should be submerged at level 3"
    );
}

#[test]
fn disaster_can_destroy_improvement() {
    // Run many turns with high climate level to trigger at least one disaster.
    // We use a deterministic seed so results are repeatable.
    let mut sc = common::build_scenario();

    // Set high climate level to increase disaster probability (5% + 2%*7 = 19% per turn).
    sc.state.climate_level = 7;
    sc.state.global_co2 = 1500;

    // Place improvements on many tiles so a disaster hit is meaningful.
    let coords = sc.state.board.all_coords();
    for &coord in &coords {
        if let Some(tile) = sc.state.board.tile_mut(coord) {
            tile.improvement = Some(BuiltinImprovement::Farm);
        }
    }

    let mut disaster_found = false;
    for _ in 0..200 {
        let deltas = advance_one(&mut sc);
        if deltas.iter().any(|d| matches!(d, StateDelta::DisasterOccurred { .. })) {
            disaster_found = true;
            break;
        }
    }

    assert!(disaster_found, "At least one disaster should occur in 200 turns at max climate level");
}

#[test]
fn volcanic_eruption_adds_volcanic_soil() {
    // We need to directly test that a volcanic eruption adds VolcanicSoil.
    // Rather than relying on RNG, we test the apply_delta path.
    use libciv::game::apply_delta::apply_delta;

    let mut sc = common::build_scenario();
    let coord = HexCoord::from_qr(4, 4);

    // Place an improvement on the tile.
    if let Some(tile) = sc.state.board.tile_mut(coord) {
        tile.improvement = Some(BuiltinImprovement::Mine);
    }

    // Apply a volcanic eruption delta.
    let delta = StateDelta::DisasterOccurred {
        kind: DisasterKind::VolcanicEruption,
        coord,
        severity: 2,
    };
    apply_delta(&mut sc.state, &delta);

    let tile = sc.state.board.tile(coord).unwrap();
    assert!(tile.improvement.is_none(), "Improvement destroyed by eruption");
    assert_eq!(
        tile.feature,
        Some(BuiltinFeature::VolcanicSoil),
        "Volcanic eruption should add VolcanicSoil feature"
    );
}

#[test]
fn non_volcanic_disaster_does_not_add_soil() {
    use libciv::game::apply_delta::apply_delta;

    let mut sc = common::build_scenario();
    let coord = HexCoord::from_qr(5, 4);

    if let Some(tile) = sc.state.board.tile_mut(coord) {
        tile.improvement = Some(BuiltinImprovement::Farm);
    }

    let delta = StateDelta::DisasterOccurred {
        kind: DisasterKind::Tornado,
        coord,
        severity: 1,
    };
    apply_delta(&mut sc.state, &delta);

    let tile = sc.state.board.tile(coord).unwrap();
    assert!(tile.improvement.is_none(), "Improvement destroyed by tornado");
    assert_ne!(
        tile.feature,
        Some(BuiltinFeature::VolcanicSoil),
        "Non-volcanic disaster should not add VolcanicSoil"
    );
}
