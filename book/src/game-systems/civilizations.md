# Civilizations & Leaders

Each player controls a `Civilization` with a unique identity, leader, abilities, and state tracking for research, government, economy, and military.

## Civilization State

```rust
struct Civilization {
    // Identity
    id: CivId,
    name: &'static str,
    adjective: &'static str,
    civ_identity: Box<dyn CivIdentity>,
    leader_identity: Box<dyn LeaderIdentity>,
    leader: Leader,

    // Cities
    cities: Vec<CityId>,

    // Era
    current_era: EraId,
    era_score: u32,
    era_age: EraAge,
    historic_moments: Vec<HistoricMoment>,
    earned_moments: HashSet<&'static str>,

    // Research
    researched_techs: Vec<TechId>,
    research_queue: VecDeque<TechProgress>,
    completed_civics: Vec<CivicId>,
    civic_in_progress: Option<CivicProgress>,

    // Government
    current_government: Option<GovernmentId>,
    active_policies: Vec<PolicyId>,
    unlocked_governments: Vec<GovernmentId>,
    unlocked_policies: Vec<PolicyId>,

    // Economy
    gold: i32,
    strategic_resources: HashMap<BuiltinResource, i32>,

    // Unlocks
    revealed_resources: Vec<ResourceId>,
    unlocked_units: Vec<UnitTypeId>,
    unlocked_buildings: Vec<BuildingId>,
    unlocked_improvements: Vec<ImprovementId>,
    eureka_triggered: Vec<TechId>,
    inspiration_triggered: Vec<CivicId>,

    // Modifiers
    great_person_modifiers: Vec<Modifier>,

    // Tourism
    lifetime_culture: u32,
    tourism_accumulated: u32,
    tourism_output: u32,
    domestic_culture: u32,

    // Visibility
    visible_tiles: HashSet<HexCoord>,
    explored_tiles: HashSet<HexCoord>,
}
```

## Leaders

```rust
struct Leader {
    name: &'static str,
    civ_id: CivId,
    abilities: Vec<Box<dyn LeaderAbility>>,
    agenda: Box<dyn Agenda>,
}
```

Leaders provide abilities (permanent modifiers) and an agenda (AI personality). Because `Leader` contains `Box<dyn Trait>` fields, it does not derive `Clone`.

## Civilization Abilities

The `CivAbility` system provides per-civilization rule overrides:

| Civilization | Leader | Unique Ability |
|-------------|--------|----------------|
| Rome | Trajan | Free Monument + Trading Post on city founding |
| Babylon | Hammurabi | Eureka gives full tech; -50% science |
| Greece | Pericles | +1 wildcard policy slot |
| Germany | Barbarossa | +1 district slot per city |
| Egypt | Cleopatra | Exclusive: Sphinx improvement |
| India | Gandhi | Exclusive: Stepwell improvement |

### Unique Units

| Unit | Civilization | Replaces | Special |
|------|-------------|----------|---------|
| Hoplite | Greece | Spearman | +10 CS when adjacent to another Hoplite |
| Samurai | Japan | Swordsman | No combat penalty when damaged |
| Varu | India | -- | -5 CS to adjacent enemy units |
| Mamluk | Egypt | Knight | Heals 10 HP every turn |

### Unique Districts

| District | Civilization | Replaces | Special |
|----------|-------------|----------|---------|
| Acropolis | Greece | Theater Square | Must be on hills |

## Traits

### StartBias
```rust
trait StartBias {
    fn terrain_preference(&self) -> Option<BuiltinTerrain>;
    fn feature_preference(&self) -> Option<BuiltinFeature>;
    fn resource_preference(&self) -> Option<BuiltinResource>;
}
```

### LeaderAbility
```rust
trait LeaderAbility {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn modifiers(&self) -> Vec<Modifier>;
}
```

### Agenda
```rust
trait Agenda {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn attitude(&self, toward: CivId) -> DiplomaticStatus;
}
```
