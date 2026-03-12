# Open Civilization VI

This folder contains a Rust implementation of a Civilization VI-style engine — data
structures and game logic for a functional engine without a graphical frontend. The
primary goal is an eventual RL environment where agents can interact with a complex
multi-variable game state.

The system is designed to be extensible via a **pure Rust trait-based API**: game
content (civilizations, units, terrain, buildings, wonders, etc.) is defined by
implementing the relevant traits in a downstream crate and linking at compile time.
There is no runtime scripting layer.

Store memories in a local memories directory, ./memory

---

## Codebase Structure

The workspace has 4 crates (3 libraries + 1 binary). Dependency order:

```
libhexgrid  ← pure geometry: HexCoord (cube coords), HexBoard/HexTile/HexEdge traits, pathfinding
libciv      ← all game state and rules: IDs, yields, enums, world, civ, rules, game, ai
civsim      ← CLI binary: `new` and `run` subcommands via clap
open4x-web  ← Leptos/WASM frontend (imports libciv compiled to wasm32)
```

> **History**: libcommon, libworld, librules, libcivcore, and libgame were all merged into
> libciv. libhexgrid remains separate as a clean geometry crate with zero game knowledge.

### Key files

| File | Purpose |
|------|---------|
| `libciv/src/ids.rs` | Macro-generated ULID-backed newtype IDs for all entities |
| `libciv/src/yields.rs` | `YieldType` enum + `YieldBundle` (sparse HashMap) |
| `libciv/src/enums.rs` | `ResourceCategory`, `UnitDomain`, `UnitCategory`, `GreatPersonType`, `AgeType`, `PolicyType` |
| `libhexgrid/src/coord.rs` | `HexCoord` with invariant enforcement, arithmetic, `HexDir`, distance, neighbors, ring |
| `libhexgrid/src/types.rs` | `MovementCost`, `Elevation` enum, `Vision` enum, `MovementProfile` enum |
| `libhexgrid/src/board.rs` | `HexTile`/`HexEdge`/`HexBoard` traits, `BoardTopology` enum |
| `libciv/src/world/tile.rs` | `WorldTile` (implements `HexTile`): terrain, feature, resource, improvement, road, rivers, owner |
| `libciv/src/world/terrain.rs` | `TerrainDef` trait + `BuiltinTerrain` (Grassland/Plains/Desert/Tundra/Snow/Coast/Ocean) |
| `libciv/src/world/feature.rs` | `FeatureDef` trait + `BuiltinFeature` (Forest/Rainforest/Marsh/Floodplain/Reef/Ice/VolcanicSoil/Oasis) |
| `libciv/src/world/edge.rs` | `EdgeFeatureDef` trait + `WorldEdge` (River/Cliff/Canal/MountainPass); stores `(HexCoord, HexDir)` |
| `libciv/src/world/improvement.rs` | `TileImprovement` trait + builtins (Farm/Mine/LumberMill/TradingPost/Fort/Airstrip/MissileSilo) |
| `libciv/src/world/road.rs` | `RoadDef` trait + builtins (AncientRoad/MedievalRoad/IndustrialRoad/Railroad) |
| `libciv/src/rules/modifier.rs` | `Modifier`, `EffectType`, `TargetSelector`, `StackingRule`, `ModifierSource`, `resolve_modifiers()` |
| `libciv/src/rules/tech.rs` | `TechNode`/`CivicNode`, `TechTree`/`CivicTree`, `Unlock` enum, `EurekaCondition` trait |
| `libciv/src/rules/policy.rs` | `Policy`, `Government`, `PolicySlots` |
| `libciv/src/rules/victory.rs` | `VictoryCondition` trait, `VictoryProgress` |
| `libciv/src/civ/civilization.rs` | `Civilization` (with `strategic_resources`), `Leader` (trait objects), `TechProgress`, `CivicProgress`; `LeaderAbility`/`Agenda`/`StartBias` traits |
| `libciv/src/civ/city.rs` | `City`, `CityKind`, `CityOwnership`, `WallLevel`, `ProductionItem` (typed IDs) |
| `libciv/src/civ/city_state.rs` | `CityStateType`, `CityStateBonus` trait, `CityStateData` |
| `libciv/src/civ/diplomacy.rs` | `DiplomaticRelation`, `DiplomaticStatus`, `GrievanceRecord`, `GrievanceVisibility` |
| `libciv/src/civ/district.rs` | `DistrictDef`/`BuildingDef` traits, `AdjacencyContext` (with `Vec<NaturalWonderId>`), `PlacedDistrict` |
| `libciv/src/game/state.rs` | `GameState` (single source of truth), `IdGenerator` (seeded ULID), `city_state_by_civ()` helper |
| `libciv/src/game/board.rs` | `WorldBoard`: `HexBoard` impl with Dijkstra pathfinding and edge canonicalization |
| `libciv/src/game/rules.rs` | `RulesEngine` trait + `DefaultRulesEngine` |
| `libciv/src/game/diff.rs` | `StateDelta` enum, `GameStateDiff` |
| `libciv/src/ai/deterministic.rs` | `Agent` trait + `HeuristicAgent` (exploration/production heuristic) |
| `libciv/tests/common/` | Shared `Scenario` setup used by all integration tests |
| `libciv/tests/gameplay.rs` | End-to-end integration tests |
| `civsim/src/main.rs` | CLI entry point |

### Implementation status

- **Phase 2 active**: rules evaluation, modifier stacking, LOS, road movement, AI agent,
  Leptos/WASM frontend all working; all tests pass with zero ignored
- **Phase 3 (next)**: full gameplay systems — combat, district placement, trade routes,
  religion, diplomacy state machine, victory evaluation, RL environment interface

### Conventions

- All entity IDs are ULID-backed newtypes defined via macro in `libcommon/src/ids.rs`;
  never use raw strings or integers as IDs
- `MovementCost` uses integer math scaled by 100 (ONE = 100)
- `BoardTopology` variants: `Flat`, `CylindricalEW` (wraps east-west), `Toroidal`
- Rust edition 2024; workspace resolver 2
- Commit format: **conventional commits** (`impl:`, `fix:`, `infra:`, `tests:`, `docs:`, `plan:`)
- `YieldBundle` is sparse (HashMap-backed); missing keys default to zero
- `&'static str` is appropriate for all built-in content names, descriptions, and
  identifiers — this is compile-time content, not user-entered data
- Structs containing `Box<dyn Trait>` fields (Leader, Civilization, City) do **not**
  derive `Clone` — trait objects are not Clone
- The `BuiltinTerrain` / `BuiltinFeature` / `BuiltinRoad` enums are convenience wrappers
  around concrete structs that implement the trait — they are not the canonical extension
  point (traits are)

### Architectural decisions (Phase 1 settled)

- **Extensibility**: pure Rust trait-based API, static linking. No dynamic linking, no
  scripting runtime. Downstream crates implement content traits and recompile.
- **No `String` for built-in names**: `&'static str` is intentional and correct for
  compile-time game content. Only external/user data at system boundaries should use `String`.
- **Elevation**: `enum { Low, Level(u8), High }` with constants `FLAT = Level(0)`,
  `HILLS = Level(1)`, `MOUNTAIN = High`
- **Vision**: `enum { Blind, Radius(u8), Omniscient }`
- **MovementProfile**: Ground / Naval / Air / Embarked / **Amphibious**
- **Edge addressing**: `(HexCoord, HexDir)` with canonical normalization — forward half
  `{E, NE, NW}` is canonical; `{W, SW, SE}` map to `(neighbor, opposite_dir)`
- **CityState**: folded into `City` via `CityKind { Regular, CityState(CityStateData) }`.
  `GameState::city_state_by_civ(CivId)` finds a city-state by its diplomatic CivId.
- **CityOwnership**: `Normal | Occupied | Puppet | Razed`. Capital status is `is_capital:
  bool` on City, not an ownership variant.
- **ProductionItem**: typed IDs — `Unit(UnitTypeId)`, `Building(BuildingId)`,
  `District(DistrictTypeId)`, `Wonder(WonderId)`
- **WallLevel**: enum with methods `defense_bonus() -> i32` and `max_hp() -> u32`;
  City stores `wall_hp: u32` for current HP
- **Civilization**: has `strategic_resources: HashMap<ResourceId, u32>`
- **Leader**: no stub strings; uses `abilities: Vec<Box<dyn LeaderAbility>>` and
  `agenda: Box<dyn Agenda>`
- **StartBias**: `terrain_preference() -> Option<TerrainId>`, `feature_preference() ->
  Option<FeatureId>`, `resource_preference() -> Option<ResourceCategory>` (all typed)
- **DiplomaticStatus**: `War | Denounced | Neutral | Friendly | Alliance`
  (no ColdWar, no OpenBorders)
- **Grievances**: `Vec<GrievanceRecord>` with `description: &'static str`, `amount: i32`,
  `visibility: GrievanceVisibility { Public | RequiresSpy | RequiresAlliance }`
- **AdjacencyContext**: `adjacent_natural_wonders: Vec<NaturalWonderId>` (typed, not count)

### Design constraints

- `GameState` is passed by reference to all systems; no global state
- Yields/housing/amenities/loyalty are **never stored** on `City` — computed via
  `RulesEngine` queries so modifiers always apply correctly (exception: accumulated loyalty
  between turns)
- `libhexgrid` must remain zero-knowledge of game concepts — it is a pure geometry/graph
  library; game content belongs in libciv
- Phase 2 macro helpers: write Rust procedural macros to reduce trait implementation
  boilerplate for content definitions (deferred)
