# Victory Conditions

## Framework

Victory conditions are pluggable via the `VictoryCondition` trait:

```rust
trait VictoryCondition {
    fn id(&self) -> VictoryId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn kind(&self) -> VictoryKind;
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress;
}
```

### Victory Kinds

```rust
enum VictoryKind {
    ImmediateWin,              // wins as soon as condition is met
    TurnLimit { turn_limit: u32 }, // evaluated at the turn limit
}
```

`ImmediateWin` victories take priority over `TurnLimit` victories. If multiple immediate wins trigger on the same turn, the first registered condition wins.

### Progress Tracking

```rust
struct VictoryProgress {
    victory_id: VictoryId,
    civ_id: CivId,
    current: u32,
    target: u32,
}
```

`is_won()` returns true when `current >= target`. `percentage()` gives progress as a float.

## Implemented Victories

### Score Victory

**Kind**: `TurnLimit`

At the specified turn limit, the civilization with the highest score wins.

Score calculation (`compute_score()`):

| Component | Points |
|-----------|--------|
| Cities owned | x5 per city |
| Total population | x1 per pop |
| Techs researched | x3 per tech |
| Civics completed | x2 per civic |
| Territory tiles | /5 (one point per 5 tiles) |

`all_scores()` returns all civilizations sorted by score descending.

### Culture Victory

**Kind**: `ImmediateWin`

A civilization wins a cultural victory when its accumulated tourism exceeds the lifetime culture of every other civilization:

```
for every other civ:
    my_tourism_accumulated > their_lifetime_culture
```

This requires building wonders, creating great works, and generating tourism over many turns while other civilizations accumulate less culture.

### Domination Victory

**Kind**: `ImmediateWin`

A civilization wins a domination victory when it controls the original capital of every other civilization. Cities are captured through military conquest -- when all defenders are eliminated and a melee unit attacks the city tile.

Progress tracks: `current = capitals controlled`, `target = total original capitals - 1`.

## Game Over

```rust
struct GameOver {
    winner: CivId,
    condition: String,  // name of the victory condition
    turn: u32,
}
```

Once `game_over` is set on `GameState`, no further victory evaluations occur. The game is complete.

## Not Yet Implemented

- **Science Victory** -- milestone chain (launch Earth Satellite, land on Moon, establish Mars Colony)
- **Diplomatic Victory** -- accumulate diplomatic favor through world congress votes
- **Religious Victory** -- convert a majority of cities in every civilization
