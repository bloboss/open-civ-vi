# Era System & Historic Moments

## Eras

The game progresses through historical eras:

| Era | Age Type |
|-----|----------|
| Ancient | Ancient |
| Classical | Classical |
| Medieval | Medieval |
| Renaissance | Renaissance |
| Industrial | Industrial |
| Modern | Modern |
| Atomic | Atomic |
| Information | Information |
| Future | Future |

Era advancement is triggered when a civilization researches enough techs and civics to cross the era threshold.

## Era Score

Each civilization accumulates era score through historic moments. At era transitions, the score determines the age of the next era:

| Score Range | Era Age | Effect |
|-------------|---------|--------|
| 0 - 11 | Dark Age | Penalties (loyalty pressure) |
| 12 - 23 | Normal Age | Standard gameplay |
| 24+ | Golden Age | Bonuses (extra policy slot, loyalty bonus) |
| 24+ (after Dark) | Heroic Age | Enhanced bonuses |

```rust
fn compute_era_age(score: u32, was_dark: bool) -> EraAge {
    if score >= GOLDEN_AGE_THRESHOLD {
        if was_dark { EraAge::Heroic } else { EraAge::Golden }
    } else if score >= DARK_AGE_THRESHOLD {
        EraAge::Normal
    } else {
        EraAge::Dark
    }
}
```

Era score resets to 0 when a new era begins.

## Historic Moments

Historic moments award era score when significant events occur:

| Moment | Era Score | Unique? |
|--------|-----------|---------|
| City Founded | 1 | No |
| Technology Researched | 1 | No |
| Civic Completed | 1 | No |
| Wonder Built | 4 | No |
| Enemy Defeated in Battle | 1 | No |
| District Constructed | 2 | No |
| Building Completed | 1 | No |
| Trade Route Established | 1 | No |
| First City Founded | 2 | Yes |

### Unique Moments

Moments marked as `unique: true` can only be earned once per era. The `earned_moments` set on `Civilization` tracks which unique moments have been claimed. The set resets on era advancement.

### Observer Pattern

The era system uses an observer that scans `StateDelta` values after each turn phase:
1. Turn phases emit deltas (e.g., `CityFounded`, `TechResearched`, `WonderBuilt`)
2. The observer matches deltas against `HistoricMomentDef` definitions
3. Matching moments emit `HistoricMomentEarned` deltas and increment `era_score`

### Relevant Deltas

- `HistoricMomentEarned { civ, moment_name, era_score }` -- moment awarded
- `EraAdvanced { civ, new_era, era_age }` -- era transition with age determination
