# Open Civilization VI

this folder contains an implementation of civilization vi in rust. Specifically,
it contains information and datastructures to create a functional engine for
civilization without graphical interaction. This is intended as an eventual
target to which I will apply RL, as if it were an RL environment to see how
agents can handle such a large state with so many variables.

This particular implementation is designed to be extensible with a focus on
eventually allowing users to specify many aspects of their games (civilizations,
natural wonders, great people, resources, units, etc) in lua, without needing to
make direct modifications to the game engine in order to make a modified game.

Store memories in a local memories directory, ./memories

---

## Codebase Structure

The workspace has 7 crates (6 libraries + 1 binary). Dependency order:

```
libcommon  ← base types: IDs (ULID newtypes), YieldBundle, enums
libhexgrid ← pure geometry: HexCoord (cube coords), HexBoard trait, pathfinding traits
libworld   ← map state: WorldTile/WorldEdge, terrain/feature/resource/improvement traits + builtins
librules   ← game rules: modifiers, tech/civic trees, policy/government, victory conditions
libcivcore ← civilization state: Civilization, City, Unit, District, Governor, Religion, etc.
libgame    ← orchestration: GameState, WorldBoard (HexBoard impl), RulesEngine, TurnEngine, diffs
civsim     ← CLI binary: `new` and `run` subcommands via clap
```

### Key files

| File | Purpose |
|------|---------|
| `libcommon/src/ids.rs` | Macro-generated ULID-backed newtype IDs for all entities |
| `libcommon/src/yields.rs` | `YieldType` enum + `YieldBundle` (sparse HashMap) |
| `libcommon/src/enums.rs` | `ResourceCategory`, `UnitDomain`, `UnitCategory`, `GreatPersonType`, `AgeType`, `PolicyType` |
| `libhexgrid/src/coord.rs` | `HexCoord` with invariant enforcement, arithmetic, `HexDir`, distance, neighbors, ring |
| `libhexgrid/src/types.rs` | `MovementCost`, `Elevation` (FLAT/HILLS/MOUNTAIN), `Vision`, `MovementProfile` (Ground/Naval/Air/Embarked) |
| `libhexgrid/src/board.rs` | `HexTile`/`HexEdge`/`HexBoard` traits, `BoardTopology` enum |
| `libworld/src/tile.rs` | `WorldTile` struct (implements `HexTile`): terrain, feature, resource, improvement, road, rivers, owner |
| `libworld/src/terrain.rs` | `TerrainDef` trait + `BuiltinTerrain` (Grassland/Plains/Desert/Tundra/Snow/Coast/Ocean) |
| `libworld/src/feature.rs` | `FeatureDef` trait + `BuiltinFeature` (Forest/Rainforest/Marsh/Floodplain/Reef/Ice/VolcanicSoil/Oasis) |
| `libworld/src/edge.rs` | `EdgeFeatureDef` trait + `WorldEdge` (River/Cliff/Canal/MountainPass) |
| `libworld/src/improvement.rs` | `TileImprovement` trait + builtins (Farm/Mine/LumberMill/TradingPost/Fort/Airstrip/MissileSilo) |
| `libworld/src/road.rs` | `RoadDef` trait + builtins (AncientRoad/MedievalRoad/IndustrialRoad/Railroad) |
| `librules/src/modifier.rs` | `Modifier`, `EffectType`, `TargetSelector`, `StackingRule`, `ModifierSource`, `resolve_modifiers()` |
| `librules/src/tech.rs` | `TechNode`/`CivicNode`, `TechTree`/`CivicTree`, `Unlock` enum, `EurekaCondition` trait |
| `librules/src/policy.rs` | `Policy`, `Government`, `PolicySlots` |
| `librules/src/victory.rs` | `VictoryCondition` trait, `VictoryProgress` |
| `libcivcore/src/civilization.rs` | `Civilization`, `Leader`, `TechProgress`, `CivicProgress`; `LeaderAbility`/`Agenda`/`StartBias` traits |
| `libcivcore/src/city.rs` | `City`, `CityStatus` (Capital/City/Occupied/Puppet/Razed), `WallLevel`, `ProductionItem` |
| `libgame/src/state.rs` | `GameState` (single source of truth), `IdGenerator` (seeded ULID) |
| `libgame/src/board.rs` | `WorldBoard`: `HexBoard` impl with Dijkstra pathfinding and hex LOS |
| `libgame/src/rules.rs` | `RulesEngine` trait + `DefaultRulesEngine` (Phase 2 stubs) |
| `libgame/src/diff.rs` | `StateDelta` enum, `GameStateDiff` |
| `civsim/src/main.rs` | CLI entry point |

### Implementation status

- **Phase 1 complete**: all structs/traits declared, project compiles with zero warnings, 12/12 active tests pass (4 are `#[ignore]`d as Phase 2 targets)
- **Phase 2 (in progress)**: rules evaluation, modifier stacking, LOS with elevation, road movement costs
- **Phase 3+ (future)**: full gameplay loop, Lua scripting API, RL environment interface

### Conventions

- All entity IDs are ULID-backed newtypes defined via macro in `libcommon/src/ids.rs`; never use raw strings or integers as IDs
- `MovementCost` uses integer math scaled by 100 (ONE = 100)
- `BoardTopology` variants: `Flat`, `CylindricalEW` (wraps east-west), `Toroidal`
- Rust edition 2024; workspace resolver 2
- Commits are tagged: `infra`, `impl`, `tests`, `fix`, `docs`
- `YieldBundle` is sparse (HashMap-backed); missing keys default to zero
- The `BuiltinTerrain` / `BuiltinFeature` / `BuiltinRoad` enums are convenience wrappers around the concrete structs that implement the trait — they are not the canonical extension point (traits are)

### Design constraints to keep in mind

- All trait objects that cross the Lua boundary will need `Send + Sync`; avoid `&'static str` for user-visible names — use `String`
- `GameState` is passed by reference to all systems; no global state
- Yields/housing/amenities/loyalty are **never stored** on `City` — computed via `RulesEngine` queries so modifiers always apply correctly (exception: `loyalty` is stored as accumulated state between turns)

### Open architectural questions (under discussion)

- The separation of `libworld`, `librules`, `libcivcore`, and `libgame` may not be the right boundary — Civilization is a naturally coupled game and these libraries may contain leaky abstractions. Merging some or all of them into a single `libcivbase` (keeping only `libhexgrid` as a pure geometry crate) is being actively considered.
- The heavy use of traits throughout may conflict with the goal of Lua-driven extensibility; this needs to be resolved before Phase 3.
