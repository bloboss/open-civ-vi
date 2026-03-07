# Phase 1 Cleanup — Implementation Plan

Derived from discussions in `AGENT_COMMENTS.md` and `AGENT_COMMENT_CLARIFICATIONS.md`.
Conventional commits format: `<type>(<scope>): <description>` per https://www.conventionalcommits.org/en/v1.0.0/
VCS: `jj` — use `jj commit` to set the message and advance, just like in git.
    Note: do not squash the commits when you are finished.

---

## Decisions Made

| Topic | Decision |
|---|---|
| Library structure | Merge `libworld`, `librules`, `libcivcore`, `libgame` into `libciv`; keep `libhexgrid` and `libcommon` separate |
| Extensibility model | Pure Rust trait-based API; static linking only; no dynamic loading or scripting runtime |
| Macro boilerplate | Deferred to Phase 2 — build `define_terrain!` etc. alongside the trait definitions they simplify |
| String types | All content is compile-time Rust — `&'static str` is fine for built-in names; trait methods return `&str`; no `String` conversion needed |
| `Elevation` | `struct Elevation(u8)` → `enum { Low, Level(u8), High }` |
| `Vision` | `struct Vision(u8)` → `enum { Blind, Radius(u8), Omniscient }` |
| `MovementProfile` | Add `Amphibious` variant |
| Edge addressing | `(HexCoord, HexCoord)` sorted pair → `(HexCoord, HexDir)` canonical; forward half = `{E, NE, NW}` |
| `ProductionItem` | `Unit(&'static str)` → `Unit(UnitTypeId)`, `Wonder(&'static str)` → `Wonder(WonderId)` — type safety, not string conversion |
| `CityStatus` | Rename to `CityOwnership { Normal, Occupied, Puppet, Razed }`; remove `Capital`; add `is_capital: bool` to `City`; document `Puppet` |
| `WallLevel` | Keep as enum; add `defense_bonus() -> i32` and `max_hp() -> u32` methods; add `wall_hp: u32` to `City` |
| `Civilization` | Add `strategic_resources: HashMap<ResourceId, u32>`; replace `ability_names: Vec<&'static str>` with `abilities: Vec<Box<dyn LeaderAbility>>`; replace `agenda_name` with `agenda: Box<dyn Agenda>` |
| `StartBias` | Trait methods return `Option<TerrainId>` / `Option<FeatureId>` / `Option<ResourceCategory>` not `&'static str` |
| `DiplomaticStatus` | Remove `ColdWar` and `OpenBorders` (latter is an `Agreement`); add `Denounced`; final: `{ War, Denounced, Neutral, Friendly, Alliance }` |
| Grievances | Replace scalar `i32` totals with `Vec<GrievanceRecord>` on each side; `GrievanceRecord { id, description: &'static str, amount: i32, visibility: GrievanceVisibility, recorded_turn: u32 }`; `enum GrievanceVisibility { Public, RequiresSpy, RequiresAlliance }` |
| `AdjacencyContext` | Replace `adjacent_natural_wonders: u32` count with `adjacent_natural_wonders: Vec<NaturalWonderId>` |
| `CityState` | Fold into `City` with `kind: CityKind` discriminant; `CityKind::CityState(CityStateData)` holds suzerain/influence; mechanics stay in `city_state.rs` module |
| Scripting/config | Pure Rust only; TOML+Rhai documented as future option if hot-reloading ever becomes necessary |

## Deferred (not in this patch)

- Macros for trait boilerplate (`define_terrain!` etc.) — Phase 2
- `CityCondition` computed flags (`Starving`, `LowHousing`, `UnderSiege`) — Phase 2, computed by rules engine
- Dynamic library loading — indefinitely deferred
- Phase 2 gameplay rules (modifier stacking, LOS with elevation, etc.)

---

## Commits

### 1 — `refactor(workspace): merge libworld, librules, libcivcore, libgame into libciv`

**Goal:** Single `libciv` crate with all domain logic. Crate structure after:
```
libcommon   (unchanged)
libhexgrid  (unchanged in this commit)
libciv      (new — absorbs libworld, librules, libcivcore, libgame)
civsim      (updated dep)
```

**Files changed:**
- Create `libciv/Cargo.toml` — depends on `libcommon`, `libhexgrid`, `ulid`, `rand`
- Create `libciv/src/lib.rs` — re-exports modules grouped as `world::`, `rules::`, `civ::`, `game::`
- Move `libworld/src/*` → `libciv/src/world/`
- Move `librules/src/*` → `libciv/src/rules/`
- Move `libcivcore/src/*` → `libciv/src/civ/`
- Move `libgame/src/*` → `libciv/src/game/`
- Update all `use lib{world,rules,civcore,game}::` imports inside moved files to `crate::`/`super::`
- `Cargo.toml` (workspace): replace `libworld`, `librules`, `libcivcore`, `libgame` members with `libciv`
- `civsim/Cargo.toml`: replace four deps with `libciv`
- Delete `libworld/`, `librules/`, `libcivcore/`, `libgame/` directories

**Tests:** All 12 currently-passing tests must still pass.

---

### 2 — `refactor(libhexgrid): convert Elevation and Vision from newtype structs to semantic enums`

**Files changed:** `libhexgrid/src/types.rs`, `libciv/src/game/board.rs`

**`Elevation`** before:
```rust
pub struct Elevation(pub u8);
impl Elevation {
    pub const FLAT: Elevation = Elevation(0);
    pub const HILLS: Elevation = Elevation(1);
    pub const MOUNTAIN: Elevation = Elevation(2);
}
```
After:
```rust
/// Low = below sea level (ocean floor). High = impassable peak (mountain).
/// Level(n) = landmass elevation; higher n = higher ground.
/// Ordering: Low < Level(0) < Level(1) < ... < Level(255) < High
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Elevation { Low, Level(u8), High }
```

**`Vision`** before:
```rust
pub struct Vision(pub u8);
```
After:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vision { Blind, Radius(u8), Omniscient }
```

**`board.rs` (libciv):** Update LOS comparisons.
- `Elevation::FLAT` → `Elevation::Level(0)`
- `Elevation::MOUNTAIN` → `Elevation::High`
- `min_elev` computation uses the new `Ord` impl (derived order gives correct semantics: `Low < Level(_) < High`)
- `tile.elevation() > min_elev` is valid unchanged under the new `Ord`

**Tests:** Update any tests referencing `Elevation::FLAT`/`HILLS`/`MOUNTAIN` or `Vision(n)`.

---

### 3 — `feat(libhexgrid): add Amphibious variant to MovementProfile`

**Files changed:** `libhexgrid/src/types.rs`

```rust
pub enum MovementProfile {
    Ground,
    Naval,
    Air,
    Embarked,
    /// Unit that moves on both land and water at full efficiency (e.g. Giant Death Robot).
    /// Uses land movement costs on land tiles and naval costs on water tiles.
    Amphibious,
}
```

No callers need updating — existing match arms are exhaustive only in `DefaultRulesEngine` stubs which return `todo!()`.

---

### 4 — `refactor(libhexgrid): readdress edges via (HexCoord, HexDir) with canonical normalization`

**Files changed:**
- `libhexgrid/src/board.rs` — update `HexEdge` trait and `HexBoard::edge` signature
- `libciv/src/world/edge.rs` — update `WorldEdge` struct
- `libciv/src/game/board.rs` — update `WorldBoard` edge storage, `set_edge`, `edge`, `edge_key`

**`HexEdge` trait:**
```rust
pub trait HexEdge {
    fn coord(&self) -> HexCoord;
    fn dir(&self) -> HexDir;
    /// Both tiles this edge separates, derived from coord+dir.
    fn endpoints(&self) -> (HexCoord, HexCoord) {
        (self.coord(), self.coord() + self.dir().unit_vec())
    }
    fn crossing_cost(&self) -> MovementCost;
}
```

**`HexBoard::edge` signature:**
```rust
fn edge(&self, coord: HexCoord, dir: HexDir) -> Option<&Self::Edge>;
```

**`WorldEdge`** before: `{ from: HexCoord, to: HexCoord, feature: ... }`
After: `{ coord: HexCoord, dir: HexDir, feature: ... }`

**Canonicalization in `WorldBoard`:**
- Forward half: `{E, NE, NW}`. Backward: `{W, SW, SE}`.
- `fn canonical(coord: HexCoord, dir: HexDir) -> (HexCoord, HexDir)`:
  - If `dir` ∈ `{E, NE, NW}`: return `(coord, dir)` unchanged
  - If `dir` ∈ `{W, SW, SE}`: return `(coord + dir.unit_vec(), dir.opposite())`
- `edges: HashMap<(HexCoord, HexDir), WorldEdge>` (key is always canonical)
- `edge(coord, dir)` and `set_edge()` both canonicalize before access

**Update Dijkstra** — change `self.edge(coord, neighbor)` to `self.edge(coord, dir)` where `dir`
comes from iterating `HexDir::all()` rather than deriving from coordinate difference.

---

### 5 — `refactor(libciv): refactor City types — CityOwnership, typed ProductionItem IDs, is_capital`

**Files changed:** `libciv/src/civ/city.rs`, `libcommon/src/ids.rs` (add `WonderId` if missing)

**`CityStatus`** → **`CityOwnership`**:
```rust
/// Political/ownership state of a city.
/// Transient conditions (Starving, LowHousing, UnderSiege) are computed each turn
/// by the rules engine — they are not stored here.
pub enum CityOwnership {
    /// Owned and fully managed by the current civilization.
    Normal,
    /// Captured; owner manages production queue but suffers loyalty/amenity penalties.
    Occupied,
    /// Captured but not annexed; AI manages production queue on the owner's behalf.
    /// Still generates yields and counts toward empire size. Distinct from Occupied
    /// in that the owner does not directly control production choices.
    Puppet,
    /// Being razed; removed from the map when raze_turns reaches zero (Phase 2).
    Razed,
}
```

**`City`** struct changes:
- Rename `status: CityStatus` → `ownership: CityOwnership`
- Add `is_capital: bool` (initialized to `false` in `City::new()`)
- Update `City::is_capital()` to return `self.is_capital`

**`ProductionItem`** — type-safe IDs:
```rust
pub enum ProductionItem {
    Unit(UnitTypeId),       // was Unit(&'static str)
    Building(BuildingId),
    District(DistrictTypeId),
    Wonder(WonderId),       // was Wonder(&'static str)
    Project(ProjectId),
}
```

---

### 6 — `feat(libciv): add WallLevel stat methods and wall_hp field to City`

**Files changed:** `libciv/src/civ/city.rs`

```rust
impl WallLevel {
    /// Combat strength bonus granted to the city's ranged attack and defense.
    pub fn defense_bonus(&self) -> i32 {
        match self {
            WallLevel::None        => 0,
            WallLevel::Ancient     => 3,
            WallLevel::Medieval    => 5,
            WallLevel::Renaissance => 8,
        }
    }

    /// Maximum HP of walls at this tier.
    pub fn max_hp(&self) -> u32 {
        match self {
            WallLevel::None        => 0,
            WallLevel::Ancient     => 50,
            WallLevel::Medieval    => 100,
            WallLevel::Renaissance => 200,
        }
    }
}
```

Add to `City`:
```rust
pub wall_hp: u32,  // current wall HP; 0 when WallLevel::None
```
Initialize in `City::new()`: `wall_hp: WallLevel::None.max_hp()` (= 0).

---

### 7 — `feat(libciv): add strategic_resources and replace Leader ability stubs with trait objects`

**Files changed:** `libciv/src/civ/civilization.rs`, `civsim/src/main.rs`

**`Civilization`** — add field:
```rust
pub strategic_resources: HashMap<ResourceId, u32>,
```
Initialize to `HashMap::new()` in `Civilization::new()`.

**`Leader`** — replace placeholder string stubs with live trait objects:
```rust
pub struct Leader {
    pub name: &'static str,
    pub civ_id: CivId,
    pub abilities: Vec<Box<dyn LeaderAbility>>,  // replaces ability_names: Vec<&'static str>
    pub agenda: Box<dyn Agenda>,                 // replaces agenda_name: &'static str
}
```

`Civilization::new()` takes a fully constructed `Leader`. Update `civsim/src/main.rs` to
construct a minimal concrete `Leader` (stub `LeaderAbility` and `Agenda` impls inline or in a
`stubs` module).

---

### 8 — `refactor(libciv): update StartBias to typed IDs and fix DiplomaticStatus and grievances`

**Files changed:** `libciv/src/civ/civilization.rs`, `libciv/src/civ/diplomacy.rs`

**`StartBias` trait:**
```rust
pub trait StartBias: std::fmt::Debug {
    fn terrain_preference(&self) -> Option<TerrainId>;
    fn feature_preference(&self) -> Option<FeatureId>;
    fn resource_preference(&self) -> Option<ResourceCategory>;
}
```

**`DiplomaticStatus`** — remove `ColdWar` and `OpenBorders`; add `Denounced`:
```rust
pub enum DiplomaticStatus { War, Denounced, Neutral, Friendly, Alliance }
```

**`GrievanceRecord`** — new struct:
```rust
pub struct GrievanceRecord {
    pub grievance_id: GrievanceId,
    pub description: &'static str,
    pub amount: i32,
    pub visibility: GrievanceVisibility,
    pub recorded_turn: u32,
}

pub enum GrievanceVisibility { Public, RequiresSpy, RequiresAlliance }
```

**`DiplomaticRelation`** — replace scalar totals:
```rust
pub struct DiplomaticRelation {
    pub civ_a: CivId,
    pub civ_b: CivId,
    pub status: DiplomaticStatus,
    pub grievances_a_against_b: Vec<GrievanceRecord>,
    pub grievances_b_against_a: Vec<GrievanceRecord>,
    pub active_agreements: Vec<AgreementId>,
    pub turns_at_war: u32,
}
```

Add helper:
```rust
pub fn opinion_score_a_toward_b(&self) -> i32 {
    self.grievances_a_against_b.iter().map(|g| -g.amount).sum()
}
```

Update `DiplomaticRelation::add_grievance(by: CivId, record: GrievanceRecord)`.
Update built-in `GrievanceTrigger` impls to produce `GrievanceRecord`s with `visibility: Public`.

---

### 9 — `refactor(libciv): replace natural_wonder count with typed IDs in AdjacencyContext`

**Files changed:** `libciv/src/civ/district.rs`, `libcommon/src/ids.rs`

Add `NaturalWonderId` to `libcommon/src/ids.rs` via the existing ID newtype macro.

**`AdjacencyContext`** — replace count with typed list:
```rust
// before:
pub adjacent_natural_wonders: u32,

// after:
pub adjacent_natural_wonders: Vec<NaturalWonderId>,
```

Allows district adjacency bonus logic to identify which wonder is adjacent (e.g. Uluru grants
+2 Faith to adjacent Holy Sites specifically).

Update `AdjacencyContext::new()` — `adjacent_natural_wonders: Vec::new()`.

---

### 10 — `refactor(libciv): fold CityState into City with CityKind discriminant`

**Files changed:** `libciv/src/civ/city.rs`, `libciv/src/civ/city_state.rs`, `libciv/src/game/state.rs`

**`CityKind`** and **`CityStateData`** — added to `city.rs`:
```rust
pub enum CityKind {
    /// A standard player or AI city.
    Regular,
    /// An independent city-state. Suzerain/influence mechanics live in CityStateData.
    CityState(CityStateData),
}

pub struct CityStateData {
    pub state_type: CityStateType,
    pub suzerain: Option<CivId>,
    pub influence: HashMap<CivId, i32>,
}
```

**`City`** — add field:
```rust
pub kind: CityKind,  // initialized to CityKind::Regular in City::new()
```

**`city_state.rs`** — keep `CityStateType`, `CityStateBonus` trait, and the helper methods
(`is_suzerain`, `get_influence`, `recalculate_suzerain`) as free functions or methods on
`CityStateData`. Remove the standalone `CityState` struct.

**`GameState`** — remove `city_states: HashMap<CivId, CityState>`; city states now live in
`cities: HashMap<CityId, City>` with `kind: CityKind::CityState(...)`. Add lookup helper:

```rust
/// Returns the city that represents the given city-state CivId, if one exists.
/// City states are stored in the cities map with owner == their diplomatic CivId.
pub fn city_state_by_civ(&self, civ_id: CivId) -> Option<&City> {
    self.cities.values().find(|c| {
        matches!(c.kind, CityKind::CityState(_)) && c.owner == civ_id
    })
}
```

---

### 11 — `docs: consolidate architecture decisions into AGENTS.md`

**Files changed:** `AGENTS.md`

- Record all finalised decisions from this plan
- Note deferred items (macros, CityCondition, dynamic loading)
- Document the edge canonicalization rule
- Note VCS is `jj`; commit convention is Conventional Commits

`PLAN.md`, `AGENT_COMMENTS.md`, `AGENT_COMMENT_CLARIFICATIONS.md` are retained as
historical record but noted as superseded by `AGENTS.md`.

---

## Commit sequence summary

```
1   refactor(workspace): merge libworld, librules, libcivcore, libgame into libciv
2   refactor(libhexgrid): convert Elevation and Vision to semantic enums
3   feat(libhexgrid): add Amphibious variant to MovementProfile
4   refactor(libhexgrid): readdress edges via (HexCoord, HexDir) with canonical normalization
5   refactor(libciv): refactor City types — CityOwnership, typed ProductionItem IDs, is_capital
6   feat(libciv): add WallLevel stat methods and wall_hp field to City
7   feat(libciv): add strategic_resources and replace Leader ability stubs with trait objects
8   refactor(libciv): update StartBias to typed IDs, fix DiplomaticStatus and grievances
9   refactor(libciv): replace natural_wonder count with typed IDs in AdjacencyContext
10  refactor(libciv): fold CityState into City with CityKind discriminant
11  docs: consolidate architecture decisions into AGENTS.md
```

Each commit must leave `cargo build` and `cargo test` clean before the next begins.
