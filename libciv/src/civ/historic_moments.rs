//! Historic moment definitions and the observer that scans `StateDelta` events
//! to detect when a civilization earns era score.

use crate::CivId;
use crate::game::diff::StateDelta;
use crate::game::state::GameState;
use super::era::{HistoricMomentDef, HistoricMomentKind};

// ---------------------------------------------------------------------------
// Const definitions of all historic moments
// ---------------------------------------------------------------------------

pub const HISTORIC_MOMENTS: &[HistoricMomentDef] = &[
    HistoricMomentDef {
        name: "City Founded",
        description: "Founded a new city.",
        era_score: 1,
        kind: HistoricMomentKind::CityFounded,
        unique: false,
    },
    HistoricMomentDef {
        name: "Technology Researched",
        description: "Completed research on a new technology.",
        era_score: 1,
        kind: HistoricMomentKind::TechResearched,
        unique: false,
    },
    HistoricMomentDef {
        name: "Civic Completed",
        description: "Completed a new civic.",
        era_score: 1,
        kind: HistoricMomentKind::CivicCompleted,
        unique: false,
    },
    HistoricMomentDef {
        name: "Wonder Built",
        description: "Completed construction of a world wonder.",
        era_score: 4,
        kind: HistoricMomentKind::WonderBuilt,
        unique: false,
    },
    HistoricMomentDef {
        name: "Enemy Defeated in Battle",
        description: "Destroyed an enemy unit in combat.",
        era_score: 1,
        kind: HistoricMomentKind::BattleWon,
        unique: false,
    },
    HistoricMomentDef {
        name: "District Constructed",
        description: "Built a new specialty district.",
        era_score: 2,
        kind: HistoricMomentKind::DistrictBuilt,
        unique: false,
    },
    HistoricMomentDef {
        name: "Building Completed",
        description: "Completed a new building in a city.",
        era_score: 1,
        kind: HistoricMomentKind::BuildingCompleted,
        unique: false,
    },
    HistoricMomentDef {
        name: "Trade Route Established",
        description: "Established a new trade route.",
        era_score: 1,
        kind: HistoricMomentKind::TradeRouteEstablished,
        unique: false,
    },
    HistoricMomentDef {
        name: "First City Founded",
        description: "Founded the civilization's very first city.",
        era_score: 2,
        kind: HistoricMomentKind::CityFounded,
        unique: true,
    },
];

// ---------------------------------------------------------------------------
// Observer: scan deltas and emit matching historic moments
// ---------------------------------------------------------------------------

/// Match a `StateDelta` to a `HistoricMomentKind`, returning the owning `CivId`.
fn delta_to_kind(delta: &StateDelta) -> Option<(CivId, HistoricMomentKind)> {
    match delta {
        StateDelta::CityFounded { owner, .. } => Some((*owner, HistoricMomentKind::CityFounded)),
        StateDelta::TechResearched { civ, .. } => Some((*civ, HistoricMomentKind::TechResearched)),
        StateDelta::CivicCompleted { civ, .. } => Some((*civ, HistoricMomentKind::CivicCompleted)),
        StateDelta::WonderBuilt { civ, .. } => Some((*civ, HistoricMomentKind::WonderBuilt)),
        StateDelta::UnitDestroyed { unit } => {
            // UnitDestroyed doesn't carry an attacker CivId; BattleWon moments
            // are handled separately via UnitAttacked + UnitDestroyed pairing.
            // For now we skip -- the caller handles battle detection.
            let _ = unit;
            None
        }
        StateDelta::DistrictBuilt { city, .. } => {
            // DistrictBuilt carries city but not civ directly; caller resolves.
            let _ = city;
            None
        }
        StateDelta::BuildingCompleted { city, .. } => {
            let _ = city;
            None
        }
        StateDelta::TradeRouteEstablished { owner, .. } => {
            Some((*owner, HistoricMomentKind::TradeRouteEstablished))
        }
        _ => None,
    }
}

/// Resolve the owning CivId for deltas that carry a CityId instead of a CivId.
fn resolve_city_owner(delta: &StateDelta, state: &GameState) -> Option<(CivId, HistoricMomentKind)> {
    match delta {
        StateDelta::DistrictBuilt { city, .. } => {
            state.city(*city).map(|c| (c.owner, HistoricMomentKind::DistrictBuilt))
        }
        StateDelta::BuildingCompleted { city, .. } => {
            state.city(*city).map(|c| (c.owner, HistoricMomentKind::BuildingCompleted))
        }
        _ => None,
    }
}

/// Detect battle victories by scanning for UnitAttacked followed by UnitDestroyed.
/// Returns the attacking civ for each destroyed defender.
fn detect_battle_victories(deltas: &[StateDelta], state: &GameState) -> Vec<(CivId, HistoricMomentKind)> {
    let mut results = Vec::new();
    for delta in deltas {
        if let StateDelta::UnitAttacked { attacker, defender, defender_damage, .. } = delta {
            // Check if defender was destroyed (appears as UnitDestroyed in same diff)
            let defender_destroyed = deltas.iter().any(|d| {
                matches!(d, StateDelta::UnitDestroyed { unit } if *unit == *defender)
            });
            if defender_destroyed {
                if let Some(attacker_unit) = state.units.iter().find(|u| u.id == *attacker) {
                    results.push((attacker_unit.owner, HistoricMomentKind::BattleWon));
                }
            }
            let _ = defender_damage;
        }
    }
    results
}

/// Scan a batch of `StateDelta`s and return historic moments earned by each civ.
///
/// The returned tuples are `(CivId, &HistoricMomentDef)` -- the caller is responsible
/// for recording them on the civilization and emitting the appropriate `StateDelta`.
pub fn observe_deltas<'a>(
    deltas: &[StateDelta],
    state: &GameState,
) -> Vec<(CivId, &'a HistoricMomentDef)> {
    let mut results: Vec<(CivId, &HistoricMomentDef)> = Vec::new();

    // Collect all (CivId, Kind) pairs from the deltas.
    let mut triggers: Vec<(CivId, HistoricMomentKind)> = Vec::new();
    for delta in deltas {
        if let Some(pair) = delta_to_kind(delta) {
            triggers.push(pair);
        }
        if let Some(pair) = resolve_city_owner(delta, state) {
            triggers.push(pair);
        }
    }
    triggers.extend(detect_battle_victories(deltas, state));

    // Match triggers against moment definitions.
    for (civ_id, kind) in &triggers {
        for moment_def in HISTORIC_MOMENTS {
            if moment_def.kind == *kind {
                // Uniqueness check: if unique, see if the civ already earned it this era.
                if moment_def.unique {
                    let already_earned = state
                        .civ(*civ_id)
                        .map(|c| c.earned_moments.contains(moment_def.name))
                        .unwrap_or(false);
                    if already_earned {
                        continue;
                    }
                    // Also check if we already emitted it in this batch.
                    let in_batch = results.iter().any(|(c, m)| *c == *civ_id && m.name == moment_def.name);
                    if in_batch {
                        continue;
                    }
                }
                results.push((*civ_id, moment_def));
                // For unique moments, only match the first definition.
                // For non-unique, we still only want one match per trigger event.
                break;
            }
        }
    }

    results
}
