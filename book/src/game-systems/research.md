# Research & Civics

## Tech Tree

The tech tree defines the technology progression:

```rust
struct TechNode {
    id: TechId,
    name: &'static str,
    cost: u32,
    prerequisites: Vec<TechId>,
    effects: Vec<OneShotEffect>,
    eureka_description: Option<&'static str>,
    eureka_effects: Vec<OneShotEffect>,
}
```

### Ancient Era Techs

| Tech | Cost | Prerequisites | Key Unlocks |
|------|------|---------------|-------------|
| Pottery | 25 | -- | Granary, Farm |
| Animal Husbandry | 25 | -- | Pasture, reveals Horses |
| Mining | 25 | -- | Mine |
| Sailing | 50 | -- | Galley |
| Archery | 50 | -- | Archer |
| Astrology | 50 | -- | Holy Site, Shrine |
| Writing | 50 | Pottery | Campus |
| Irrigation | 50 | Pottery | -- |
| Bronze Working | 50 | Mining | Encampment, reveals Iron |
| The Wheel | 50 | Mining | -- |
| Masonry | 50 | Mining | Ancient Walls |

### Research Queue

Each civilization maintains a research queue:

```rust
research_queue: VecDeque<TechProgress>
```

```rust
struct TechProgress {
    tech_id: TechId,
    progress: u32,
    boosted: bool,
}
```

The front of the queue is the active research. Each turn, the civilization's total science output is applied to the active tech. When `progress >= cost`, the tech completes and its `OneShotEffect`s fire.

### Eureka Moments

Each tech can have a eureka condition that provides a research boost (typically 40% of the cost). When triggered:
1. `TechProgress.boosted` is set to `true`
2. The bonus is applied to `progress`
3. The `eureka_triggered` list on the civilization prevents double-triggering

Babylon's unique ability makes eurekas complete the tech entirely.

### One-Shot Effects

Tech completion triggers `OneShotEffect`s:

```rust
enum OneShotEffect {
    RevealResource(BuiltinResource),
    UnlockUnit(UnitTypeId),
    UnlockBuilding(BuildingId),
    UnlockImprovement(BuiltinImprovement),
    TriggerEureka(TechId),
    TriggerInspiration(CivicId),
    FreeUnit(UnitTypeId),
    FreeBuilding(BuildingId),
    UnlockGovernment(GovernmentId),
    AdoptGovernment(GovernmentId),
    UnlockPolicy(PolicyId),
    GrantModifier(Modifier),
}
```

Effects use a two-phase `effect_queue` drain to handle cascading effects (e.g., a tech that triggers an eureka on another tech).

## Civic Tree

Civics mirror the tech tree but are progressed by culture output:

```rust
struct CivicNode {
    id: CivicId,
    name: &'static str,
    cost: u32,
    prerequisites: Vec<CivicId>,
    effects: Vec<OneShotEffect>,
    inspiration_description: Option<&'static str>,
    inspiration_effects: Vec<OneShotEffect>,
}
```

### Ancient Era Civics

| Civic | Cost | Prerequisites | Key Unlocks |
|-------|------|---------------|-------------|
| Code of Laws | 20 | -- | Chiefdom government, Discipline policy |
| Craftsmanship | 40 | Code of Laws | Ilkum policy, Theater Square |
| Foreign Trade | 40 | Code of Laws | Trader unit |
| Early Empire | 70 | Craftsmanship, Foreign Trade | Autocracy government, Urban Planning policy |
| Mysticism | 50 | Code of Laws | Revelation policy |

### Inspiration

Inspirations work like eurekas but for civics -- completing a condition gives a culture boost toward the civic.

## Base Yields

Every city contributes a base of **+1 science/turn** and **+1 culture/turn** regardless of infrastructure. This ensures research and civic progress even for new cities with no districts.
