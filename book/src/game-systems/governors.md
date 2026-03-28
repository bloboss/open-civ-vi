# Governors

> **Status: Partial implementation.** Governor definitions and data structures exist. Assignment and establishment mechanics are not yet fully integrated.

## Structure

```rust
struct Governor {
    id: GovernorId,
    def_name: &'static str,
    owner: CivId,
    assigned_city: Option<CityId>,
    promotions: Vec<&'static str>,
    turns_to_establish: u8,
}
```

Governors are assigned to cities and provide bonuses once established. The `turns_to_establish` counter decrements each turn; the governor's effects activate only after it reaches 0.

## Traits

```rust
trait GovernorDef {
    fn id(&self) -> GovernorId;
    fn name(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn base_ability_description(&self) -> &'static str;
}

trait GovernorPromotion {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn requires(&self) -> Option<&'static str>;
}
```

## Built-in Governors

| Governor | Title | Base Ability |
|----------|-------|-------------|
| Liang | Surveyor | Builder improvements in city cost -1 charge |
| Magnus | Steward | +1 Production from harvested resources |
| Amani | Diplomat | +2 loyalty per turn in assigned city |

(Additional governors defined via macro.)

## Current Effects

- Governors provide a loyalty stabilization bonus in the city they are assigned to
- The loyalty system already checks for governor presence when computing loyalty pressure

## Not Yet Implemented

- `assign_governor()` action in RulesEngine
- Establishment timer countdown in TurnEngine
- Promotion tree unlocking
- Governor-specific modifier application
- Governor reassignment (with re-establishment delay)
