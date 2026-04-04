//! Helper functions extracted from `rules.rs` to reduce file size.
//!
//! Contains: tile/border helpers, loyalty pressure, diplomacy status computation,
//! citizen assignment, and tile yield gating.

use std::collections::HashSet;
use crate::{AgreementId, CityId, CivId, YieldBundle};
use crate::civ::DiplomaticRelation;
use crate::civ::DiplomaticStatus;
use crate::rules::modifier::EffectType;
use crate::world::tile::WorldTile;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::{HexCoord, HexDir};

use super::board::WorldBoard;
use super::diff::{GameStateDiff, StateDelta};
use super::state::GameState;

// ── Tile and border helpers ──────────────────────────────────────────────────

/// Claim a single tile for a city, emitting `TileClaimed` if the tile was unclaimed.
/// Skips enemy-owned tiles silently (happens at map edge during founding near a rival).
pub(crate) fn try_claim_tile(
    state: &mut GameState,
    civ_id: CivId,
    city_id: CityId,
    coord: HexCoord,
    diff: &mut GameStateDiff,
) {
    let newly_claimed = if let Some(t) = state.board.tile_mut(coord) {
        match t.owner {
            Some(owner) if owner == civ_id => false, // already ours, no delta
            Some(_) => false,                         // enemy tile, skip
            None => {
                t.owner = Some(civ_id);
                true
            }
        }
    } else {
        false
    };

    if newly_claimed {
        if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
            city.territory.insert(coord);
        }
        diff.push(StateDelta::TileClaimed { civ: civ_id, city: city_id, coord });
    }
}

/// Cost in shadow-culture to claim a tile at `distance` hexes from the city center.
/// Ring 1 tiles are free and claimed automatically at founding; this is only
/// called for distances 2–5.
pub(crate) fn tile_border_cost(distance: u32) -> u32 {
    (10.0 + (6.0 * distance as f64).powf(1.3)) as u32
}

/// Per-city culture output for one turn: 1 base culture plus culture from
/// worked tiles. The base matches the per-city culture added in `compute_yields`.
/// Used exclusively for the border expansion shadow accumulator; does not
/// affect the civilization's culture pool.
pub(crate) fn city_culture_output(board: &WorldBoard, city: &crate::civ::City) -> u32 {
    let tile_culture: u32 = city.worked_tiles.iter()
        .filter_map(|&c| board.tile(c))
        .map(|t| t.total_yields().culture.max(0) as u32)
        .sum();
    1 + tile_culture
}

/// Return the `HexDir` from `from` to an adjacent `to`, handling board wrapping.
pub(crate) fn neighbor_dir(board: &WorldBoard, from: HexCoord, to: HexCoord) -> Option<HexDir> {
    HexDir::ALL.iter().find(|&&dir| {
        board.normalize(from + dir.unit_vec()) == Some(to)
    }).copied()
}

/// Apply a resolved set of `EffectType`s to a `YieldBundle`.
/// Flat bonuses are applied first, then percentage bonuses.
pub(crate) fn apply_effects(effects: &[EffectType], mut base: YieldBundle) -> YieldBundle {
    for &effect in effects {
        if let EffectType::YieldFlat(yt, amount) = effect {
            base.add_yield(yt, amount);
        }
    }
    for &effect in effects {
        if let EffectType::YieldPercent(yt, pct) = effect {
            let current = base.get(yt);
            let bonus   = (current * pct) / 100;
            base.add_yield(yt, bonus);
        }
    }
    base
}

/// Compute tile yields, suppressing the resource component when the civ lacks
/// the required reveal tech (PHASE3-4.2). Improvement yields are also skipped
/// when pillaged, consistent with `WorldTile::total_yields`.
pub(crate) fn tile_yields_gated(tile: &WorldTile, known_techs: &HashSet<&str>) -> YieldBundle {
    let mut yields = tile.terrain.base_yields();

    if let Some(feat) = tile.feature {
        yields += feat.yield_modifier();
    }

    if let Some(impr) = tile.improvement
        && !tile.improvement_pillaged
    {
        yields += impr.yield_bonus();
    }

    if let Some(res) = tile.resource {
        let reveal_tech = res.reveal_tech();
        // Include resource yields only when:
        //   1. No reveal tech is required, or the civ has already researched it.
        //   2. The resource is not concealed by an overlying feature
        //      (Forest/Rainforest hide resources until the feature is cleared).
        let tech_ok = reveal_tech.is_none_or(|t| known_techs.contains(t));
        let concealed = tile.feature
            .is_some_and(|f| f.conceals_resources());
        if tech_ok && !concealed {
            yields += res.base_yields();
        }
    }

    yields
}

/// Assign the highest-yield unworked tile within 3 rings of the city to the
/// city's worked set. Called automatically when a city's population grows.
/// Locked tiles are never displaced; unlocked tiles may be reassigned later.
pub(crate) fn auto_assign_citizen(board: &WorldBoard, city: &mut crate::civ::City) -> Option<HexCoord> {
    let best = (1u32..=3)
        .flat_map(|r| city.coord.ring(r))
        .filter(|coord| {
            board.tile(*coord).is_some() && !city.worked_tiles.contains(coord)
        })
        .max_by_key(|coord| {
            board.tile(*coord)
                .map(|t| {
                    let y = t.total_yields();
                    y.food + y.production + y.gold + y.science + y.culture
                })
                .unwrap_or(0)
        });

    if let Some(coord) = best {
        city.worked_tiles.push(coord);
        Some(coord)
    } else {
        None
    }
}

// ── Loyalty pressure helpers ─────────────────────────────────────────────────

/// Maximum distance (hex tiles) at which a foreign city exerts loyalty pressure.
pub(crate) const LOYALTY_PRESSURE_RADIUS: u32 = 9;

/// Base loyalty pressure exerted by a single city at distance `d`.
/// Falls off as `10 / d` so closer cities exert dramatically more pressure.
/// Returns 0 when `d == 0` (the city itself) or `d > LOYALTY_PRESSURE_RADIUS`.
pub(crate) fn city_loyalty_pressure_at_distance(population: u32, distance: u32) -> i32 {
    if distance == 0 || distance > LOYALTY_PRESSURE_RADIUS {
        return 0;
    }
    // Base pressure scales with population: each pop adds +1 base pressure.
    let base = population as i32;
    // Fall-off: 10 / d, so adjacent city (d=1) gives full 10×base,
    // d=2 gives 5×base, d=9 gives ~1×base.
    (base * 10) / distance as i32
}

/// Compute the net loyalty delta for a single city this turn.
///
/// Positive delta means loyalty trends toward max (owner has strong presence);
/// negative means foreign pressure is eroding loyalty.
///
/// Sources of loyalty pressure:
/// - **Domestic cities**: Each city owned by the same civ within
///   `LOYALTY_PRESSURE_RADIUS` adds positive pressure proportional to population
///   and inversely proportional to distance.
/// - **Foreign cities**: Each city owned by a different civ within
///   `LOYALTY_PRESSURE_RADIUS` adds negative pressure (same formula).
/// - **Capital bonus**: The city's own civ's capital within range adds +5 flat.
/// - **Occupied penalty**: Occupied cities suffer -5 per turn base penalty.
/// - **Governor bonus**: An established governor in this city adds +8 loyalty/turn.
/// - **Population bonus**: City's own population adds +1 per 2 pop (loyalty
///   from its own citizens).
pub(crate) fn compute_city_loyalty_delta(
    city_idx: usize,
    cities: &[crate::civ::City],
    governors: &[crate::civ::Governor],
) -> i32 {
    let city = &cities[city_idx];
    let owner = city.owner;
    let coord = city.coord;

    // Skip city-states — they don't participate in the loyalty system.
    if matches!(city.kind, crate::civ::city::CityKind::CityState(_)) {
        return 0;
    }

    let mut domestic_pressure: i32 = 0;
    let mut foreign_pressure: i32 = 0;
    let mut has_capital_nearby = false;

    for (i, other) in cities.iter().enumerate() {
        if i == city_idx {
            continue;
        }
        // Skip city-states as pressure sources.
        if matches!(other.kind, crate::civ::city::CityKind::CityState(_)) {
            continue;
        }
        let dist = coord.distance(&other.coord);
        if dist > LOYALTY_PRESSURE_RADIUS {
            continue;
        }
        let pressure = city_loyalty_pressure_at_distance(other.population, dist);
        if other.owner == owner {
            domestic_pressure += pressure;
            if other.is_capital {
                has_capital_nearby = true;
            }
        } else {
            foreign_pressure += pressure;
        }
    }

    let mut delta: i32 = 0;

    // Net city pressure: domestic pushes loyalty up, foreign pushes it down.
    delta += domestic_pressure - foreign_pressure;

    // Capital proximity bonus.
    if has_capital_nearby {
        delta += 5;
    }

    // Occupied city penalty: loyalty erodes faster.
    if city.ownership == crate::civ::city::CityOwnership::Occupied {
        delta -= 5;
    }

    // Governor bonus: an established governor stabilizes loyalty.
    let has_governor = governors.iter().any(|g| {
        g.owner == owner
            && g.assigned_city == Some(city.id)
            && g.is_established()
    });
    if has_governor {
        delta += 8;
    }

    // Self-population bonus: the city's own citizens contribute loyalty.
    delta += city.population as i32 / 2;

    // Is-capital bonus: capitals are naturally more loyal.
    if city.is_capital {
        delta += 10;
    }

    // Clamp the delta so loyalty changes are gradual (max ±20 per turn).
    delta.clamp(-20, 20)
}

/// Find the civilization exerting the highest foreign loyalty pressure on a city.
/// Returns `None` if no foreign civ exerts any pressure (city becomes Free City).
pub(crate) fn highest_pressure_civ(
    city_idx: usize,
    cities: &[crate::civ::City],
) -> Option<CivId> {
    let city = &cities[city_idx];
    let owner = city.owner;
    let coord = city.coord;

    let mut pressure_by_civ: Vec<(CivId, i32)> = Vec::new();

    for (i, other) in cities.iter().enumerate() {
        if i == city_idx {
            continue;
        }
        if other.owner == owner {
            continue;
        }
        if matches!(other.kind, crate::civ::city::CityKind::CityState(_)) {
            continue;
        }
        let dist = coord.distance(&other.coord);
        let pressure = city_loyalty_pressure_at_distance(other.population, dist);
        if pressure > 0 {
            if let Some(entry) = pressure_by_civ.iter_mut().find(|(c, _)| *c == other.owner) {
                entry.1 += pressure;
            } else {
                pressure_by_civ.push((other.owner, pressure));
            }
        }
    }

    pressure_by_civ.sort_by(|a, b| b.1.cmp(&a.1));
    pressure_by_civ.first().map(|(civ, _)| *civ)
}

// ── Diplomacy helpers ────────────────────────────────────────────────────────

/// Find the index of the `DiplomaticRelation` between two civs in `state`,
/// creating a new `Neutral` relation and appending it if none exists.
pub(crate) fn find_or_create_relation(state: &mut GameState, a: CivId, b: CivId) -> usize {
    if let Some(idx) = state.diplomatic_relations.iter().position(|r| {
        (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a)
    }) {
        return idx;
    }
    state.diplomatic_relations.push(DiplomaticRelation::new(a, b));
    state.diplomatic_relations.len() - 1
}

/// Compute the opinion score used for status thresholds: the arithmetic mean
/// of each side's net opinion (both directions averaged).
pub(crate) fn combined_score(rel: &DiplomaticRelation) -> i32 {
    (rel.opinion_score_a_toward_b() + rel.opinion_score_b_toward_a()) / 2
}

/// Map a combined opinion score to a `DiplomaticStatus`.
/// Does **not** apply the War-persistence rule; use `compute_diplomatic_status`
/// for the full transition logic including War persistence.
pub(crate) fn status_from_score(score: i32, active_agreements: &[AgreementId]) -> DiplomaticStatus {
    if score > 50 {
        if active_agreements.is_empty() {
            DiplomaticStatus::Friendly
        } else {
            DiplomaticStatus::Alliance
        }
    } else if score < -20 {
        DiplomaticStatus::Denounced
    } else {
        DiplomaticStatus::Neutral
    }
}

/// Determine the new status for a relation, honouring the War-persistence
/// rule: once at war, the relation stays at War while the combined score
/// remains below -50. All other transitions are driven purely by score.
pub(crate) fn compute_diplomatic_status(rel: &DiplomaticRelation) -> DiplomaticStatus {
    // War persists until explicitly ended via make_peace(). It does NOT
    // auto-resolve from opinion score improvements.
    if rel.status == DiplomaticStatus::War {
        return DiplomaticStatus::War;
    }
    // Alliance persists until explicitly broken; it does NOT auto-resolve
    // from opinion score changes.
    if rel.status == DiplomaticStatus::Alliance {
        return DiplomaticStatus::Alliance;
    }
    let score = combined_score(rel);
    status_from_score(score, &rel.active_agreements)
}
