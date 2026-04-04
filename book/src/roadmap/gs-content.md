# GS-4 through GS-16: Content & Features

> **Reference**: `original-xml/DLC/Expansion2/Data/Expansion2_*.xml`

This document covers all Gathering Storm content additions and gameplay features
that build on existing systems (no new core systems required).

---

## GS-4: Future Era

### New Technologies (~11)

| Tech | Cost | Prerequisites | Unlocks |
|---|---|---|---|
| Seasteads | 2155 | Composites | Seastead improvement |
| Advanced AI | 2155 | Computers | Future era units |
| Advanced Power Cells | 2155 | Nuclear Fusion | Enhanced power |
| Cybernetics | 2500 | Advanced AI | Giant Death Robot |
| Smart Materials | 2500 | Nanotechnology | Advanced buildings |
| Predictive Systems | 2500 | Telecommunications | Enhanced AI |
| Offworld Mission | 2500 | Robotics, Nuclear Fusion | Exoplanet project |
| Future Tech | 2500 | (repeatable) | Score points |

### New Civics (~11)

| Civic | Cost | Prerequisites | Unlocks |
|---|---|---|---|
| Environmentalism | 2415 | Cultural Heritage | Carbon Recapture |
| Corporate Libertarianism | 2880 | Globalization | Government |
| Digital Democracy | 2880 | Social Media | Government |
| Synthetic Technocracy | 2880 | Social Media | Government |
| Information Warfare | 2880 | Social Media | Cyber policies |
| Cultural Hegemony | 3200 | Future Civic prereqs | Cultural bonuses |
| Exodus Imperative | 3200 | Future Civic prereqs | Space bonuses |
| Near Future Governance | 2880 | Globalization | Transition policies |
| Future Civic | 3200 | (repeatable) | Score points |

### New Governments (3)

| Government | Era | Slots (M/E/D/W) | Bonus |
|---|---|---|---|
| Corporate Libertarianism | Future | 2/4/0/2 | +50% tourism from trade |
| Digital Democracy | Future | 1/1/4/2 | +2 favor/turn per suzerainty |
| Synthetic Technocracy | Future | 0/3/1/4 | +100% district adjacency |

### New Units (2)

| Unit | Era | CS | Cost | Type |
|---|---|---|---|---|
| Giant Death Robot | Atomic | 130 | 1500 | Heavy Cavalry |
| Rock Band | Modern | — | 600 | Cultural (faith purchase) |

---

## GS-5: New Civilizations (8)

Each follows the existing `civ_registry.rs` pattern:

### Canada (Leader: Wilfrid Laurier)
- **Ability**: Four Faces of Peace — cannot declare surprise wars; +100% diplomatic favor
- **Unique Unit**: Mountie — cavalry, +5 CS near National Parks
- **Unique Improvement**: Hockey Rink — +1 Amenity, +1 Culture, +2 Appeal

### Hungary (Leader: Matthias Corvinus)
- **Ability**: Pearl of the Danube — +50% production for buildings across rivers
- **Unique Unit**: Huszar — light cavalry, +3 CS per active alliance
- **Unique Building**: Thermal Bath (replaces Zoo) — +2 Amenity, +2 Production, +3 Tourism

### Inca (Leader: Pachacuti)
- **Ability**: Mit'a — citizens can work mountain tiles; +1 food per terrace farm
- **Unique Unit**: Warakaq — medieval ranged, +1 attack per adjacent Warakaq
- **Unique Improvement**: Terrace Farm — +1 food, scales with adjacent mountains

### Mali (Leader: Mansa Musa)
- **Ability**: Songs of the Jeli — -30% production for units/buildings; mines +4 gold
- **Unique Unit**: Mandekalu Cavalry — heavy cavalry, prevents gold loss on nearby kills
- **Unique District**: Suguba (replaces Commercial Hub) — faith/gold purchase discount

### Maori (Leader: Kupe)
- **Ability**: Mana — starts in ocean; unimproved features +2 production/culture
- **Unique Unit**: Toa — melee, adjacent enemies -5 CS
- **Unique Building**: Marae (replaces Amphitheater) — +2 culture/faith per feature

### Ottoman (Leader: Suleiman)
- **Ability**: Great Turkish Bombard — +50% production for siege, conquered cities no loyalty loss
- **Unique Unit**: Barbary Corsair — naval raider, coastal raid for gold
- **Unique Building**: Grand Bazaar (replaces Bank) — extra strategic/luxury resources

### Phoenicia (Leader: Dido)
- **Ability**: Mediterranean Colonies — 100% loyalty in cities on same continent as capital
- **Unique Unit**: Bireme — ancient naval, protects nearby traders
- **Unique District**: Cothon (replaces Harbor) — +50% naval unit production, move capital project

### Sweden (Leader: Kristina)
- **Ability**: Nobel Prize — +50 diplomatic favor on great person recruitment
- **Unique Unit**: Carolean — anti-cavalry, +3 CS per unused movement
- **Unique Building**: Open-Air Museum — +2 culture/tourism per terrain type in city

---

## GS-6: New City-States (9)

| City-State | Type | Suzerain Bonus |
|---|---|---|
| Akkad | Militaristic | Melee units +5 CS |
| Bologna | Scientific | +1 great person point per district |
| Cahokia | Trade | Free unique Cahokia Mounds improvement |
| Cardiff | Industrial | +2 power from harbor buildings |
| Fez | Scientific | +20 science on converting a city |
| Mexico City | Industrial | +2 production from improvements |
| Nazca | Religious | Free Nazca Line improvement |
| Ngazargamu | Militaristic | +20% combat XP for mounted units |
| Rapa Nui | Cultural | Free Moai improvement |

---

## GS-7: New Units (~13)

### Generic units (5)

| Unit | Era | CS | Cost | Class | Notes |
|---|---|---|---|---|---|
| Skirmisher | Medieval | 30R | 150 | Ranged | New ranged tier |
| Courser | Medieval | 44 | 200 | Light Cavalry | New cavalry tier |
| Cuirassier | Renaissance | 55 | 330 | Heavy Cavalry | Niter cost |
| Rock Band | Modern | — | 600 | Cultural | Faith purchase only |
| Giant Death Robot | Atomic | 130 | 1500 | Heavy Cavalry | Superweapon |

### Civ-unique units (8)

| Unit | Civ | Replaces | CS |
|---|---|---|---|
| Mountie | Canada | — | 60 |
| Huszar | Hungary | — | 55 |
| Warakaq | Inca | — | 40R |
| Mandekalu Cavalry | Mali | Knight | 55 |
| Toa | Maori | Swordsman | 36 |
| Barbary Corsair | Ottoman | Privateer | 44R |
| Bireme | Phoenicia | Galley | 35 |
| Carolean | Sweden | Pike and Shot | 55 |

---

## GS-8: New Buildings (~20)

### Power plants (7) — requires GS-1

| Building | District | Cost | Power | CO2 |
|---|---|---|---|---|
| Coal Power Plant | Industrial Zone | 580 | 4 | 1 |
| Oil Power Plant | Industrial Zone | 580 | 4 | 1 |
| Nuclear Power Plant | Industrial Zone | 580 | 16 | 0 |
| Hydroelectric Dam | Dam | 580 | 6 | 0 |
| Solar Farm | (improvement) | — | 2 | 0 |
| Wind Farm | (improvement) | — | 2 | 0 |
| Geothermal Plant | (improvement) | — | 4 | 0 |

### Other buildings

| Building | District | Cost | Notes |
|---|---|---|---|
| Flood Barrier | City Center | 400 | Prevents sea level submersion |
| Food Market | City Center | 290 | +4 food |
| Shopping Mall | City Center | 390 | +3 gold |
| Aquatics Center | Water Park | 445 | +2 amenity, regional |

### Civ-unique buildings (4)

| Building | Civ | Replaces | Notes |
|---|---|---|---|
| Thermal Bath | Hungary | Zoo | +2 amenity, +2 production |
| Grand Bazaar | Ottoman | Bank | Extra resources |
| Open-Air Museum | Sweden | — | +2 culture/tourism |
| Marae | Maori | Amphitheater | +2 culture/faith per feature |

---

## GS-9: New Improvements (~10)

| Improvement | Prereq | Yields | Notes |
|---|---|---|---|
| Solar Farm | — | +2 power | Renewable energy |
| Wind Farm | — | +2 power | Renewable energy |
| Offshore Wind Farm | — | +2 power | Sea tile |
| Geothermal Plant | — | +4 power | Geothermal fissure |
| Seastead | Seasteads tech | +2 food, +1 housing | Ocean tile |
| Mountain Tunnel | — | (movement) | Allows crossing mountains |
| Ski Resort | — | +1 tourism | Snow/tundra |
| Terrace Farm | — | +1 food | Inca unique, mountain adj |
| Hockey Rink | — | +1 amenity, +1 culture | Canada unique |
| Pa | — | +2 defense, +1 culture | Maori unique |

---

## GS-10: New Natural Wonders (7)

| Wonder | Tiles | Yields | Notes |
|---|---|---|---|
| Chocolate Hills | 3 | +1 food, +2 science | Philippines |
| Devil's Tower | 1 | +2 faith, +2 science | Wyoming |
| Gobustan | 2 | +3 culture, +1 production | Azerbaijan |
| Ik-Kil | 1 | +5 faith, +1 science | Mexico cenote |
| Pamukkale | 2 | +4 amenity, +2 science | Turkey |
| White Desert | 2 | +3 gold, +3 culture | Egypt |
| Vesuvius | 2 | +1 production (volcanic) | Italy, can erupt |

---

## GS-11: Tech & Civic Extensions

See GS-4 above for the full list. Implementation: add nodes to
`tech_tree_def.rs` and `civic_tree_def.rs` using the existing pattern.

---

## GS-12: New Policies (~16)

### Future-era policies
- Future Victory (Science/Culture/Domination/Diplomatic variants)
- Future Counter (corresponding counter-policies)

### Additional policies
| Policy | Slot | Effect |
|---|---|---|
| Equestrian Orders | Military | +50% cavalry production |
| Drill Manuals | Military | +25% unit XP |
| Retinues | Military | -50% upgrade cost |
| Force Modernization | Military | +100% modern unit production |
| Cyber Warfare | Military | +3 spy levels |
| Music Censorship | Economic | +25% tourism from music |
| Flower Power | Wildcard | +4 culture per park |

---

## GS-13: City Projects

New production queue item type. Projects are city-scoped, don't produce a
unit/building, and have special completion effects.

### Space Race Projects
| Project | District | Cost | Effect |
|---|---|---|---|
| Launch Mars Base | Spaceport | 1800 | Science victory milestone |
| Exoplanet Expedition | Spaceport | 2400 | 4th science victory milestone |
| Orbital Laser | Spaceport | 1200 | Military project |
| Terrestrial Laser | Spaceport | 1200 | Military project |

### Climate Projects
| Project | District | Cost | Effect |
|---|---|---|---|
| Carbon Recapture | Industrial Zone | 400 | Remove CO2 from atmosphere |
| Decommission Coal/Oil Plant | Industrial Zone | 200 | Remove fossil fuel plant |

### Competition Projects
| Project | Any City | Cost | Effect |
|---|---|---|---|
| Send Aid | — | 400 | Contribute to emergency response |
| Train Athletes | — | 400 | World Games competition |
| Train Astronauts | — | 400 | Space Station competition |

### Implementation
- Add `ProductionItem::Project(ProjectId)` variant
- Create `ProjectDef` struct with cost, district requirement, completion effect
- Register projects in `GameState`
- Handle completion in `advance_turn` production phase

---

## GS-14: Revised Diplomatic Victory

Replace `BuiltinVictoryCondition::Diplomatic { threshold: 100 }` with
VP-based system from World Congress (GS-3).

**Blocked by**: GS-3

---

## GS-15: Revised Science Victory

Add 4th milestone: Exoplanet Expedition (requires Offworld Mission tech +
Spaceport project).

**Blocked by**: GS-11 (Offworld Mission tech) + GS-13 (project system)

---

## GS-16: Rock Band / Cultural Combat

New unit type that targets foreign cities with tourism instead of military
damage. Purchases with Faith, gains fans (XP), earns promotions, risks
disbanding after each performance.

### Mechanics
- Rock Band "attacks" a city district → gains tourism output
- Higher-level districts = more tourism
- After each performance, roll for disband (30% base, modified by promotions)
- XP gained per performance, promotions reduce disband chance
- 4 promotion types: Album Cover Art, Indie, Space Rock, Religious Rock

**Blocked by**: GS-7 (Rock Band unit definition)
