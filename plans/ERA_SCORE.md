# Plan: Era Score System

## Context

The game has a stub `Era` struct and `EraTrigger` trait in `libciv/src/civ/era.rs`, plus an
`AgeType` enum in `enums.rs` and `current_era: AgeType` on `Civilization`. The TODO at
PHASE3-8.8 calls for concrete era advancement. This plan implements the Civ VI **Era Score**
system: historic moments earn era score, and when a global era advances, each civ enters a
Dark, Normal, Golden, or Heroic Age based on their accumulated score.

---

## Design Overview

### Core Concepts

1. **Historic Moments** -- named events triggered by game actions (founding a city, researching
   a tech, building a wonder, winning a battle, etc.). Each has a fixed era score value.
2. **Era Score** -- per-civ accumulator that resets when the global era advances.
3. **Era Age** -- when the global era advances, each civ's accumulated era score is compared
   against thresholds to determine whether they enter a Dark Age, Normal Age, Golden Age, or
   Heroic Age (Golden Age immediately following a Dark Age).
4. **Era Dedications** -- stub only. When entering a new age, civs will eventually choose a
   dedication that grants bonuses. This plan stubs the data structures but does not implement
   the bonus effects.

### Observer Pattern

The era score system acts as an **observer on StateDelta events**. After each `advance_turn`
phase produces deltas, the era score observer scans them and emits `HistoricMomentEarned`
deltas for matching triggers. This avoids coupling the era system into every individual game
action.

### Era Advancement

Era advancement is driven by tech/civic thresholds on the global `Era` definition. When any
civ researches enough techs + civics to cross the threshold, the global era advances for all
civs. Each civ's era age is determined by their accumulated era score vs. configurable
thresholds.

---

## Data Structures

### `HistoricMomentKind` (enum)

```rust
pub enum HistoricMomentKind {
    CityFounded,
    TechResearched,
    CivicCompleted,
    WonderBuilt,
    BattleWon,           // killed an enemy unit
    FirstCityFounded,    // first city ever founded by this civ (capital)
    MetNewCivilization,  // first contact via diplomacy change
    DistrictBuilt,
    BuildingCompleted,
    TradeRouteEstablished,
}
```

### `HistoricMomentDef` (const data)

```rust
pub struct HistoricMomentDef {
    pub name: &'static str,
    pub description: &'static str,
    pub era_score: u32,
    pub kind: HistoricMomentKind,
    pub unique: bool,  // can only be earned once per civ per era
}
```

Defined as `pub const HISTORIC_MOMENTS: &[HistoricMomentDef]` in a new file
`libciv/src/civ/historic_moments.rs`.

### `HistoricMoment` (recorded event)

```rust
pub struct HistoricMoment {
    pub civ: CivId,
    pub moment_name: &'static str,
    pub era_score: u32,
    pub turn: u32,
    pub era: AgeType,
}
```

### `EraAge` (enum)

```rust
pub enum EraAge {
    Dark,
    Normal,
    Golden,
    Heroic,
}
```

### Fields added to `Civilization`

```rust
pub era_score: u32,            // accumulated this era; resets on era change
pub era_age: EraAge,           // current age (Dark/Normal/Golden/Heroic)
pub historic_moments: Vec<HistoricMoment>,
pub earned_moments: HashSet<&'static str>,  // uniqueness guard for unique moments
```

### Fields added to `GameState`

```rust
pub eras: Vec<Era>,              // ordered list of era definitions
pub current_era_index: usize,    // index into eras
```

### `StateDelta` additions

```rust
HistoricMomentEarned { civ: CivId, moment: &'static str, era_score: u32 },
EraAdvanced { civ: CivId, new_era: AgeType, era_age: EraAge },
```

### `EraDedication` (stub)

```rust
pub struct EraDedication {
    pub name: &'static str,
    pub description: &'static str,
    pub age_type: EraAge,  // which age unlocks this
    // TODO: pub modifiers: Vec<Modifier>,
}
```

---

## File Changes

### A. `libciv/src/civ/era.rs` -- expand

- Add `EraAge` enum (Dark, Normal, Golden, Heroic) with derive traits.
- Add `HistoricMomentKind` enum.
- Add `HistoricMomentDef` struct.
- Add `HistoricMoment` struct (recorded per-civ event).
- Add `EraDedication` stub struct.
- Add era score threshold constants: `DARK_AGE_THRESHOLD`, `GOLDEN_AGE_THRESHOLD`.
- Add `compute_era_age(era_score: u32, was_dark_age: bool) -> EraAge` function.
- Add `check_era_advancement(state: &GameState) -> bool` function.
- Keep existing `Era` struct and `EraTrigger` trait.

### B. `libciv/src/civ/historic_moments.rs` -- new file

- `pub const HISTORIC_MOMENTS: &[HistoricMomentDef]` with ~10 initial moment definitions.
- `pub fn observe_deltas(deltas: &[StateDelta], state: &GameState) -> Vec<(CivId, HistoricMomentDef)>`
  -- scans deltas and returns matching moments for each civ.

### C. `libciv/src/civ/civilization.rs`

- Add `era_score: u32`, `era_age: EraAge`, `historic_moments: Vec<HistoricMoment>`,
  `earned_moments: HashSet<&'static str>` fields.
- Initialize in `Civilization::new()`.

### D. `libciv/src/game/state.rs`

- Add `eras: Vec<Era>` and `current_era_index: usize` fields.
- Initialize with a default Ancient era.

### E. `libciv/src/game/diff.rs`

- Add `HistoricMomentEarned` and `EraAdvanced` variants to `StateDelta`.
- Uncomment the existing `EraAdvanced` TODO comment.

### F. `libciv/src/game/rules.rs`

- After existing phases in `advance_turn`, add **Phase 5c: Era score observer**.
  - Scan the diff produced so far for trigger events.
  - For each match, record the historic moment on the civ and emit
    `HistoricMomentEarned` delta.
- Add **Phase 5d: Era advancement check**.
  - If tech/civic thresholds crossed, advance global era.
  - For each civ, compute era age from accumulated score, emit `EraAdvanced` delta.
  - Reset era scores.

### G. `libciv/src/civ/mod.rs`

- Add `pub mod historic_moments;` and re-export key types.

### H. `libciv/src/lib.rs`

- Re-export `EraAge` and relevant era types.

---

## Era Thresholds

Default thresholds (tunable constants):

| Era Score Range     | Age    |
|---------------------|--------|
| 0 -- 11             | Dark   |
| 12 -- 23            | Normal |
| 24+                 | Golden |
| 24+ (after Dark)    | Heroic |

Global era advancement triggers when **any civ** has researched enough techs + completed
enough civics to cross the era's threshold. The `Era` struct already has `tech_count` and
`civic_count` fields for this purpose.

---

## Testing

### Integration tests (`libciv/tests/gameplay.rs` or new `era_score.rs`)

1. `test_historic_moment_earned_on_city_founded` -- found a city, verify
   `HistoricMomentEarned` delta emitted and civ's `era_score` increased.
2. `test_historic_moment_earned_on_tech_researched` -- complete a tech, verify moment.
3. `test_historic_moment_earned_on_wonder_built` -- complete a wonder, verify moment.
4. `test_unique_moment_not_duplicated` -- earn a unique moment twice, verify only one recorded.
5. `test_era_advancement_resets_score` -- advance era, verify `era_score` reset to 0.
6. `test_golden_age_from_high_score` -- accumulate enough score, advance era, verify Golden Age.
7. `test_dark_age_from_low_score` -- advance era with low score, verify Dark Age.
8. `test_heroic_age_after_dark` -- Dark Age followed by high score -> Heroic Age.
9. `test_era_advanced_delta_emitted` -- verify `EraAdvanced` delta in diff.
10. `test_battle_won_earns_era_score` -- kill enemy unit, verify moment.

---

## Out of Scope (Future)

- **Era dedication bonuses** -- `EraDedication` struct is stubbed but modifiers are not
  applied. Future work will add `Modifier` effects keyed to `EraAge`.
- **Era-specific great people** -- deferred until great people system is more complete.
- **UI/CLI display** -- era score display in `civsim` deferred.
- **AI era score optimization** -- AI does not yet consider era score in decisions.
