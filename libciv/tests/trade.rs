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
        siege_bonus: 0,
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
