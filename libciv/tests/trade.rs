/// Integration tests for trade routes and the Trader unit.
mod common;

use libciv::{DefaultRulesEngine, RulesEngine};
use libciv::game::{RulesError, StateDelta};
use libciv::{CivId, UnitId, UnitCategory, UnitDomain, UnitTypeId};
use libciv::game::state::UnitTypeDef;
use libciv::civ::BasicUnit;
use libhexgrid::coord::HexCoord;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Add a trader unit at `coord` owned by `owner` and return its UnitId.
fn spawn_trader(s: &mut common::Scenario, owner: CivId, coord: HexCoord) -> UnitId {
    let trader_type = UnitTypeId::from_ulid(s.state.id_gen.next_ulid());
    s.state.unit_type_defs.push(UnitTypeDef {
        id: trader_type,
        name: "trader",
        production_cost: 40,
        domain: UnitDomain::Land,
        category: UnitCategory::Trader,
        max_movement: 200,
        combat_strength: None,
        range: 0,
        vision_range: 2,
        can_found_city: false,
        resource_cost: None,
        siege_bonus: 0, max_charges: 0, exclusive_to: None, replaces: None,
    });
    let unit_id = s.state.id_gen.next_unit_id();
    s.state.units.push(BasicUnit {
        id: unit_id,
        unit_type: trader_type,
        owner,
        coord,
        domain: UnitDomain::Land,
        category: UnitCategory::Trader,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None,
    });
    unit_id
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// After establish_trade_route the trader unit is removed and the diff
/// contains UnitDestroyed + TradeRouteEstablished.
#[test]
fn establish_route_consumes_trader_unit() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Snapshot IDs before any mutable borrow.
    let rome_id = s.rome_id;
    let babylon_city = s.babylon_city;
    let rome_coord = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);

    let diff = rules.establish_trade_route(&mut s.state, trader, babylon_city).unwrap();

    // Trader no longer in state.
    assert!(!s.state.units.iter().any(|u| u.id == trader));

    // Trade route created.
    assert_eq!(s.state.trade_routes.len(), 1);

    // Diff contains UnitDestroyed and TradeRouteEstablished.
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitDestroyed { unit } if *unit == trader)));
    assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::TradeRouteEstablished { .. })));
}

/// Domestic route (same-civ cities) yields: origin 3 gold, destination 1 gold.
#[test]
fn establish_route_domestic_yields() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Add a second city for Rome at a different coord so we can have a domestic route.
    let second_city_id = s.state.id_gen.next_city_id();
    let second_coord = HexCoord::from_qr(5, 6);
    let mut city2 = libciv::civ::City::new(second_city_id, "Ostia".into(), s.rome_id, second_coord);
    city2.is_capital = false;
    s.state.cities.push(city2);
    let rome_id = s.rome_id;
    s.state.civilizations.iter_mut()
        .find(|c| c.id == rome_id).unwrap()
        .cities.push(second_city_id);

    let rome_coord = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);

    rules.establish_trade_route(&mut s.state, trader, second_city_id).unwrap();

    let route = &s.state.trade_routes[0];
    assert_eq!(route.origin_yields.gold, 3);
    assert_eq!(route.destination_yields.gold, 1);
}

/// International route (different-civ cities) yields: origin 6 gold, destination 4 gold.
#[test]
fn establish_route_international_yields() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_id = s.rome_id;
    let babylon_city = s.babylon_city;
    let rome_coord = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);

    rules.establish_trade_route(&mut s.state, trader, babylon_city).unwrap();

    let route = &s.state.trade_routes[0];
    assert_eq!(route.origin_yields.gold, 6);
    assert_eq!(route.destination_yields.gold, 4);
}

/// compute_yields includes trade route gold for the origin civ.
#[test]
fn compute_yields_includes_trade_gold() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_id = s.rome_id;
    let babylon_city = s.babylon_city;

    let before_gold = rules.compute_yields(&s.state, rome_id).gold;

    let rome_coord = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);
    rules.establish_trade_route(&mut s.state, trader, babylon_city).unwrap();

    let after_gold = rules.compute_yields(&s.state, rome_id).gold;
    // International: origin gets +6 gold.
    assert_eq!(after_gold - before_gold, 6);
}

/// A route with turns_remaining = 30 is removed after 31 advance_turn calls,
/// and a TradeRouteExpired delta is emitted.
#[test]
fn route_expires_after_30_turns() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_id = s.rome_id;
    let babylon_city = s.babylon_city;
    let rome_coord = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);
    rules.establish_trade_route(&mut s.state, trader, babylon_city).unwrap();

    assert_eq!(s.state.trade_routes.len(), 1);

    // Run 30 turns — route still alive (turns_remaining decrements 29→0 on last turn).
    for _ in 0..30 {
        rules.advance_turn(&mut s.state);
    }
    assert_eq!(s.state.trade_routes.len(), 1, "route should survive 30 turns");

    // On turn 31 the route (turns_remaining == 0) is expired at Phase 2b.
    let diff_31 = rules.advance_turn(&mut s.state);
    assert!(s.state.trade_routes.is_empty(), "route should be removed after 31 turns");
    assert!(diff_31.deltas.iter().any(|d| matches!(d, StateDelta::TradeRouteExpired { .. })));
}

/// establish_trade_route with a non-trader unit returns NotATrader.
#[test]
fn establish_route_fails_if_not_trader() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let babylon_city = s.babylon_city;
    // rome_warrior is a combat unit.
    let result = rules.establish_trade_route(&mut s.state, s.rome_warrior, babylon_city);
    assert!(matches!(result, Err(RulesError::NotATrader)));
}

// ── Autonomous trader movement tests ─────────────────────────────────────

/// assign_trade_route sets the trader's origin and destination fields and
/// emits a TradeRouteAssigned delta.
#[test]
fn assign_trade_route_sets_destination() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_id = s.rome_id;
    let babylon_city = s.babylon_city;
    let rome_coord = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);

    let diff = rules.assign_trade_route(&mut s.state, trader, babylon_city).unwrap();

    // Check unit fields were set.
    let unit = s.state.units.iter().find(|u| u.id == trader).unwrap();
    assert_eq!(unit.trade_origin, Some(s.rome_city));
    assert_eq!(unit.trade_destination, Some(babylon_city));

    // Check diff contains TradeRouteAssigned.
    assert!(diff.deltas.iter().any(|d| matches!(d,
        StateDelta::TradeRouteAssigned { unit: u, destination: d, .. }
        if *u == trader && *d == babylon_city
    )));
}

/// assign_trade_route fails with NotATrader for non-trader units.
#[test]
fn assign_trade_route_fails_if_not_trader() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let result = rules.assign_trade_route(&mut s.state, s.rome_warrior, s.babylon_city);
    assert!(matches!(result, Err(RulesError::NotATrader)));
}

/// assign_trade_route fails when the trader is not at a city owned by its civ.
#[test]
fn assign_trade_route_fails_if_not_at_origin_city() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    // Spawn trader at an arbitrary location, not on a city.
    let rome_id = s.rome_id;
    let babylon_city = s.babylon_city;
    let trader = spawn_trader(&mut s, rome_id, HexCoord::from_qr(5, 5));
    let result = rules.assign_trade_route(&mut s.state, trader, babylon_city);
    assert!(matches!(result, Err(RulesError::NoOriginCity)));
}

/// A trader with an assigned destination moves toward the destination city
/// each turn during advance_turn.
#[test]
fn trader_moves_autonomously_toward_destination() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_id = s.rome_id;
    let rome_coord = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);

    // Remove warriors to avoid occupancy conflicts during trader movement.
    s.state.units.retain(|u| u.category == UnitCategory::Trader);

    rules.assign_trade_route(&mut s.state, trader, s.babylon_city).unwrap();

    let start_coord = s.state.units.iter().find(|u| u.id == trader).unwrap().coord;

    // After one advance_turn, the trader should have moved from its starting position.
    rules.advance_turn(&mut s.state);

    // The trader may still exist (hasn't arrived yet) — check it moved.
    if let Some(unit) = s.state.units.iter().find(|u| u.id == trader) {
        assert_ne!(unit.coord, start_coord, "trader should have moved from start position");
    }
    // Or it may have already arrived and been consumed (on very small maps).
}

/// After enough turns, the trader arrives and the trade route is established
/// automatically.
#[test]
fn trader_auto_establishes_route_on_arrival() {
    let mut s = common::build_scenario();
    let rules = DefaultRulesEngine;

    let rome_id = s.rome_id;
    let rome_coord = s.state.cities.iter().find(|c| c.id == s.rome_city).unwrap().coord;
    let trader = spawn_trader(&mut s, rome_id, rome_coord);

    // Remove warriors to avoid occupancy conflicts.
    s.state.units.retain(|u| u.category == UnitCategory::Trader);

    rules.assign_trade_route(&mut s.state, trader, s.babylon_city).unwrap();

    assert!(s.state.trade_routes.is_empty(), "no trade route yet");

    // Run enough turns for the trader to reach Babylon at (10, 5) from Roma at (3, 3).
    // Distance is ~8 hexes; movement = 200 (2 hexes/turn), so ~4 turns should suffice.
    for _ in 0..20 {
        rules.advance_turn(&mut s.state);
        if !s.state.trade_routes.is_empty() {
            break;
        }
    }

    // Trade route should have been established.
    assert_eq!(s.state.trade_routes.len(), 1, "trade route should be established on arrival");

    // Trader should have been consumed.
    assert!(!s.state.units.iter().any(|u| u.id == trader),
        "trader should be consumed after establishing route");

    // Route should have correct origin and destination.
    let route = &s.state.trade_routes[0];
    assert_eq!(route.origin, s.rome_city);
    assert_eq!(route.destination, s.babylon_city);
    assert_eq!(route.owner, rome_id);
}
