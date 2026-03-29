# Religion System

Phase 4, items 9-11. Religion founding, belief system, and religious spread.

## Status: Complete

The religion system has been fully implemented with 24 integration tests in `tests/religion.rs`.

### Implemented features

| Feature | Location | Tests |
|---------|----------|-------|
| Religion founding | `RulesEngine::found_religion()` | Great Prophet at Holy Site, name validation, belief assignment |
| Pantheon founding | `RulesEngine::found_pantheon()` | 25 faith cost, Follower-category belief |
| Belief system | `civ/religion.rs`, `civ/belief_defs.rs` | 16 built-in beliefs across 4 categories (Founder, Follower, Worship, Enhancer) |
| Belief modifier integration | `advance_turn` + `compute_yields` | Belief modifiers collected and applied via standard modifier pipeline |
| Religious spread | `RulesEngine::spread_religion()` | Missionary/Apostle units, charge consumption, follower conversion |
| Passive pressure | `advance_turn` Phase 3d | Per-city religious pressure from adjacent cities and holy cities |
| Theological combat | `RulesEngine::theological_combat()` | Religious unit combat with strength and health |
| Faith purchase | `RulesEngine::purchase_with_faith()` | Buy units and worship buildings with faith |
| Faith accumulation | `advance_turn` Phase 3e | Per-civ faith yield from Holy Sites and beliefs |
| City majority religion | `City::majority_religion()` | Tracks followers per religion, returns majority |

### Religion deltas

All religion actions emit proper `StateDelta` variants:
- `ReligionFounded`, `PantheonFounded`, `BeliefSelected`
- `ReligionSpread`, `ReligiousPressureApplied`, `CityConvertedReligion`
- `TheologicalCombat`, `FaithChanged`

No further implementation needed. The Religious Victory condition (Phase 3, item 7) is the only remaining religion-adjacent work.
