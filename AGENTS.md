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
civsim      ← CLI binary: `new`, `run`, `demo`, `ai-demo`, `play` subcommands via clap
open4x-web  ← Leptos/WASM frontend (imports libciv compiled to wasm32)
```

> **History**: libcommon, libworld, librules, libcivcore, and libgame were all merged into
> libciv. libhexgrid remains separate as a clean geometry crate with zero game knowledge.

### Key files

| File | Purpose |
|------|---------|
| `libciv/src/ids.rs` | Macro-generated ULID-backed newtype IDs for all entities |
| `libciv/src/yields.rs` | `YieldType` enum + `YieldBundle` (flat struct, not HashMap) |
| `libciv/src/enums.rs` | `ResourceCategory`, `UnitDomain`, `UnitCategory`, `GreatPersonType`, `AgeType`, `PolicyType` |
| `libhexgrid/src/coord.rs` | `HexCoord` with invariant enforcement, arithmetic, `HexDir`, distance, neighbors, ring |
| `libhexgrid/src/types.rs` | `MovementCost`, `Elevation` enum, `Vision` enum, `MovementProfile` enum |
| `libhexgrid/src/board.rs` | `HexTile`/`HexEdge`/`HexBoard` traits, `BoardTopology` enum |
| `libciv/src/world/tile.rs` | `WorldTile` (implements `HexTile`): terrain, hills, feature, resource, improvement, road, rivers, natural_wonder, owner |
| `libciv/src/world/terrain.rs` | `BuiltinTerrain` plain enum (Grassland/Plains/Desert/Tundra/Snow/Coast/Ocean/Mountain) |
| `libciv/src/world/feature.rs` | `BuiltinFeature` plain enum (Forest/Rainforest/Marsh/Floodplain/Reef/Ice/VolcanicSoil/Oasis) |
| `libciv/src/world/resource.rs` | `BuiltinResource` plain enum (8 bonus, 8 luxury, 7 strategic); `reveal_tech()` for gating |
| `libciv/src/world/edge.rs` | `EdgeFeatureDef` trait + `WorldEdge` (River/Canal/MountainPass); stores `(HexCoord, HexDir)` |
| `libciv/src/world/improvement.rs` | `BuiltinImprovement` (Farm/Mine/LumberMill/TradingPost/Fort/Airstrip/MissileSilo) + `ImprovementRequirements` |
| `libciv/src/world/road.rs` | `RoadDef` trait + `BuiltinRoad` (Ancient/Medieval/Industrial/Railroad); `as_def()` wrapper |
| `libciv/src/world/wonder.rs` | `NaturalWonder` trait + `BuiltinNaturalWonder` (5 wonders); `as_def()` wrapper |
| `libciv/src/rules/modifier.rs` | `Modifier`, `EffectType`, `TargetSelector`, `StackingRule`, `ModifierSource`, `resolve_modifiers()` |
| `libciv/src/rules/effect.rs` | `OneShotEffect` enum + `CascadeClass`; replaces old `Unlock` enum |
| `libciv/src/rules/tech.rs` | `TechNode`/`CivicNode`, `TechTree`/`CivicTree`; nodes carry `Vec<OneShotEffect>` (not `Vec<Unlock>`) |
| `libciv/src/rules/tech_tree_def.rs` | Full tech tree definition (Pottery, Mining, …) with effects and eureka data |
| `libciv/src/rules/civic_tree_def.rs` | Full civic tree definition with inspiration data |
| `libciv/src/rules/policy.rs` | `Policy`, `Government`, `PolicySlots` |
| `libciv/src/rules/victory.rs` | `VictoryCondition` trait, `VictoryProgress` (no concrete implementations yet) |
| `libciv/src/civ/civilization.rs` | `Civilization` (research_queue VecDeque, unlocked_* tracking, fog-of-war sets), `TechProgress`, `CivicProgress`, `Leader`, `LeaderAbility`/`Agenda` traits |
| `libciv/src/civ/city.rs` | `City`, `CityKind`, `CityOwnership`, `WallLevel`, `ProductionItem`; `worked_tiles` + `locked_tiles` |
| `libciv/src/civ/city_state.rs` | `CityStateType`, `CityStateBonus` trait, `CityStateData` |
| `libciv/src/civ/diplomacy.rs` | `DiplomaticRelation`, `DiplomaticStatus`, `GrievanceRecord`, `GrievanceVisibility` |
| `libciv/src/civ/district.rs` | `DistrictDef`/`BuildingDef` traits, `AdjacencyContext`, `PlacedDistrict` |
| `libciv/src/civ/religion.rs` | `Religion`, `Belief` trait, `BeliefContext` (no spread/conversion logic) |
| `libciv/src/civ/trade.rs` | `TradeRoute`; `is_international(&[City])` compares city owners; `compute_route_yields(bool)` returns gold yields; full lifecycle via `establish_trade_route()` |
| `libciv/src/game/state.rs` | `GameState` (single source of truth), `IdGenerator`, `UnitTypeDef`, `BuildingDef` registries, `effect_queue` |
| `libciv/src/game/board.rs` | `WorldBoard`: `HexBoard` impl, Dijkstra pathfinding (road override active), LOS, edge canonicalization |
| `libciv/src/game/rules.rs` | `RulesEngine` trait (14 methods) + `DefaultRulesEngine` |
| `libciv/src/game/diff.rs` | `StateDelta` enum (35+ variants), `GameStateDiff`, `AttackType` |
| `libciv/src/game/turn.rs` | `TurnEngine` (stub: calls `advance_turn`, discards diff) |
| `libciv/src/game/visibility.rs` | `recalculate_visibility()` free function |
| `libciv/src/ai/deterministic.rs` | `Agent` trait + `HeuristicAgent` (exploration/production heuristic) |
| `libciv/tests/common/` | Shared `Scenario` setup used by all integration tests |
| `libciv/tests/gameplay.rs` | End-to-end integration tests |
| `libciv/tests/improvements.rs` | Improvement placement validation tests |
| `civsim/src/main.rs` | CLI entry point with interactive `play` loop |

### Implementation status

- **Active**: rules evaluation, modifier stacking, LOS, road movement, AI agent, improvement
  placement with tech/terrain validation, district placement with territory/tech/civic validation,
  trade routes (establish/expire/yields), cultural border expansion (per-city culture accumulation,
  tile claiming), research queue, base city science + culture (1/turn each), Leptos/WASM frontend;
  all tests pass with zero ignored (169+ tests)
- **Next**: coherent map generation, city defenses + ranged attacks, road placement by builders,
  builder charges, religion founding + spread, loyalty system, TurnEngine diff aggregation

### Conventions

- All entity IDs are ULID-backed newtypes defined via `define_id!` macro in `libciv/src/ids.rs`
- `MovementCost` uses integer math scaled by 100 (ONE = 100, TWO = 200, THREE = 300)
- `BoardTopology` variants: `Flat`, `CylindricalEW` (wraps east-west), `Toroidal`
- Rust edition 2024; workspace resolver 2
- Commit format: **jj** (not git). **conventional commits** (`impl:`, `fix:`, `infra:`, `tests:`, `docs:`, `plan:`)
- Built-in terrain, features, resources, and improvements use **plain enums** with direct method dispatch — no trait objects, no inner zero-sized structs
- `BuiltinRoad`, `BuiltinEdgeFeature`, `BuiltinNaturalWonder` still use the wrapper-with-`as_def()` pattern (trait objects needed for extensibility)
- `&'static str` is appropriate for all built-in content names — compile-time content, not user data
- Structs containing `Box<dyn Trait>` fields (`Leader`, `Civilization`) do **not** derive `Clone`
- `YieldBundle` is a flat struct (named `i32` fields), not HashMap-backed

### Architectural decisions (settled)

- **Extensibility**: pure Rust trait-based API, static linking. No dynamic linking, no scripting.
- **No `String` for built-in names**: `&'static str` is intentional. Only user/external data uses `String`.
- **Elevation**: `enum { Low, Level(u8), High }` — `Level(0)` = flat, `Level(1)` = hills, `High` = impassable mountain peak. Hills are also stored as `WorldTile::hills: bool` to allow hills + terrain combinations.
- **Edge addressing**: `(HexCoord, HexDir)` with canonical normalization — forward half `{E, NE, NW}` is canonical; `{W, SW, SE}` map to `(neighbor, opposite_dir)`
- **CityState**: folded into `City` via `CityKind { Regular, CityState(CityStateData) }`. `GameState::city_state_by_civ(CivId)` finds a city-state by its diplomatic CivId.
- **CityOwnership**: `Normal | Occupied | Puppet | Razed`. Capital status is `is_capital: bool` on City.
- **ProductionItem**: typed IDs — `Unit(UnitTypeId)`, `Building(BuildingId)`, `District(DistrictTypeId)`, `Wonder(WonderId)`
- **Tech completion effects**: use `OneShotEffect` enum (not old `Unlock`). Applied via two-phase `effect_queue` drain to prevent cascades.
- **Research tracking**: `research_queue: VecDeque<TechProgress>` (ordered queue; front is active). Civics use `civic_in_progress: Option<CivicProgress>` (single active).
- **Yields never cached on City/Civ**: computed at query time via `RulesEngine::compute_yields()`. Exception: production and food accumulation stored for multi-turn progress.
- **Base science**: every city owned by a civ contributes +1 science/turn as a base, applied in `compute_yields` before modifiers.

### Design constraints

- `GameState` is passed by reference to all systems; no global state
- `libhexgrid` must remain zero-knowledge of game concepts
- All `RulesEngine` operations return `GameStateDiff` (supporting replay and RL observation)
- Collections are `Vec` with linear-scan lookups; indexed maps only if profiling demands

---

## Remaining Systems to Implement

See `ARCHITECTURE.md` §8 for detailed specs. Summary:

| # | System | Status |
|---|--------|--------|
| 8.1 | Coherent map generation | Not started |
| 8.2 | District placement + building construction | ✅ District placement; building construction in production queue |
| 8.3 | City defenses + ranged city attacks | Data structures only |
| 8.4 | Trade routes + trader units | ✅ Implemented (establish/expire/yields/civsim UI) |
| 8.5 | Religion (founding, spread, beliefs) | Stub only |
| 8.6 | Culture borders, loyalty, tourism | ✅ Borders; loyalty/tourism not started |
| 8.7 | Strategic resource consumption for units | Not started |
| 8.8 | Road placement by builders | Not started |
| 8.9 | Builder charges | Not started |
| 8.10 | Great people accumulation + recruitment | Data structures only |
| 8.11 | Era score and age system | Not started |
| 8.12 | Governor assignment + effects | Data structures only |
| 8.13 | Victory condition evaluation | Trait only |
| 8.14 | Natural wonder discovery events | Not started |
| 8.15 | TurnEngine consolidation (diff aggregation) | Stub |
