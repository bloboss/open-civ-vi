# Culture, Tourism & Borders

## Cultural Border Expansion

Cities expand their territory over time through culture accumulation.

### Mechanics

Each city has a `CultureBorder` that tracks accumulated culture points. Every turn:
1. The city earns culture (minimum +1/turn base, plus building/policy/modifier bonuses)
2. Culture accumulates toward the next tile claim threshold
3. When the threshold is reached, the cheapest unclaimed tile in the city's expansion range is claimed

### Expansion Priority

Tiles are evaluated by ring distance from the city center:
- **Ring 1**: claimed immediately on founding (free)
- **Ring 2**: first expansion target (lowest culture cost)
- **Ring 3-5**: progressively more expensive
- **Beyond ring 5**: border expansion stops

Within the same ring, tiles are prioritized by yield potential.

### Tile Claiming

`RulesEngine::claim_tile()` handles territory acquisition:
- **Unclaimed tiles**: claimed normally
- **Own tiles**: idempotent (no-op)
- **Enemy tiles**: rejected unless `force=true` (culture flip from loyalty revolt)
- **Range check**: must be within 3 tiles of the claiming city

`reassign_tile()` moves tiles between cities of the same civilization.

## Tourism

Tourism is the offensive component of the cultural victory. It accumulates each turn and is compared against other civilizations' domestic culture.

### Tourism Sources

| Source | Tourism/Turn |
|--------|-------------|
| World Wonders | varies per wonder |
| Great Works of Writing | 2 |
| Great Works of Art | 3 |
| Great Works of Music | 4 |
| Relics | 4 |
| Artifacts | 3 |

Tourism is computed by `compute_tourism()` which sums all wonder tourism and great work tourism for a civilization.

### Domestic Culture

Each civilization accumulates `lifetime_culture` -- the total culture earned across all turns. This represents the civilization's cultural defense.

`domestic_tourists()` computes the number of domestic tourists as `lifetime_culture / 100`.

### Cultural Dominance

A civilization achieves cultural dominance over another when:

```
tourism_accumulated > opponent.lifetime_culture
```

Cultural dominance must be achieved over ALL other civilizations simultaneously to win a cultural victory.

### Relevant Deltas

- `TourismGenerated { civ, amount }` -- emitted each turn when tourism is produced
- `LifetimeCultureUpdated { civ, total }` -- tracks culture accumulation

## Loyalty

Cities have a loyalty value (0-100) that determines their allegiance.

### Loyalty Pressure

Each turn, cities gain or lose loyalty based on:
- **Own cities nearby**: positive pressure
- **Foreign cities nearby**: negative pressure (weighted by population)
- **Capital bonus**: capitals resist loyalty loss
- **Governor bonus**: governors stabilize loyalty
- **Occupation penalty**: occupied cities lose loyalty faster

Loyalty changes are clamped to +/-20 per turn.

### Revolt

When loyalty reaches 0:
1. The city revolts
2. Ownership transfers to the civilization exerting the most pressure
3. Territory transfers with the city
4. A `CityRevolted` delta is emitted

### Special Cases

- City-states skip loyalty checks entirely
- Capitals have a strong loyalty bonus, making them resistant to flipping
- Governors add a flat loyalty bonus when established in a city
