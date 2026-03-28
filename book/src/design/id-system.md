# ID System

All entities in the game are identified by strongly-typed, ULID-backed IDs.

## The define_id! Macro

```rust
define_id!(CityId);
define_id!(UnitId);
define_id!(CivId);
// ... 22 ID types total
```

Each invocation generates a newtype wrapper around `ulid::Ulid` with:
- `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`
- `from_ulid(Ulid)` -- construct from raw ULID
- `nil()` -- sentinel/placeholder value
- `as_ulid()` -- extract the inner ULID
- `Display` -- formats as ULID string

## All ID Types

| ID Type | Entity |
|---------|--------|
| `CivId` | Civilization |
| `CityId` | City |
| `UnitId` | Unit instance |
| `UnitTypeId` | Unit type definition |
| `BuildingId` | Building instance |
| `DistrictTypeId` | District type |
| `TechId` | Technology |
| `CivicId` | Civic |
| `PolicyId` | Policy card |
| `GovernmentId` | Government type |
| `WonderId` | World wonder |
| `ReligionId` | Religion |
| `BeliefId` | Religious belief |
| `GreatPersonId` | Great person |
| `GreatWorkId` | Great work |
| `TradeRouteId` | Trade route |
| `AgreementId` | Diplomatic agreement |
| `GrievanceId` | Diplomatic grievance |
| `GovernorId` | Governor |
| `EraId` | Historical era |
| `VictoryId` | Victory condition |
| `TerrainId`, `FeatureId`, `EdgeFeatureId`, `ImprovementId`, `ResourceId`, `RoadId`, `NaturalWonderId` | World content |

## IdGenerator

```rust
struct IdGenerator { /* seeded RNG */ }
```

Methods:
- `new(seed: u64)` -- deterministic generator from seed
- `next_ulid()` -- raw ULID
- `next_city_id()`, `next_unit_id()`, `next_civ_id()`, etc. -- typed ID generation

The generator uses seeded RNG to ensure deterministic ID generation across runs with the same seed. This is critical for replay and testing.

## Wire Protocol IDs

The `open4x-api` crate mirrors all ID types with its own `define_api_id!` macro. API IDs are serializable (`Serialize`/`Deserialize`) and use the same ULID representation, allowing zero-cost conversion between internal and wire formats.

## Design Rationale

- **Type safety**: `CityId` and `UnitId` are distinct types -- you cannot accidentally pass one where the other is expected
- **No String IDs**: ULIDs are compact, sortable, and globally unique without coordination
- **Deterministic**: seeded generation means the same game produces the same IDs, enabling exact replay
- **Nil sentinel**: `Id::nil()` provides a safe placeholder without `Option` overhead where appropriate
