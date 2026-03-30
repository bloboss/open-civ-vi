//! Trade route handlers: `assign_trade_route`, `establish_trade_route`.

use crate::{CityId, UnitId};
use crate::civ::unit::Unit;

use super::RulesError;
use super::super::diff::{GameStateDiff, StateDelta};
use super::super::state::GameState;

/// Assign a trade route destination to a trader unit.
pub(crate) fn assign_trade_route(
    state: &mut GameState,
    trader_unit: UnitId,
    destination: CityId,
) -> Result<GameStateDiff, RulesError> {
    use crate::UnitCategory;

    let (unit_owner, unit_coord, unit_category) = state.units.iter()
        .find(|u| u.id() == trader_unit)
        .map(|u| (u.owner(), u.coord(), u.category()))
        .ok_or(RulesError::UnitNotFound)?;

    if unit_category != UnitCategory::Trader {
        return Err(RulesError::NotATrader);
    }

    let origin_id = state.cities.iter()
        .find(|c| c.owner == unit_owner && c.coord == unit_coord)
        .map(|c| c.id)
        .ok_or(RulesError::NoOriginCity)?;

    if !state.cities.iter().any(|c| c.id == destination) {
        return Err(RulesError::CityNotFound);
    }

    if origin_id == destination {
        return Err(RulesError::SameCity);
    }

    let unit = state.units.iter_mut()
        .find(|u| u.id == trader_unit)
        .ok_or(RulesError::UnitNotFound)?;
    unit.trade_origin = Some(origin_id);
    unit.trade_destination = Some(destination);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::TradeRouteAssigned {
        unit: trader_unit,
        origin: origin_id,
        destination,
    });
    Ok(diff)
}

/// Consume a trader unit and establish a trade route to `destination`.
pub(crate) fn establish_trade_route(
    state: &mut GameState,
    trader_unit: UnitId,
    destination: CityId,
) -> Result<GameStateDiff, RulesError> {
    use crate::UnitCategory;
    use crate::civ::trade::compute_route_yields;

    let (unit_owner, unit_coord, unit_category, stored_origin) = state.units.iter()
        .find(|u| u.id() == trader_unit)
        .map(|u| (u.owner(), u.coord(), u.category(), u.trade_origin))
        .ok_or(RulesError::UnitNotFound)?;

    if unit_category != UnitCategory::Trader {
        return Err(RulesError::NotATrader);
    }

    let origin_id = if let Some(origin) = stored_origin {
        origin
    } else {
        state.cities.iter()
            .find(|c| c.owner == unit_owner && c.coord == unit_coord)
            .map(|c| c.id)
            .ok_or(RulesError::NoOriginCity)?
    };

    if !state.cities.iter().any(|c| c.id == destination) {
        return Err(RulesError::CityNotFound);
    }

    if origin_id == destination {
        return Err(RulesError::SameCity);
    }

    let international = {
        let origin_owner = state.cities.iter().find(|c| c.id == origin_id).map(|c| c.owner);
        let dest_owner   = state.cities.iter().find(|c| c.id == destination).map(|c| c.owner);
        matches!((origin_owner, dest_owner), (Some(a), Some(b)) if a != b)
    };
    let (origin_yields, dest_yields) = compute_route_yields(international);

    let route_id = state.id_gen.next_trade_route_id();
    let mut route = crate::civ::TradeRoute::new(route_id, origin_id, destination, unit_owner);
    route.origin_yields      = origin_yields;
    route.destination_yields = dest_yields;
    route.turns_remaining    = Some(30);

    state.units.retain(|u| u.id() != trader_unit);
    state.trade_routes.push(route);

    let mut diff = GameStateDiff::new();
    diff.push(StateDelta::UnitDestroyed { unit: trader_unit });
    diff.push(StateDelta::TradeRouteEstablished {
        route: route_id,
        origin: origin_id,
        destination,
        owner: unit_owner,
    });
    Ok(diff)
}
