# Trade Routes

Trade routes connect two cities, generating yields for both the origin and destination.

## Establishing Routes

Trade routes are established by Trader units:

1. **Queue a Trader** in a city's production queue
2. **Move the Trader** to the origin city (or use it from where it was produced)
3. **Assign a destination** via `EstablishTradeRoute { trader, destination }`
4. The Trader moves autonomously toward the destination each turn
5. On arrival, the route is established and the Trader unit is consumed

### Autonomous Movement

Once assigned a destination, Trader units move automatically each turn along the shortest path. When they arrive at the destination city, the trade route is established automatically -- no further player action needed.

## Route Yields

Yields are computed by `compute_route_yields()`:

### Domestic Routes (same civilization)

| Endpoint | Yields |
|----------|--------|
| Origin | +3 Gold |
| Destination | +1 Gold |

### International Routes (different civilizations)

| Endpoint | Yields |
|----------|--------|
| Origin | +6 Gold |
| Destination | +4 Gold |

Trade gold is included in `compute_yields()` for the origin and destination cities.

## Route Duration

Each route lasts **30 turns**. When `turns_remaining` reaches 0:
- The route expires
- A `TradeRouteExpired` delta is emitted
- The Trader unit is already consumed, so no unit is returned

## Validation

`establish_trade_route()` validates:
- The unit is a Trader (category = `Trader`)
- The unit belongs to the acting civilization
- The destination is a valid city
- The Trader is at the origin city's location

## State

```rust
struct TradeRoute {
    id: TradeRouteId,
    origin: CityId,
    destination: CityId,
    owner: CivId,
    origin_yields: YieldBundle,
    destination_yields: YieldBundle,
    turns_remaining: u32,
}
```

Trade routes are stored in `GameState.trade_routes` (referenced from the state). The `is_international()` method checks if origin and destination belong to different civilizations.
