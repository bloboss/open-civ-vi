# Religion

> **Status: Stub implementation only.** Data structures exist but the core mechanics (founding, spreading, belief effects) are not yet implemented.

## Data Structures

### Religion

```rust
struct Religion {
    id: ReligionId,
    name: String,
    founded_by: CivId,
    holy_city: CityId,
    beliefs: Vec<BeliefId>,
    followers: HashMap<CityId, u32>,
}
```

### Belief Trait

```rust
trait Belief {
    fn id(&self) -> BeliefId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
}
```

### BeliefContext

```rust
struct BeliefContext {
    followers: u32,
    holy_cities: u32,
}
```

## Planned Mechanics

The following systems are defined in the architecture but not yet implemented:

1. **Founding** -- Great Prophets found religions at Holy Sites
2. **Beliefs** -- each religion selects beliefs that provide modifiers (founder, follower, worship building, enhancer)
3. **Spread** -- religious units (Missionary, Apostle) spread religion to cities
4. **Pressure** -- passive religious pressure from nearby cities with followers
5. **Holy Wars** -- theological combat between religious units
6. **Worship Buildings** -- special buildings unlocked by the worship belief

The modifier system already supports `ModifierSource::Religion` and conditions like `CityHasWorshipBuilding` and `PerForeignCityWithWorshipBuilding`, so the infrastructure for belief effects is in place.

See the [Roadmap](../roadmap.md) for implementation priority.
