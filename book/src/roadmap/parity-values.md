# P0 — Fix Value Discrepancies

Every item below exists in the implementation but has a yield, cost, prerequisite,
or reveal-tech value that differs from the Civ VI base-game XML. These are pure
data fixes — no new enum variants, no new files.

**Source XML**: `original-xml/Base/Assets/Gameplay/Data/`

---

## Resource Yields

File: `libciv/src/world/resource.rs`

### Luxury resources with wrong yields

| Resource | Current Yield | Correct Yield (XML) | Fix |
|---|---|---|---|
| Silk | +3 Gold | +1 Culture | Change yield type and value |
| Spices | +1 Food, +1 Gold | +2 Food | Remove Gold, increase Food to 2 |
| Incense | +1 Gold, +1 Faith | +1 Faith | Remove Gold yield |
| Sugar | +2 Food, +1 Gold | +2 Food | Remove Gold yield |
| Cotton | +3 Gold | +3 Gold | ✓ Correct |

### Strategic resources with wrong yields

| Resource | Current Yield | Correct Yield (XML) | Fix |
|---|---|---|---|
| Horses | +1 Production | +1 Food, +1 Production | Add Food yield |
| Iron | +1 Production | +1 Science | Change from Production to Science |

### Strategic resources with wrong reveal techs

| Resource | Current Reveal Tech | Correct Reveal Tech (XML) | Fix |
|---|---|---|---|
| Oil | Refining | Steel (`TECH_STEEL`) | Change to Steel |
| Aluminum | Refining | Radio (`TECH_RADIO`) | Change to Radio |
| Uranium | Nuclear Fission | Combined Arms (`TECH_COMBINED_ARMS`) | Change to Combined Arms |

> **Note**: "Refining" does not exist as a technology in Civ VI base game.
> The implementation uses placeholder tech names. Oil and Aluminum reveal techs
> will need to reference real tech IDs once P1 adds later-era techs.

---

## Technology Tree

File: `libciv/src/rules/tech_tree_def.rs`

### Misplaced technology

| Issue | Detail |
|---|---|
| **Theology** is in the tech tree | In Civ VI, Theology is a **Civic** (Classical era, cost 120, prereqs: Drama & Poetry + Mysticism). Move it from `tech_tree_def.rs` to `civic_tree_def.rs`. |

### Wrong prerequisites

| Tech | Current Prereq | Correct Prereq (XML) | Fix |
|---|---|---|---|
| Writing | Pottery | **None** (no prerequisites) | Remove Pottery prereq |

### Missing eureka details (cosmetic)

These are not gameplay-critical but are documented for completeness:

| Tech | Current Eureka | Correct Eureka (XML) |
|---|---|---|
| Pottery | "Harvest any resource" | N/A (no eureka in base game — it's always available) |
| Irrigation | "Find a Floodplain or River" | "Farm a resource" |
| The Wheel | "Build a Quarry" | "Mine a resource" |

---

## Civic Tree

File: `libciv/src/rules/civic_tree_def.rs`

### Wrong prerequisites

| Civic | Current Prereqs | Correct Prereqs (XML) | Fix |
|---|---|---|---|
| Early Empire | Craftsmanship + Foreign Trade | **Foreign Trade** only | Remove Craftsmanship prereq |
| Mysticism | Code of Laws | **Foreign Trade** | Change prereq from Code of Laws to Foreign Trade |

### Missing Ancient-era civics

These should be added as part of P0 since they are Ancient era and unlock
content already referenced in the implementation:

| Civic | Cost | Prereqs | Effects (XML) |
|---|---|---|---|
| Military Tradition | 50 | Craftsmanship | Unlocks policies |
| State Workforce | 70 | Craftsmanship | Unlocks policies, Games & Recreation |

---

## District Prerequisites

File: `libciv/src/civ/district.rs`

| District | Current Prereq | Correct Prereq (XML) | Fix |
|---|---|---|---|
| CommercialHub | Unreachable | Currency (`TECH_CURRENCY`) | Set to Currency |
| IndustrialZone | Unreachable | Apprenticeship (`TECH_APPRENTICESHIP`) | Set to Apprenticeship |
| Harbor | Sailing | Celestial Navigation (`TECH_CELESTIAL_NAVIGATION`) | Requires Classical tech (P1) |
| Aqueduct | Masonry | Engineering (`TECH_ENGINEERING`) | Requires Classical tech (P1) |
| EntertainmentComplex | Early Empire | Games & Recreation (`CIVIC_GAMES_RECREATION`) | Requires Classical civic (P1) |

> **Note**: CommercialHub and IndustrialZone can be fixed now since Currency and
> Apprenticeship are placeholder names for techs not yet in the tree. The
> remaining three require Classical-era techs/civics from P1.

---

## Improvement Values

File: `libciv/src/world/improvement.rs`

| Improvement | Issue | Current | Correct (XML) | Fix |
|---|---|---|---|---|
| LumberMill | Wrong base yield | +2 Production | +1 Production | Reduce to 1 |
| LumberMill | Wrong prereq tech | Unreachable | Machinery (`TECH_MACHINERY`) | Requires Medieval tech (P1) |
| Stepwell | Wrong housing | +1 Housing | +2 Housing | Increase to 2 |

---

## Natural Wonder Yields

File: `libciv/src/world/wonder.rs`

| Wonder | Current Yields | Correct Yields (XML) | Fix |
|---|---|---|---|
| Cliffs of Dover | +2 Culture, +1 Gold | +3 Culture, +3 Gold, +2 Food | Increase all three |
| Galapagos | +2 Science (on tile) | Adjacent tiles: +2 Science (tile itself has no yield) | Change to adjacent-yield model |
| Krakatoa | +4 Production | **Not in base game** — DLC content | Mark as non-base or remove |
| Grand Mesa | +2 Prod, +1 Food | **Not in base game** — DLC content | Mark as non-base or remove |
| Uluru | +3 Faith | **Not in base game** — DLC content | Mark as non-base or remove |

> **Note**: 3 of the 5 implemented natural wonders (Krakatoa, Grand Mesa, Uluru)
> are not in the Civ VI base game XML. They appear in DLC. The implementation
> should either tag them as DLC content or replace them with base-game wonders
> from the [missing list](parity-content.md#natural-wonders).

---

## Feature Variants

File: `libciv/src/world/feature.rs`

| Feature | Issue |
|---|---|
| Reef | Not in base game `Features.xml` (Gathering Storm) — tag as GS content |
| VolcanicSoil | Not in base game `Features.xml` (Gathering Storm) — tag as GS content |

---

## Civilization: Babylon

File: `libciv/src/rules/civ_registry.rs`

Babylon (Hammurabi) is **not a base-game civilization** — it was added in the
New Frontier Pass. The implementation should tag it as DLC content. The 19
base-game civilizations are listed in [parity-systems.md](parity-systems.md#civilizations).

---

## Unit Values

File: `libciv/src/civ/unit.rs` / `civsim/src/main.rs`

| Unit | Field | Current | Correct (XML) | Fix |
|---|---|---|---|---|
| Slinger | Combat (melee) | 10 | 5 | Reduce to 5 |
| Slinger | RangedCombat | — (not set) | 15 | Add ranged combat = 15 |
| Archer | Note | Not registered as base unit | Should be base Classical unit | Add to base unit registry |

---

## Summary of P0 changes

| File | Changes |
|---|---|
| `resource.rs` | Fix 4 luxury yields, 2 strategic yields, 3 reveal techs |
| `tech_tree_def.rs` | Remove Theology node, fix Writing prereq |
| `civic_tree_def.rs` | Add Theology (Classical), fix Early Empire + Mysticism prereqs, add Military Tradition + State Workforce |
| `district.rs` | Fix CommercialHub + IndustrialZone prereqs (2 immediate; 3 deferred to P1) |
| `improvement.rs` | Fix LumberMill yield (2→1), Stepwell housing (1→2) |
| `wonder.rs` | Fix Cliffs of Dover yields; tag Krakatoa/Grand Mesa/Uluru as DLC |
| `feature.rs` | Tag Reef + VolcanicSoil as GS content |
| `unit.rs` / `main.rs` | Fix Slinger combat values |
