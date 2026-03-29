# Great People Recruitment

Phase 2, item 3 on the roadmap. Points accumulation, candidate pool, and the `recruit_great_person()` action.

## Current State

Great person recruitment is **already implemented**. The status.md description is outdated.

### What exists

- **Points accumulation** in `advance_turn` Phase 5a (`game/rules.rs:1841`): each district generates `GP_BASE_POINTS_PER_DISTRICT` (1) per turn for its associated `GreatPersonType`. Points stored in `Civilization::great_person_points: HashMap<GreatPersonType, u32>`.
- **Candidate pool with era gating**: `next_candidate_name()` in `civ/great_people.rs` filters `great_person_defs` by type, era, and availability. `era_is_current_or_earlier()` gates candidates.
- **Auto-recruitment threshold**: `recruitment_threshold()` returns `GP_BASE_THRESHOLD (60) + GP_THRESHOLD_INCREMENT (60) * previously_recruited_count`. When accumulated points reach the threshold, Phase 5a auto-recruits and spawns the great person at the civ's capital.
- **`recruit_great_person()` on `RulesEngine`** (`game/rules.rs:3309`): manual patronage action. Deducts gold at `GP_PATRONAGE_GOLD_PER_POINT` (3 gold per remaining point), spawns the unit via `spawn_great_person()`, emits `GreatPersonRecruited`.
- **`retire_great_person()`**: applies `RetireEffect` (combat bonus, production burst, gold grant), emits `GreatPersonRetired`. 6 passing tests.
- **Built-in great people**: 18 definitions across all 9 types (Generals, Admirals, Engineers, Merchants, Scientists, Writers, Artists, Musicians, Prophets) spanning Ancient and Classical eras.
- **District mapping**: `district_great_person_types()` maps Campus→Scientist, TheaterSquare→Writer/Artist/Musician, CommercialHub→Merchant, Harbor→Admiral, HolySite→Prophet, Encampment→General, IndustrialZone→Engineer.

### Remaining work

The core system is complete. Optional enhancements:

1. **More great person definitions** -- only Ancient and Classical eras have candidates. Medieval+ eras need definitions to match the later-era tech/civic tree expansion (Phase 5 dependency).
2. **GP point modifiers** -- policies and buildings that grant bonus GP points per turn are not yet wired into Phase 5a. The `EffectType::YieldFlat(GreatPersonPoints, _)` path exists in the modifier system but isn't queried during accumulation.
3. **Patronage with faith** -- currently gold-only. Civ VI allows faith patronage for certain types (Prophets always, others via policies).

### Implementation notes for enhancements

**GP point modifiers** (medium complexity):
- In Phase 5a of `advance_turn`, after computing base district points, collect `Modifier`s with `EffectType::YieldFlat(YieldType::GreatPersonPoints, _)` from the civ's active policies, buildings, and wonders.
- Apply resolved modifier bonus to the relevant `GreatPersonType` bucket.
- Test: assign a policy granting +2 GP points, verify accumulation rate doubles for the relevant type.

**Later-era great people** (content work):
- Add `GreatPersonDef` entries for Medieval, Renaissance, Industrial, Modern, Atomic, Information, and Future eras.
- Each era should have at least one candidate per `GreatPersonType`.
- Blocked on later-era tech/civic trees (Phase 5, item 12) for era gating to be meaningful.
