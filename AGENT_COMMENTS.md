# Agent Response to COMMENTS.md

## General Notes

### Library consolidation

**Agree strongly.** The original architecture spec imposed strict separation (libworld has no knowledge of civilizations, librules has no knowledge of world state, etc.), but this breaks down immediately in practice. Examples:

- `TileImprovement::tile_yields` already takes `ImprovementContext<'a>` which holds a `&Civilization` — libworld is already coupled to libcivcore concepts
- `AdjacencyContext` for districts requires `&[&WorldTile]` — libcivcore is already coupled to libworld
- The `RulesEngine` query methods all take `&GameState`, which contains everything

The coupling is real and unavoidable. Civ VI's mechanics are inherently holistic: a district's adjacency bonus depends on terrain, river proximity, wonders, and other districts simultaneously. Forcing this through trait boundaries produces either anemic interfaces or leaking abstractions at every seam.

**Recommendation:** Keep `libhexgrid` as a pure geometry crate (it has no domain coupling and is genuinely reusable). Merge `libworld`, `librules`, `libcivcore`, and `libgame` into a single `libcivbase`. The module structure within that crate can still follow the same logical groupings without the inter-crate boundary overhead.

### Trait overuse vs. Lua extensibility

**Agree this is a real tension.** The current design uses traits everywhere (TerrainDef, FeatureDef, TileImprovement, DistrictDef, BuildingDef, GovernorDef, LeaderAbility, Agenda, Agreement, GrievanceTrigger, EurekaCondition, VictoryCondition, EraTrigger...). For Rust-native mods this is fine, but for Lua:

- Lua can't implement Rust traits directly — a binding layer must bridge them
- Every `Box<dyn Trait>` crossing the boundary needs a wrapper type
- Methods that take `&GameState` are especially problematic since `GameState` is a large, deeply nested struct

The practical path is probably: **use traits for the fixed internal extension points** (things the engine itself needs to query polymorphically, like `RulesEngine` and `VictoryCondition`), and **use data tables for the Lua-facing definitions** (terrain, features, improvements, buildings, etc. as data records with callbacks for the few truly dynamic parts). This is actually how Civ VI's own Lua modding works — most content is defined in XML/SQL data tables, with Lua only for event hooks.

---

## libhexgrid

### HexTile and HexEdge coordinate relationship

**Valid concern.** The current code does not document this clearly. The intended relationship: a `HexEdge` is shared between two adjacent `HexTile`s. Given tile at `{0,0,0}`, the edge in direction `E` is shared with the tile at `{1,0,-1}`. The edge's `from`/`to` endpoints are the two tile coords that share it. This should be documented explicitly in `board.rs` and `edge.rs`.

### Elevation as enum with Low/Level(u8)/High

**Agree.** The current implementation uses `Elevation` as named constants (FLAT=0, HILLS=1, MOUNTAIN=2) on a struct, not the enum described in the architecture spec. The enum form is better because:

- `Low` (below sea level / ocean floor) and `High` (impassable mountain) are true sentinels — they don't participate in the `Level(u8)` arithmetic
- Sea level rise logic becomes `tile.elevation < Level(current_sea_level)` which is clear and type-safe
- The current `u8` constants can't distinguish "this is conceptually a sea tile" from "this is coastal land at elevation 0"

I have no rebuttal here — this should be implemented as specified.

### Vision as enum with Blind/Radius(u8)/Omniscient

**Agree.** Same reasoning as Elevation. `Blind` and `Omniscient` are qualitatively different from `Radius(n)`, not just edge cases of it. The current implementation already uses an enum-like approach (Vision has a variant structure) — this is largely already done or easy to implement.

### MovementProfile hybrid option

**Partially agree.** The Giant Death Robot and similar amphibious units do need a profile that isn't purely ground or naval. However, the current `MovementProfile` enum (Ground, Naval, Air, Embarked) could handle this with an `Amphibious` variant rather than a fully "hybrid" profile. The question is whether hybrid movement cost should be `min(ground_cost, naval_cost)` or something else — that logic needs to be specified before implementing. For now the enum extension is straightforward.

---

## libcivcore

### String errors and `'static` lifetime

**Valid concern.** Several error types use `&'static str` which will break the moment a Lua-sourced string is involved in an error path. These should be converted to owned `String` or a proper error enum before Phase 3. This is low priority for Phase 2 but should not be deferred past Phase 3.

### city.rs — CityStatus should not include Capital

**Agree.** `Capital` is a property of a `City` (a boolean or a separate field on `Civilization` pointing to the capital `CityId`), not a status. A capital can simultaneously be occupied or under siege. Mixing it into the status enum makes `is_capital()` depend on `status == CityStatus::Capital`, which becomes wrong the moment the capital is captured. The fix: remove `Capital` from `CityStatus` and store `capital_city: CityId` on `Civilization`. `CityStatus` then represents the city's current condition: `Normal`, `UnderSiege`, `Occupied`, `Razed`.

### city.rs — Puppet city

A puppet city is a real Civ VI mechanic: a captured city that you choose not to annex (you can't directly manage its production queue — the AI manages it automatically) but still generates yields and counts toward your empire. It's distinct from Occupied (where you've annexed and are managing it but suffer loyalty/amenity penalties) and from a normal city. I'd keep it, but the distinction between Occupied and Puppet should be clarified in code comments.

### city.rs — CityStatus ambiguity

**Agree with the concern.** The current enum variants are underspecified. A city can be both starving and occupied simultaneously. The right model is likely a `CityStatus` representing the political/ownership state (`Normal`, `Occupied`, `Razed`, optionally `Puppet`) plus a separate set of flags or a `Vec<CityCondition>` for transient states (`UnderSiege`, `Starving`, `LowHousing`, `LowAmenities`). These are computed each turn rather than stored.

### city.rs — WallLevel as trait

**Partially agree.** If mods can define new wall types, a trait is appropriate. However, walls in Civ VI are more data-driven than behavior-driven (they provide a defense value and HP modifier). A `WallDef` trait with `defense_bonus() -> i32` and `hp_modifier() -> i32` would work. The `WallLevel` enum could remain as the set of built-in implementations. This is consistent with how `TerrainDef`, `RoadDef`, etc. are handled.

### city.rs — ProductionItem UUIDs

**Agree.** Using string names for `Unit` and `Wonder` in `ProductionItem` is brittle — name collisions across mods, no refactor safety, no O(1) lookup. The architecture spec already requires all extensible definitions to expose an ID (e.g. `UnitTypeId`, `WonderId`). The fix is straightforward: `ProductionItem::Unit(UnitTypeId)` and `ProductionItem::Wonder(WonderId)`.

### city_state.rs — Fold into City

**Conditionally agree.** A city state and a regular city share most structural properties (location, population, yields, districts, walls). The differences are behavioral: city states have a suzerain/envoy system rather than a civ owner, and they don't build settlers or pursue research. Folding them into `City` with a `city_kind: CityKind` discriminant (Regular vs. CityState) is cleaner than a parallel struct. The suzerain/envoy data would be in an `Option<CityStateData>` field. The bonus definitions (suzerain modifiers, envoy thresholds) can remain in a separate system as noted.

On `name` being `String` rather than `&'static str`: **definitely agree**, this applies everywhere — Lua-loaded content can't use static strings.

### civilization.rs — Terrain preferences with TerrainId

**Agree.** Using `String` for terrain type names in `StartBias` creates the same brittleness as string-keyed production items. `StartBias` should reference `TerrainId` and `FeatureId` from `libcommon`.

### civilization.rs — Leader names/descriptions as String

**Agree.** See the general note on `&'static str` vs Lua extensibility. All user-visible names should be `String`.

### civilization.rs — Leader abilities and strategic resources

**Agree on both.** The "less brittle mapping" for leader abilities is addressed by the `Vec<Box<dyn LeaderAbility>>` approach in the architecture spec — the current `civilization.rs` stores `ability_names: Vec<String>` which is a stub. The real implementation should use the trait objects.

On strategic resources: `Civilization` should track consumable resource stockpiles (e.g. `strategic_resources: HashMap<ResourceId, u32>`). This is missing from the current struct.

### diplomacy.rs — Status variants

**Agree.** `War`, `Neutral`, `Friendly`, `Denounced`, `Alliance` are the right states. `ColdWar` is not a Civ VI diplomatic status — it was probably a misinterpretation. `OpenBorders` is correctly an `Agreement`, not a status. The current architecture spec already has the right enum; the implementation just needs to match it.

**On grievance reasons in DiplomaticRelation:** Agree — grievances should be typed structs with a description and a visibility level, not strings. This makes the diplomacy screen implementable (you can show the reason and whether the player can see it based on spy/alliance visibility).

### district.rs — Natural wonders in adjacency context

**Agree.** The `AdjacencyContext` currently doesn't include natural wonder data. In Civ VI, natural wonders provide adjacency bonuses to Holy Sites and certain other districts. `AdjacencyContext` should include `Option<&NaturalWonder>` for adjacent tiles that have one.

### Era.rs — Static strings

**Agree.** Same issue as everywhere else — `String` not `&'static str`.

---

## Summary of changes I'd prioritize before Phase 2

1. **Merge libraries** — keep `libhexgrid` separate, merge the rest into `libcivbase`
2. **Fix `Elevation` and `Vision`** — implement the enum forms as specified in ARCHITECTURE.md (currently diverged)
3. **Remove `Capital` from `CityStatus`** — low-risk, high-clarity improvement
4. **Replace `&'static str` with `String`** for all user-visible names throughout
5. **Add `strategic_resources: HashMap<ResourceId, u32>`** to `Civilization`
6. **Fix `ProductionItem`** to use typed IDs (`UnitTypeId`, `WonderId`) instead of strings

Items 3–6 are small, safe changes. Items 1–2 are more involved but should precede significant Phase 2 work to avoid refactoring under load.
