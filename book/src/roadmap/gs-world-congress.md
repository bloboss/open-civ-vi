# GS-3: World Congress

> **Reference**: `original-xml/DLC/Expansion2/Data/Expansion2_Congress.xml`,
> `Expansion2_Victories.xml`

## Overview

Periodic global assembly where civs propose and vote on resolutions using
diplomatic favor. Replaces the simple favor-threshold diplomatic victory with
a Congress-based system.

## Mechanics

### Sessions

World Congress convenes every 30 turns (configurable). Between sessions, civs
accumulate diplomatic favor. When a session starts:

1. **Proposal phase**: Each civ with enough favor can propose one resolution
2. **Voting phase**: Civs spend favor as votes (1 favor = 1 vote)
3. **Resolution**: Majority wins; effects apply globally until next session

### Diplomatic Favor

Sources (per turn):
- Base: +1 per civ
- Per city-state suzerainty: +1
- Alliance: +1 per active alliance
- Government type: varies (+1 to +3)
- Not at war bonus: +1

Spent as votes during Congress sessions.

### Resolution Types (23 from XML)

| Resolution | Category | Effect |
|---|---|---|
| Diplomatic Victory | Special | Award diplomatic victory points |
| Luxury Ban/Boost | Economic | Ban or boost a specific luxury resource |
| Trade Treaty | Economic | +/- trade route yields globally |
| World Religion | Religious | Bonus/penalty to a specific religion |
| Arms Control | Military | +/- unit production cost |
| Heritage Organization | Cultural | +/- tourism/culture bonuses |
| Border Control | Diplomatic | +/- open borders effects |
| Urban Development | Infrastructure | +/- district costs |
| Public Works | Infrastructure | +/- improvement yields |
| Global Energy Treaty | Climate | +/- power plant CO2 emissions |
| Espionage Pact | Espionage | +/- spy effectiveness |
| Military Advisory | Military | +/- combat strength bonuses |

### Special Sessions

- **Nobel Prizes** (Literature, Peace, Physics): Compete for great person points
- **World Games**: Compete for tourism/culture
- **Climate Accords**: Cooperative CO2 reduction
- **Emergencies**: Military, religious, nuclear responses

### Diplomatic Victory (Revised)

Victory points earned through:
- Winning Congress votes: +1 VP per resolution you proposed that passed
- Winning competitions: +2 VP (Nobel, World Games)
- Aid requests: +1 VP per emergency resolved
- Threshold: 20 VP to win

## Data Model

```rust
pub struct WorldCongress {
    pub session_interval: u32,        // turns between sessions (default 30)
    pub next_session_turn: u32,       // when the next session fires
    pub active_resolutions: Vec<Resolution>,
    pub diplomatic_victory_points: HashMap<CivId, u32>,
}

pub struct Resolution {
    pub kind: ResolutionKind,
    pub proposed_by: CivId,
    pub target: Option<ResolutionTarget>,  // specific resource, religion, etc.
    pub votes_for: u32,
    pub votes_against: u32,
    pub passed: bool,
    pub turns_remaining: u32,
}

pub enum ResolutionKind { /* 23 variants from XML */ }
pub enum ResolutionTarget { Resource(BuiltinResource), Religion(ReligionId), /* ... */ }
```

## Implementation Steps

1. Create `libciv/src/civ/congress.rs` — WorldCongress struct, Resolution, voting
2. Add `world_congress: WorldCongress` to `GameState`
3. Add `RulesEngine::propose_resolution()`, `vote_on_resolution()`
4. Add Congress session phase in `advance_turn` (check if session turn reached)
5. Apply resolution effects as global modifiers
6. Replace diplomatic victory with VP-based system
7. Add `StateDelta::CongressSessionStarted`, `ResolutionProposed`,
   `ResolutionVoted`, `ResolutionPassed`, `DiplomaticVPEarned`
8. Add special session types (emergencies, competitions)
9. Add 8+ integration tests

## Files

| File | Changes |
|---|---|
| `libciv/src/civ/congress.rs` | **NEW** — World Congress system |
| `libciv/src/game/state.rs` | Add `world_congress` field |
| `libciv/src/game/rules/mod.rs` | New RulesEngine methods |
| `libciv/src/game/rules/turn_phase.rs` | Congress session phase |
| `libciv/src/game/victory.rs` | Revise Diplomatic victory to VP-based |
| `libciv/src/game/diff.rs` | New delta variants |
| `civsim/src/main.rs` | Congress TUI commands |

## Dependencies

- **Blocked by**: Nothing — can start independently
- **Blocks**: GS-14 (revised diplomatic victory)
