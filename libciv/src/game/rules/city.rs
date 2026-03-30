//! City handlers: `found_city`, `claim_tile`, `reassign_tile`, `assign_citizen`, `compute_yields`.

use std::collections::HashSet;
use crate::{CityId, CivId, UnitId, YieldBundle};
use libhexgrid::board::HexBoard;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::MovementCost;

use super::{RulesError, lookup_bundle};
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::rules_helpers::{
    apply_effects, tile_yields_gated, try_claim_tile,
};
use super::super::state::GameState;
use crate::rules::modifier::{ConditionContext, resolve_modifiers};

/// Consume a settler unit and found a new city at its current position.
pub(crate) fn found_city(
    state:   &mut GameState,
    settler: UnitId,
    name:    String,
) -> Result<GameStateDiff, RulesError> {
    let (coord, civ_id, unit_type_id) = {
        let u = state.unit(settler).ok_or(RulesError::UnitNotFound)?;
        (u.coord, u.owner, u.unit_type)
    };

    let is_settler = state.unit_type_defs.iter()
        .any(|d| d.id == unit_type_id && d.can_found_city);
    if !is_settler { return Err(RulesError::NotASettler); }

    let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;
    if !tile.terrain.is_land() {
        return Err(RulesError::InvalidFoundingTerrain);
    }
    if tile.terrain.movement_cost() == MovementCost::Impassable {
        return Err(RulesError::InvalidFoundingTerrain);
    }

    if state.cities.iter().any(|c| c.coord == coord) {
        return Err(RulesError::TileOccupied);
    }
    if state.cities.iter().any(|c| c.coord.distance(&coord) <= 3) {
        return Err(RulesError::TooCloseToCity);
    }

    let city_id = state.id_gen.next_city_id();
    let is_capital = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .is_none_or(|c| c.cities.is_empty());
    let mut city = crate::civ::City::new(city_id, name, civ_id, coord);
    city.is_capital = is_capital;

    if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
        civ.cities.push(city_id);
    }
    state.cities.push(city);
    state.units.retain(|u| u.id != settler);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::UnitDestroyed { unit: settler });
    diff.push(StateDelta::CityFounded { city: city_id, coord, owner: civ_id });

    try_claim_tile(state, civ_id, city_id, coord, &mut diff);
    for nb in state.board.neighbors(coord) {
        try_claim_tile(state, civ_id, city_id, nb, &mut diff);
    }

    // ── Civ ability: on_city_founded hooks ──────────────────────────────
    let civ_identity = state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .and_then(|c| c.civ_identity);
    if let Some(bundle) = lookup_bundle(civ_identity) {
        use crate::civ::civ_ability::CityFoundedHook;
        for hook in &bundle.on_city_founded {
            match hook {
                CityFoundedHook::FreeBuilding(building_name) => {
                    if let Some(bdef) = state.building_defs.iter()
                        .find(|d| d.name == *building_name)
                        && let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id)
                        && !city.buildings.contains(&bdef.id)
                    {
                        let bid = bdef.id;
                        city.buildings.push(bid);
                        diff.push(StateDelta::BuildingCompleted {
                            city: city_id, building: building_name,
                        });
                    }
                }
                CityFoundedHook::FreeTradingPost => {
                    if let Some(tile) = state.board.tile_mut(coord)
                        && tile.improvement.is_none()
                    {
                        tile.improvement = Some(
                            crate::world::improvement::BuiltinImprovement::TradingPost,
                        );
                        diff.push(StateDelta::ImprovementPlaced {
                            coord,
                            improvement: crate::world::improvement::BuiltinImprovement::TradingPost,
                        });
                    }
                }
                CityFoundedHook::RoadToCapital => {
                    // Stub: road building requires pathfinding infrastructure.
                    // TODO: build road along shortest path to capital.
                }
            }
        }
    }

    Ok(diff)
}

/// Claim `coord` for the civilization that owns `city_id`.
pub(crate) fn claim_tile(
    state: &mut GameState,
    city_id: CityId,
    coord: HexCoord,
    force: bool,
) -> Result<GameStateDiff, RulesError> {
    let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

    let (city_coord, civ_id) = state.cities.iter()
        .find(|c| c.id == city_id)
        .map(|c| (c.coord, c.owner))
        .ok_or(RulesError::CityNotFound)?;

    let dist = city_coord.distance(&coord);
    if !(1..=3).contains(&dist) {
        return Err(RulesError::TileNotInCityRange);
    }

    let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;

    match tile.owner {
        Some(owner) if owner == civ_id => {
            return Ok(GameStateDiff::new());
        }
        Some(_) if !force => return Err(RulesError::TileOwnedByEnemy),
        Some(_) | None => {}
    }

    if let Some(t) = state.board.tile_mut(coord) {
        t.owner = Some(civ_id);
    }
    if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
        city.territory.insert(coord);
    }
    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TileClaimed { civ: civ_id, city: city_id, coord });
    Ok(diff)
}

/// Reassign `coord` from one city to another within the same civilization.
pub(crate) fn reassign_tile(
    state: &mut GameState,
    from_city: CityId,
    to_city: CityId,
    coord: HexCoord,
) -> Result<GameStateDiff, RulesError> {
    let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

    let from_civ = state.cities.iter()
        .find(|c| c.id == from_city)
        .map(|c| c.owner)
        .ok_or(RulesError::CityNotFound)?;

    let (to_coord, to_civ) = state.cities.iter()
        .find(|c| c.id == to_city)
        .map(|c| (c.coord, c.owner))
        .ok_or(RulesError::CityNotFound)?;

    if from_civ != to_civ {
        return Err(RulesError::CitiesNotSameCiv);
    }
    let civ_id = from_civ;

    if from_city == to_city {
        return Ok(GameStateDiff::new());
    }

    let owner = state.board.tile(coord)
        .ok_or(RulesError::InvalidCoord)?
        .owner;
    if owner != Some(civ_id) {
        return Err(RulesError::TileNotOwned);
    }

    let to_dist = to_coord.distance(&coord);
    if !(1..=3).contains(&to_dist) {
        return Err(RulesError::TileNotInCityRange);
    }

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TileReassigned { civ: civ_id, from_city, to_city, coord });
    Ok(diff)
}

/// Assign a citizen to work `tile` in `city`.
pub(crate) fn assign_citizen(
    state: &mut GameState,
    city_id: CityId,
    tile: HexCoord,
    lock: bool,
) -> Result<GameStateDiff, RulesError> {
    let city_idx = state.cities.iter().position(|c| c.id == city_id)
        .ok_or(RulesError::CityNotFound)?;

    let tile = state.board.normalize(tile).ok_or(RulesError::InvalidCoord)?;

    if state.board.tile(tile).is_none() {
        return Err(RulesError::InvalidCoord);
    }

    if state.cities[city_idx].coord.distance(&tile) > 3 {
        return Err(RulesError::InvalidCoord);
    }

    let mut diff = GameStateDiff::new();
    let city = &mut state.cities[city_idx];

    if !city.worked_tiles.contains(&tile) {
        city.worked_tiles.push(tile);
        diff.push(StateDelta::CitizenAssigned { city: city_id, tile });
    }
    if lock {
        city.locked_tiles.insert(tile);
    }

    Ok(diff)
}

/// Compute all yields for a civilization this turn.
pub(crate) fn compute_yields(state: &GameState, civ_id: CivId) -> YieldBundle {
    let mut total = YieldBundle::default();

    let known_techs: HashSet<&str> = state.civ(civ_id)
        .map(|civ| {
            state.tech_tree.nodes.values()
                .filter(|n| civ.researched_techs.contains(&n.id))
                .map(|n| n.name)
                .collect()
        })
        .unwrap_or_default();

    for city in state.cities.iter().filter(|c| c.owner == civ_id) {
        for &coord in &city.worked_tiles {
            if let Some(tile) = state.board.tile(coord) {
                total += tile_yields_gated(tile, &known_techs);
            }
        }
    }

    let city_count = state.cities.iter().filter(|c| c.owner == civ_id).count();
    total.science += city_count as i32;
    total.culture += city_count as i32;

    // ── Trade route yields ────────────────────────────────────────────────
    for route in &state.trade_routes {
        if route.owner == civ_id {
            total += route.origin_yields.clone();
        }
    }
    for route in &state.trade_routes {
        let dest_owner = state.cities.iter()
            .find(|c| c.id == route.destination)
            .map(|c| c.owner);
        if dest_owner == Some(civ_id) && route.owner != civ_id {
            total += route.destination_yields.clone();
        }
    }

    // Collect modifiers.
    let modifiers = {
        let mut mods = state.civ(civ_id)
            .map(|civ| {
                let mut m = civ.get_modifiers(
                    &state.policies,
                    &state.governments,
                    &state.diplomatic_relations,
                );
                m.extend(civ.get_tree_modifiers(&state.tech_tree, &state.civic_tree));
                if let Some(bundle) = lookup_bundle(civ.civ_identity) {
                    m.extend(bundle.civ_modifiers);
                    m.extend(bundle.leader_modifiers);
                }
                m
            })
            .unwrap_or_default();
        for gov in &state.governors {
            if gov.owner == civ_id && gov.is_established() && gov.assigned_city.is_some() {
                mods.extend(crate::civ::governor::get_governor_modifiers(gov));
            }
        }
        mods
    };

    let ctx = ConditionContext::for_civ(civ_id, state);
    let effects = resolve_modifiers(&modifiers, Some(&ctx));
    apply_effects(&effects, total)
}
