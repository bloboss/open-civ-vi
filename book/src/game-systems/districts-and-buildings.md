# Districts & Buildings

## Districts

Districts are specialized city zones placed on tiles within a city's territory. Each district type unlocks specific buildings and provides adjacency bonuses.

### Built-in Districts

| District | Base Cost | Tech/Civic Required | Terrain |
|----------|-----------|-------------------|---------|
| Campus | 54 | Writing | Land (not mountain) |
| Theater Square | 54 | Craftsmanship (civic) | Land (not mountain) |
| Commercial Hub | 54 | -- | Land (not mountain) |
| Harbor | 54 | -- | Coast (water only) |
| Holy Site | 54 | Astrology | Land (not mountain) |
| Encampment | 54 | Bronze Working | Land (not mountain) |
| Industrial Zone | 54 | -- | Land (not mountain) |
| Entertainment Complex | 54 | -- | Land (not mountain) |
| Water Park | 54 | -- | Coast (water only) |
| Aqueduct | 54 | -- | Land (not mountain) |
| Dam | 54 | -- | Land (not mountain) |
| Canal | 54 | -- | Land (not mountain) |

### Placement Rules

District placement is validated by `RulesEngine::place_district()`:

1. **Ownership**: tile must be owned by the placing civilization
2. **Range**: tile must be within 3 tiles of the city center
3. **City center**: cannot place on the city center tile
4. **Uniqueness**: only one of each district type per city
5. **Occupancy**: only one district per tile
6. **Terrain**: must match the district's terrain requirements (e.g., Harbor requires Coast)
7. **Tech/Civic**: required tech or civic must be researched
8. **Civ-specific**: unique districts (e.g., Acropolis) have additional constraints (must be on hills)

### Adjacency Bonuses

Districts gain yield bonuses from adjacent features:
- Campus: +1 Science per adjacent Mountain, +0.5 per adjacent Rainforest
- Commercial Hub: +2 Gold per adjacent River
- Holy Site: +1 Faith per adjacent Mountain, +0.5 per adjacent Forest
- Industrial Zone: +1 Production per adjacent Mine, +0.5 per adjacent Quarry
- (And more via the modifier system)

Adjacency is computed via `AdjacencyContext` and applied through the modifier pipeline.

## Buildings

Buildings are constructed within districts (or the city center):

```rust
struct BuildingDef {
    id: BuildingId,
    name: &'static str,
    cost: u32,
    maintenance: i32,
    yields: YieldBundle,
    requires_district: Option<BuiltinDistrict>,
    great_work_slots: Vec<GreatWorkSlotType>,
    exclusive_to: Option<&'static str>,
    replaces: Option<BuildingId>,
}
```

Buildings are produced via the city's production queue (`ProductionItem::Building(BuildingId)`). When completed:
- The building ID is added to the city's `buildings` list
- Yields from the building are included in future `compute_yields()` calls
- Gold maintenance is deducted each turn
- Great work slots (if any) become available for placing great works

### Tech Unlock

Buildings are unlocked by researching their required tech. The `OneShotEffect::UnlockBuilding` effect fires on tech completion, adding the building to the civilization's `unlocked_buildings` list.

### Free Buildings

Some civilization abilities grant free buildings on city founding. For example, Rome's Trajan grants a free Monument (which normally requires a Theater Square district) when founding a city.

## Wonders

World wonders are unique buildings -- only one civilization can build each wonder. They are produced via `ProductionItem::Wonder(WonderId)` and provide powerful bonuses through the modifier system. Wonders also generate tourism for the cultural victory path.
