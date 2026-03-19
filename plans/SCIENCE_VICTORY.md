# Science Victory Implementation Plan

## Context

The codebase currently has no Science Victory implementation. The victory system infrastructure exists (`VictoryCondition` trait, `VictoryProgress`, victory evaluation in `advance_turn` Phase 5c) with `CultureVictory` and `ScoreVictory` as examples. The tech tree only covers the Ancient Era (12 techs). To implement Science Victory, we need: (1) a full tech tree from Ancient through Information era, (2) a Spaceport district, (3) a space project production system, and (4) the `ScienceVictory` condition that checks completed projects.

A civilization wins a Science Victory by completing four sequential space projects:
1. **Launch Satellite** — requires Satellites tech + Spaceport district
2. **Moon Colony** — requires Robotics tech + completed Satellite
3. **Mars Colony** — requires Nuclear Fusion tech + completed Moon Colony
4. **Interstellar Colony Ship** — requires Nanotechnology tech + completed Mars Colony

---

## Step 1: Expand the Tech Tree (~70 new techs)

**Files:** `libciv/src/ids.rs`, `libciv/src/rules/tech_tree_def.rs`

Add all techs from Classical through Information era per the provided dependency graph. The full list (with costs and prerequisites from the user's spec):

**Classical (8):** Celestial Navigation (120), Currency (120), Horseback Riding (120), Iron Working (120), Shipbuilding (200), Mathematics (200), Construction (200), Engineering (200)

**Medieval (7):** Military Tactics (300), Apprenticeship (300), Stirrups (390), Machinery (300), Education (390), Military Engineering (390), Castles (390)

**Renaissance (9):** Cartography (540), Mass Production (540), Banking (540), Gunpowder (540), Printing (540), Square Rigging (660), Astronomy (660), Metal Casting (660), Siege Tactics (660)

**Industrial (8):** Industrialization (805), Scientific Theory (805), Ballistics (805), Military Science (805), Steam Power (925), Sanitation (925), Economics (925), Rifling (925)

**Modern (7):** Flight (1035), Replaceable Parts (1035), Steel (1035), Electricity (1135), Radio (1135), Chemistry (1135), Combustion (1135)

**Atomic (8):** Advanced Flight (1225), Rocketry (1225), Advanced Ballistics (1225), Combined Arms (1225), Plastics (1225), Computers (1375), Nuclear Fission (1375), Synthetic Materials (1375)

**Information (10):** Telecommunications (1540), Satellites (1540), Guidance Systems (1540), Lasers (1540), Composites (1540), Stealth Technology (1540), Robotics (1795), Nuclear Fusion (1795), Nanotechnology (1795), Future Tech (2050)

### Changes to `ids.rs`:
Add ~57 new fields to `TechRefs` (one per tech). Keep the flat struct — Rust catches missing fields at compile time.

### Changes to `tech_tree_def.rs`:
Follow the existing pattern exactly: generate IDs in fixed order (appended after existing IDs), define `TechNode` structs with prerequisites/effects, call `tree.add_node()`, return in `TechRefs` literal.

All ~57 techs get correct costs and prerequisites per the user's dependency graph. Effects strategy: **critical path + stubs** — only science-victory-relevant techs and Campus building techs get unlock effects; all others get empty `effects: vec![]` (to be filled in later). Specifically:
- **Writing**: already exists with `UnlockBuilding("Library")` effect
- **Education**: `effects: vec![UnlockBuilding("University")]`
- **Chemistry**: `effects: vec![UnlockBuilding("Research Lab")]`
- **Rocketry**: no special OneShotEffect needed (Spaceport district gates on `tech_refs.rocketry` directly via `DistrictRequirements`)
- **Satellites, Robotics, Nuclear Fusion, Nanotechnology**: no effects needed (project eligibility checked at production time)
- **All other techs**: `effects: vec![], eureka_effects: vec![]`

### Dependencies:
Follow the exact dependency graph from the user's dot specification. Every prerequisite arrow becomes a `vec![prereq_id]` entry.

---

## Step 2: Add Spaceport District

**File:** `libciv/src/civ/district.rs`

Add `Spaceport` variant to `BuiltinDistrict` enum:

```rust
Spaceport,  // in enum
```

Add match arms for `name()` ("Spaceport"), `base_cost()` (135 — late-game, higher cost), and `requirements()`:

```rust
BuiltinDistrict::Spaceport => DistrictRequirements {
    requires_land:      true,
    requires_water:     false,
    forbidden_terrains: &[BuiltinTerrain::Mountain],
    required_tech:      Some(tech_refs.rocketry),
    required_civic:     None,
},
```

The existing `place_district()` in `game/rules.rs` already validates `DistrictRequirements` including tech prereqs, so Spaceport placement is automatically gated by Rocketry tech. No changes needed in `rules.rs` for district placement.

---

## Step 3: Implement Campus District Buildings (Library, University, Research Lab)

**Files:** `libciv/src/game/state.rs`, `libciv/src/game/rules.rs`, `libciv/src/civ/district.rs`

The Campus district exists but has no buildings. Science victory requires a functional science pipeline, so we need the three Campus buildings that generate science yields.

### 3a: Register Campus buildings in GameState

In `game/state.rs`, the `BuildingDef` struct already exists:
```rust
pub struct BuildingDef {
    pub id: BuildingId,
    pub name: &'static str,
    pub cost: u32,
    pub maintenance: u32,
    pub yields: YieldBundle,
    pub requires_district: Option<&'static str>,
}
```

Register three buildings in game initialization (wherever `building_defs` is populated — typically in `civsim/src/main.rs` or test setup):

| Building | Cost | Maintenance | Yields | Required District | Required Tech |
|----------|------|-------------|--------|-------------------|---------------|
| Library | 90 | 1 | +2 science | Campus | Writing |
| University | 250 | 2 | +4 science | Campus | Education |
| Research Lab | 480 | 3 | +5 science | Campus | Chemistry |

**Note:** `BuildingDef.requires_district` uses `Option<&'static str>` (a district name string), not `BuiltinDistrict`. All three buildings use `Some("Campus")`.

### 3b: Add tech gating for buildings

Currently `BuildingDef` has no `required_tech` field. Add one:

```rust
pub struct BuildingDef {
    pub id: BuildingId,
    pub name: &'static str,
    pub cost: u32,
    pub maintenance: u32,
    pub yields: YieldBundle,
    pub requires_district: Option<&'static str>,
    pub required_tech: Option<TechId>,  // NEW
}
```

This allows Library to require Writing, University to require Education, Research Lab to require Chemistry.

### 3c: Implement building production completion in advance_turn

At the TODO on line ~960, add a `ProductionItem::Building(building_id)` match arm:
1. Look up `BuildingDef` by `building_id` in `state.building_defs`.
2. Check `city.production_stored >= def.cost`.
3. Validate: if `requires_district` is set, city must have that district in `city.districts` (match by name).
4. Validate: if `required_tech` is set, civ must have it researched.
5. Validate: building not already in `city.buildings` (no duplicates).
6. On success: deduct production, pop queue, add `building_id` to `city.buildings`, emit `StateDelta::BuildingCompleted`.

### 3d: Include building yields in compute_yields

In `compute_yields()` (line ~700-761 of `rules.rs`), after summing worked tile yields and base yields, add a loop over each city's buildings:

```rust
for &building_id in &city.buildings {
    if let Some(def) = state.building_defs.iter().find(|b| b.id == building_id) {
        yields = yields + def.yields;
    }
}
```

This ensures Library/University/Research Lab science yields flow into the civ's science-per-turn, which in turn drives tech research progress.

### 3e: Deduct building maintenance

In `advance_turn` (after road maintenance in Phase 2c or alongside it), sum building maintenance costs per civ and deduct from gold:

```rust
for city in state.cities.iter().filter(|c| c.owner == civ_id) {
    for &building_id in &city.buildings {
        if let Some(def) = state.building_defs.iter().find(|b| b.id == building_id) {
            building_maintenance += def.maintenance as i32;
        }
    }
}
```

---

## Step 4: Add `Project` Variant to `ProductionItem`

(Previously Step 3)

**File:** `libciv/src/civ/city.rs`

```rust
pub enum ProductionItem {
    Unit(UnitTypeId),
    Building(BuildingId),
    District(BuiltinDistrict),
    Wonder(WonderId),
    Project(&'static str),  // NEW
}
```

Using `&'static str` is consistent with how the codebase references built-in content. Space projects are a small fixed set — a full ID-based registry would be overengineering.

---

## Step 4: Track Completed Projects on Civilization

**File:** `libciv/src/civ/civilization.rs`

Add field to `Civilization`:

```rust
pub completed_projects: Vec<&'static str>,
```

Initialize to `Vec::new()` in `Civilization::new()`. This mirrors `unlocked_units`, `unlocked_buildings`, etc.

---

## Step 5: Add `ProjectCompleted` StateDelta

**File:** `libciv/src/game/diff.rs`

```rust
ProjectCompleted { civ: CivId, city: CityId, project: &'static str },
```

---

## Step 6: Implement Project Production Completion

**File:** `libciv/src/game/rules.rs`

At the TODO on line ~960 (`// TODO(PHASE3-4.3): Building, District, Wonder completion.`), add project completion logic.

### Helper functions (in `rules.rs` or a new `libciv/src/rules/project.rs`):

```rust
fn space_project_cost(name: &str) -> Option<u32> {
    match name {
        "Launch Satellite"         => Some(1500),
        "Moon Colony"              => Some(2000),
        "Mars Colony"              => Some(2500),
        "Interstellar Colony Ship" => Some(3000),
        _ => None,
    }
}

fn space_project_required_tech(name: &str, tech_refs: &TechRefs) -> Option<TechId> {
    match name {
        "Launch Satellite"         => Some(tech_refs.satellites),
        "Moon Colony"              => Some(tech_refs.robotics),
        "Mars Colony"              => Some(tech_refs.nuclear_fusion),
        "Interstellar Colony Ship" => Some(tech_refs.nanotechnology),
        _ => None,
    }
}

fn space_project_prerequisite(name: &str) -> Option<&'static str> {
    match name {
        "Moon Colony"              => Some("Launch Satellite"),
        "Mars Colony"              => Some("Moon Colony"),
        "Interstellar Colony Ship" => Some("Mars Colony"),
        _ => None,
    }
}
```

### Completion logic in `advance_turn` Phase 2a:

After the existing unit completion block, add a `ProductionItem::Project(name)` match arm:

1. Look up cost via `space_project_cost(name)`.
2. Check `city.production_stored >= cost`.
3. Validate: city has `BuiltinDistrict::Spaceport` in `city.districts`.
4. Validate: civ has required tech researched.
5. Validate: prerequisite project (if any) is in `civ.completed_projects`.
6. Validate: project not already in `civ.completed_projects` (no double-build).
7. On success: deduct production, pop queue, add to `civ.completed_projects`, emit `ProjectCompleted`.

Also add stubs/implementations for `Building` and `Wonder` completion at the same TODO site (match must be exhaustive for the new `Project` variant).

---

## Step 7: Implement `ScienceVictory`

**File:** `libciv/src/game/victory.rs`

```rust
#[derive(Debug)]
pub struct ScienceVictory {
    pub id: VictoryId,
}

impl VictoryCondition for ScienceVictory {
    fn id(&self) -> VictoryId { self.id }
    fn name(&self) -> &'static str { "Science Victory" }
    fn description(&self) -> &'static str {
        "Complete all four space race projects: Launch Satellite, Moon Colony, \
         Mars Colony, and Interstellar Colony Ship."
    }
    fn kind(&self) -> VictoryKind { VictoryKind::ImmediateWin }

    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        const PROJECTS: [&str; 4] = [
            "Launch Satellite", "Moon Colony", "Mars Colony", "Interstellar Colony Ship"
        ];
        let completed = state.civ(civ_id)
            .map(|civ| PROJECTS.iter()
                .filter(|p| civ.completed_projects.contains(p))
                .count() as u32)
            .unwrap_or(0);

        VictoryProgress { victory_id: self.id, civ_id, current: completed, target: 4 }
    }
}
```

The existing Phase 5c victory evaluation in `advance_turn` will automatically detect when `current >= target` (4/4) and set `game_over`.

---

## Step 8: Update Exports

**Files:**
- `libciv/src/game/mod.rs` — add `ScienceVictory` to `pub use victory::{...}`
- `libciv/src/lib.rs` — add `ScienceVictory` to the top-level re-export line

---

## Step 9: Integration Tests

**File:** New `libciv/tests/science_victory.rs` (or add to `gameplay.rs`)

Tests:
1. **test_science_victory_progress** — Manually add projects to `civ.completed_projects`, verify `check_progress` reports correct current/target.
2. **test_science_victory_wins_on_all_four** — Complete all 4 projects, verify `is_won()` returns true.
3. **test_science_victory_not_won_with_three** — Complete 3 of 4, verify `is_won()` returns false.
4. **test_project_completion_in_advance_turn** — Set up city with Spaceport + required tech + enough production_stored, queue `ProductionItem::Project("Launch Satellite")`, run `advance_turn`, verify `completed_projects` contains it and `ProjectCompleted` delta emitted.
5. **test_project_requires_spaceport** — Queue project in city without Spaceport, verify it does not complete.
6. **test_project_requires_prerequisite** — Queue "Moon Colony" without "Launch Satellite" completed, verify it stays in queue.
7. **test_science_victory_triggers_game_over** — Register `ScienceVictory`, complete all 4 projects, verify `state.game_over` is set.
8. **test_spaceport_requires_rocketry** — Verify `BuiltinDistrict::Spaceport.requirements()` requires `tech_refs.rocketry`.

---

## Implementation Order

Recommended order: 1 (tech tree) → 2 (Spaceport district) → 3 (Campus buildings + building completion) → 4+5+6 (Project type + completed_projects + ProjectCompleted delta) → 7 (project completion in advance_turn) → 8 (ScienceVictory) → 9 (exports) → 10 (tests)

---

## Verification

1. `cargo build --workspace` — all crates compile
2. `cargo clippy --workspace -- -D warnings` — no warnings
3. `cargo test --workspace` — all existing + new tests pass
4. Key test: end-to-end science victory where a civ completes all 4 projects and `game_over` is set

---

## Edge Cases

- **No double-build**: `completed_projects.contains()` check prevents building the same project twice
- **Multiple Spaceports**: Projects are per-civ (tracked on `Civilization`), not per-city — any city with a Spaceport can build them
- **City destroyed mid-build**: Production is lost naturally; no special handling needed
- **Future Tech**: Repeatable tech (special handling: doesn't get added to `researched_techs` permanently, or is added with score bonus). Can be deferred as a follow-up.
