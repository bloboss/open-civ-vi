# City-State Mechanics

Phase 5, item 15. Quests, envoys, suzerainty bonuses.

## Current State

- `CityStateType` enum: Cultural, Industrial, Militaristic, Religious, Scientific, Trade (`civ/city_state.rs`).
- `CityStateBonus` trait with `yields_for_suzerain()`.
- `CityStateData` struct: `state_type`, `suzerain: Option<CivId>`, `influence: HashMap<CivId, i32>`.
- `recalculate_suzerain()` picks the civ with the highest influence.
- City-states stored as `City` with `kind = CityKind::CityState(CityStateData)`.
- `GameState::city_state_by_civ(CivId)` accessor exists.
- `PerCivStateSuzerain` condition exists in modifier system but returns `Scale(0)` (stub).
- No envoy system, no quest system, no suzerainty bonus application.

## Design

### Envoys

Envoys are the currency for city-state influence. Civs earn envoys and send them to city-states.

- Each envoy sent to a city-state grants +1 influence.
- Envoys are earned from: civic completions, certain great people, and periodic per-era grants.
- The civ with the most envoys at a city-state is its suzerain (minimum 3 envoys required).

### Suzerainty Bonuses

The suzerain receives:
- The city-state's unique bonus (based on `CityStateType`).
- The city-state's resources.
- The ability to levy the city-state's military.

### Envoy Tier Bonuses

| Envoys | Bonus |
|--------|-------|
| 1 | Small yield bonus based on type |
| 3 | Eligible for suzerainty; medium bonus |
| 6 | Large yield bonus |

### Quests (simplified)

City-states issue quests that, when completed, grant bonus envoys:
- "Send a trade route" (+2 envoys)
- "Build a specific district" (+2 envoys)
- "Recruit a Great Person" (+2 envoys)

## Implementation Plan

### Step 1: Add envoy tracking

On `Civilization`:

```rust
pub envoys_available: u32,                           // unspent
pub envoys_placed: HashMap<CivId, u32>,              // city-state civ_id -> count
```

Replace `CityStateData::influence` with envoy-based calculation.

### Step 2: Add `RulesEngine::send_envoy()`

```rust
fn send_envoy(&self, state: &mut GameState, civ_id: CivId, city_state_civ: CivId)
    -> Result<GameStateDiff, RulesError>;
```

Validation: civ has available envoys, target is a city-state. Deduct envoy, increment placement count, recalculate suzerainty. Emit `StateDelta::EnvoySent`.

### Step 3: Add `StateDelta` variants

```rust
EnvoySent { civ: CivId, city_state: CivId, total: u32 },
SuzerainChanged { city_state: CivId, old: Option<CivId>, new: Option<CivId> },
EnvoyEarned { civ: CivId, amount: u32, source: &'static str },
```

### Step 4: Envoy earning in `advance_turn`

- On civic completion: grant 1 envoy (via `OneShotEffect::GrantEnvoy`).
- Periodic: every N turns, grant 1 envoy to each civ (or on era advancement).

### Step 5: Suzerainty bonus application

Wire `PerCivStateSuzerain` condition in `rules/modifier.rs` to actually count suzerainties. Add type-based yield bonuses:

| CityStateType | Suzerain Bonus |
|--------------|----------------|
| Scientific | +2 Science per Campus |
| Cultural | +2 Culture per Theater Square |
| Industrial | +2 Production per Industrial Zone |
| Trade | +4 Gold per Commercial Hub |
| Militaristic | +2 Production toward units |
| Religious | +2 Faith per Holy Site |

### Step 6: Simplified quests

```rust
pub struct CityStateQuest {
    pub description: &'static str,
    pub kind: QuestKind,
    pub reward_envoys: u32,
}

pub enum QuestKind {
    SendTradeRoute,
    BuildDistrict(BuiltinDistrict),
    RecruitGreatPerson,
}
```

Add `active_quest: Option<CityStateQuest>` to `CityStateData`. Check quest completion conditions in `advance_turn` or when relevant deltas are observed.

### Step 7: Tests

1. Send envoy, verify influence increases and suzerainty recalculates.
2. Suzerain bonus modifiers apply to civ's yields.
3. Insufficient envoys returns error.
4. Quest completion grants bonus envoys.
5. Suzerainty changes when another civ surpasses envoy count.

## Complexity

Medium-high. New mechanics (envoys, quests), modifier wiring, turn phase additions. ~400 lines of new code.

## Dependencies

- Diplomatic victory (Phase 3, item 8) benefits from city-state suzerainty as a favor source.
- `PerCivStateSuzerain` modifier condition needs this to function.
