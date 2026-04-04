# P2–P7 — Missing Content

This document lists every concrete game-content item present in the Civ VI base
game XML but absent from the implementation. Items are grouped by category and
annotated with the XML values needed for implementation.

---

## Resources

### Missing Bonus Resources (2)

| Resource | Yield (XML) | Valid Terrain | Improvement |
|---|---|---|---|
| Bananas | +1 Food | Rainforest | Plantation |
| Crabs | +2 Gold | Coast | Fishing Boats |

> Maize, Deer (in some lists) — Deer is already implemented. Maize is not in
> base game XML (added in later DLC).

### Missing Luxury Resources (16)

| Resource | Yield (XML) | Typical Terrain | Improvement |
|---|---|---|---|
| Citrus | +2 Food | Grassland, Plains | Plantation |
| Cocoa | +3 Gold | Rainforest | Plantation |
| Coffee | +1 Culture | Grassland, Plains | Plantation |
| Diamonds | +3 Gold | Rainforest, Grassland | Mine |
| Dyes | +1 Faith | Rainforest, Forest | Plantation |
| Furs | +1 Food, +1 Gold | Tundra, Forest | Camp |
| Gypsum | +1 Gold, +1 Production | Plains, Desert | Quarry |
| Jade | +1 Culture | Grassland, Plains | Mine |
| Marble | +1 Culture | Grassland, Plains, Hills | Quarry |
| Mercury | +1 Science | Plains | Mine |
| Pearls | +1 Faith | Coast | Fishing Boats |
| Silver | +3 Gold | Desert, Tundra | Mine |
| Tea | +1 Science | Grassland | Plantation |
| Tobacco | +1 Faith | Grassland, Plains | Plantation |
| Truffles | +3 Gold | Forest, Rainforest | Camp |
| Whales | +1 Gold, +1 Production | Coast | Fishing Boats |

### Implementation notes (Resources)

- Add variants to `BuiltinResource` enum in `resource.rs`
- Implement `base_yields()`, `resource_category()`, `reveal_tech()` for each
- Update mapgen `place_resources()` to include new resources in spawn tables
- Luxury resources provide +1 Amenity when improved (existing mechanic)

---

## Natural Wonders

### Currently implemented (5 — but 3 are non-base)

| Wonder | In Base XML? | Notes |
|---|---|---|
| Krakatoa | **No** (DLC) | Tag as DLC or replace |
| Grand Mesa | **No** (DLC) | Tag as DLC or replace |
| Uluru | **No** (DLC) | Tag as DLC or replace |
| Cliffs of Dover | Yes | Fix yields (see P0) |
| Galapagos Islands | Yes | Fix yield model (see P0) |

### Missing Base-Game Natural Wonders (10)

All data from `Features.xml` natural wonder entries.

| Wonder | Tiles | Yields (on tile) | Adjacent Yields | Passable? | Fresh Water? |
|---|---|---|---|---|---|
| Great Barrier Reef | 2 | +3 Food, +2 Science | Campus adj: +2 Science | Yes | No |
| Crater Lake | 1 | +5 Faith, +1 Science | — | No (lake) | Yes |
| Dead Sea | 2 | +2 Culture, +2 Faith | — | No (lake) | No |
| Mount Everest | 3 | — | +1 Faith to adjacent tiles | No | No |
| Mount Kilimanjaro | 1 | — | +2 Food to adjacent tiles | No | No |
| Pantanal | 4 | +2 Culture, +2 Food | — | Yes (marsh-like) | No |
| Piopiotahi (Milford Sound) | 3 | — | +1 Culture, +1 Gold to adjacent | No | No |
| Torres del Paine | 2 | — | Doubles adjacent terrain yields | No | No |
| Tsingy de Bemaraha | 1 | — | +1 Culture, +1 Science to adjacent | No | No |
| Yosemite | 2 | — | +1 Gold, +1 Science to adjacent | No | No |

### Implementation notes (Natural Wonders)

- Each wonder is a struct implementing `NaturalWonder` trait in `wonder.rs`
- Multi-tile wonders need coordinate group handling in mapgen
- Adjacent-yield wonders need a `adjacent_yield_bonus()` method (new pattern)
- Torres del Paine's "double adjacent terrain yields" is a unique modifier

---

## Improvements

### Missing Standard Improvements (3)

| Improvement | Prereq Tech | Base Yield | Notes |
|---|---|---|---|
| Oil Well | Steel | +2 Production | Land-only; for Oil resource |
| Offshore Oil Rig | Plastics | +2 Production | Sea-only; for Oil resource |
| Beach Resort | Radio | Gold from Appeal | Tourism source; min appeal 4 |

### Missing Unique Improvements (7)

| Improvement | Civilization | Prereq | Yields | Special |
|---|---|---|---|---|
| Chateau | France | Humanism civic | +2 Culture, +1 Gold | Must be adjacent to bonus/luxury; +2 Gold near river |
| Colossal Head | La Venta (CS) | — | +2 Faith | +1 Faith adj forest/jungle |
| Great Wall | China | Masonry | +1 Gold (scaling) | Must build in line; frontier only; defense +4 |
| Kurgan | Scythia | Animal Husbandry | +3 Gold, +1 Faith | +1 Faith adj pasture |
| Mission | Spain | Education | +2 Faith | +2 Science adj campus; bonuses on other continents |
| Roman Fort | Rome | — | Defense +4 | Built by Legion unit only |
| Ziggurat | Sumeria | — | +2 Science | +1 Culture adj river |

### Implementation notes (Improvements)

- Standard improvements: add to `BuiltinImprovement` enum
- Unique improvements: add with `exclusive_to` field linking to civilization trait
- Beach Resort requires the Appeal system to be fully wired
- Roman Fort requires the Legion's `CanBuildFort` ability (already exists)

---

## Districts

### Missing Standard Districts (4)

| District | Cost | Prereq | Notes |
|---|---|---|---|
| Aerodrome | 54 | Flight tech | Air unit slots |
| Neighborhood | 54 | Urbanization civic | Housing from appeal |
| Spaceport | 1800 | Rocketry tech | Science Victory projects |
| City Center | 0 | — | Implicit in current impl; XML defines it explicitly |

### Missing Unique Districts (6)

| District | Civilization | Replaces | Cost | Prereq |
|---|---|---|---|---|
| Lavra | Russia | Holy Site | 27 | Astrology |
| Mbanza | Kongo | Neighborhood | 27 | Guilds civic |
| Street Carnival | Brazil | Ent. Complex | 27 | Games & Rec. civic |
| Royal Navy Dockyard | England | Harbor | 27 | Celestial Navigation |
| *(Acropolis)* | Greece | Theater Sq. | 27 | Drama & Poetry civic |
| *(Hansa)* | Germany | Industrial Zone | 27 | Apprenticeship |
| *(Bath)* | Rome | Aqueduct | 18 | Engineering |

> Acropolis, Hansa, and Bath are already implemented in `civ_registry.rs` as
> unique districts. They are listed here for completeness but need no new work.

### District Prereq Fixes (deferred from P0 — requires P1 techs)

| District | Current Prereq | Correct Prereq |
|---|---|---|
| Harbor | Sailing | Celestial Navigation (Classical tech) |
| Aqueduct | Masonry | Engineering (Classical tech) |
| Entertainment Complex | Early Empire | Games & Recreation (Classical civic) |

### Non-Base Districts to Tag

These are in the implementation but NOT in base game XML:

| District | DLC |
|---|---|
| WaterPark | Gathering Storm |
| Dam | Gathering Storm |
| Canal | Gathering Storm |

---

## Buildings

### Missing Buildings by District (~60 total)

#### City Center Buildings (8 in XML, ~4 implemented)

| Building | Cost | Prereq | Yields | Implemented? |
|---|---|---|---|---|
| Monument | 60 | — | +2 Culture | Partial (referenced) |
| Palace | 1 | — | +5 Gold, +2 Culture, +2 Science, +2 Production, +1 Housing, +1 Amenity | No |
| Granary | 65 | Pottery | +1 Food, +2 Housing | Yes |
| Walls | 80 | Masonry | +100 Outer Defense | Yes |
| Water Mill | 80 | The Wheel | +1 Food, +1 Production | Referenced |
| Castle (Medieval Walls) | 225 | Castles tech | +100 Outer Defense | No |
| Star Fort (Renaissance Walls) | 305 | Siege Tactics | +100 Outer Defense | No |
| Sewer | 200 | Sanitation | +2 Housing | No |

#### Campus Buildings (3)

| Building | Cost | Prereq Tech | Yields |
|---|---|---|---|
| Library | 90 | Writing | +2 Science, +1 Citizen Slot | Yes |
| University | 250 | Education | +4 Science, +1 Housing | No |
| Research Lab | 580 | Chemistry | +5 Science | No |

#### Holy Site Buildings (3 + 9 worship)

| Building | Cost | Prereq | Yields |
|---|---|---|---|
| Shrine | 70 | Astrology | +2 Faith, +1 Citizen Slot | Yes |
| Temple | 120 | Theology civic | +4 Faith, +1 Citizen Slot, +1 Relic Slot | No |
| *(9 Worship buildings)* | 190 each | Religion belief | Various faith/culture/science | No |

#### Commercial Hub Buildings (3)

| Building | Cost | Prereq Tech | Yields |
|---|---|---|---|
| Market | 120 | Currency | +3 Gold, +1 Citizen Slot | No |
| Bank | 290 | Banking | +5 Gold, +1 Citizen Slot | No |
| Stock Exchange | 390 | Economics | +7 Gold, +1 Citizen Slot | No |

#### Harbor Buildings (3)

| Building | Cost | Prereq Tech | Yields |
|---|---|---|---|
| Lighthouse | 120 | Celestial Navigation | +1 Food, +1 Gold, +1 Housing, +1 Citizen Slot | No |
| Shipyard | 290 | Mass Production | +Production = adjacency bonus | No |
| Seaport | 580 | Electricity | +2 Gold, +1 Food, +1 Housing | No |

#### Encampment Buildings (4)

| Building | Cost | Prereq Tech | Effect |
|---|---|---|---|
| Barracks | 90 | Bronze Working | +25 XP for melee/ranged, +1 Housing | No |
| Stable | 120 | Horseback Riding | +25 XP for cavalry, +1 Housing | No |
| Armory | 195 | Military Engineering | +25 XP for all land, +1 Citizen Slot | No |
| Military Academy | 390 | Military Science | +25 XP for all land, +1 Housing | No |

#### Industrial Zone Buildings (3)

| Building | Cost | Prereq Tech | Yields |
|---|---|---|---|
| Workshop | 195 | Apprenticeship | +2 Production, +1 Citizen Slot | No |
| Factory | 390 | Industrialization | +3 Production (regional 6-tile) | No |
| Power Plant | 580 | Electricity | +4 Production (regional 6-tile) | No |

#### Entertainment Complex Buildings (3)

| Building | Cost | Prereq | Yields |
|---|---|---|---|
| Arena | 150 | Games & Rec. civic | +2 Amenities | No |
| Zoo | 445 | Natural History civic | +1 Amenity (regional) | No |
| Stadium | 660 | Prof. Sports civic | +2 Amenities (regional) | No |

#### Theater Square Buildings (3)

| Building | Cost | Prereq | Yields |
|---|---|---|---|
| Amphitheater | 150 | Drama & Poetry civic | +2 Culture, +1 Writing Slot | No |
| Art/Artifact Museum | 290 | Humanism civic | +1 Culture, +Great Work Slots | No |
| Broadcast Center | 580 | Radio tech | +1 Culture, +1 Music Slot | No |

#### Aerodrome Buildings (2)

| Building | Cost | Prereq Tech | Effect |
|---|---|---|---|
| Hangar | 465 | Flight | +2 Air Slots | No |
| Airport | 600 | Advanced Flight | +2 Air Slots, +1 Tourism/turn | No |

---

## World Wonders

### All 29 Base-Game Wonders (0 implemented)

#### Ancient Era (3)

| Wonder | Cost | Prereq | Key Effect |
|---|---|---|---|
| Stonehenge | 180 | Astrology | Free Great Prophet |
| Hanging Gardens | 180 | Irrigation | +15% growth in city, +2 Housing |
| Pyramids | 220 | Masonry | Builders get +1 charge |

#### Classical Era (7)

| Wonder | Cost | Prereq | Key Effect |
|---|---|---|---|
| Oracle | 290 | Mysticism civic | +2 Great Person points/turn |
| Great Lighthouse | 290 | Celestial Navigation | +1 Movement for naval, +1 Admiral point |
| Colossus | 400 | Shipbuilding | +1 Trade Route capacity, +3 Gold |
| Petra | 400 | Mathematics | +2 Food, +2 Gold, +1 Production on desert tiles |
| Colosseum | 400 | Games & Rec. civic | +3 Amenities to all cities within 6 tiles |
| Great Library | 400 | Recorded History civic | +2 Science, random tech boost |
| Mahabodhi Temple | 400 | Theology civic | +4 Faith, 2 free Apostles |
| Terracotta Army | 400 | Construction | +1 Promotion for all land units |

#### Medieval Era (4)

| Wonder | Cost | Prereq | Key Effect |
|---|---|---|---|
| Hagia Sophia | 710 | Education | +4 Faith, Missionaries +1 spread charge |
| Alhambra | 710 | Castles | +1 Military policy slot, +2 Amenities |
| Chichen Itza | 710 | Guilds civic | +2 Culture, +1 Production to Rainforest tiles |
| Mont St. Michel | 710 | Divine Right civic | +2 Faith, +2 Relic slots |

#### Renaissance Era (4)

| Wonder | Cost | Prereq | Key Effect |
|---|---|---|---|
| Venetian Arsenal | 920 | Mass Production | Double naval unit production |
| Great Zimbabwe | 920 | Banking | +5 Gold, +1 Trade Route, +2 Gold/bonus resource |
| Forbidden City | 920 | Printing | +1 Wildcard policy slot |
| Potala Palace | 1060 | Astronomy | +1 Diplomatic policy slot, +3 Culture |

#### Industrial Era (3)

| Wonder | Cost | Prereq | Key Effect |
|---|---|---|---|
| Ruhr Valley | 1240 | Industrialization | +30% Production, +1 Prod/mine & quarry |
| Bolshoi Theatre | 1240 | Opera & Ballet civic | +2 Great Writing & Music slots |
| Oxford University | 1240 | Scientific Theory | +20% Science, 2 free techs |

#### Modern Era (5)

| Wonder | Cost | Prereq | Key Effect |
|---|---|---|---|
| Big Ben | 1450 | Economics | Double treasury, +1 Economic policy slot |
| Hermitage | 1450 | Natural History civic | +3 Art slots |
| Eiffel Tower | 1620 | Steel | +2 Appeal to all tiles |
| Broadway | 1620 | Mass Media civic | +3 Writing/Music slots |
| Cristo Redentor | 1620 | Mass Media civic | Tourism from seaside resorts not reduced |

#### Atomic/Information Era (3)

| Wonder | Cost | Prereq | Key Effect |
|---|---|---|---|
| Estádio do Maracanã | 1740 | Prof. Sports civic | +6 Amenities to all cities |
| Sydney Opera House | 1850 | Cultural Heritage civic | +8 Culture, +5 Great Musician points |

### Implementation notes (World Wonders)

- The `WonderDef` struct already exists in `game/state.rs`
- Each wonder needs: cost, prereq tech/civic, placement rules, effects
- Effects use the existing `Effect` enum (may need new variants for some wonders)
- Wonders that grant policy slots need `ExtraPolicySlot` effect
- Wonders that boost all cities need regional-effect infrastructure

---

## Units

### Missing Generic Units by Era (~72)

#### Ancient Era (missing 2)

| Unit | Combat | Ranged | Cost | Movement | Prereq | Class |
|---|---|---|---|---|---|---|
| Scout | 10 | — | 30 | 3 | — | Recon |
| Galley | 30 | — | 65 | 3 | Sailing | Naval Melee |

> Warrior (20/40), Slinger (5+15R/35), Settler (80), Builder (50), Trader (40)
> are already implemented.

#### Classical Era (6)

| Unit | Combat | Ranged | Cost | Movement | Prereq | Class |
|---|---|---|---|---|---|---|
| Archer | 15 | 25 | 60 | 2 | Archery | Ranged |
| Spearman | 25 | — | 65 | 2 | Bronze Working | Anti-Cavalry |
| Heavy Chariot | 28 | — | 65 | 2 | The Wheel | Heavy Cavalry |
| Horseman | 36 | — | 80 | 4 | Horseback Riding | Light Cavalry |
| Catapult | 25 | — | 120 | 2 | Mathematics | Siege |
| Quadrireme | 20 | 25 | 120 | 3 | Shipbuilding | Naval Ranged |

#### Medieval Era (8)

| Unit | Combat | Ranged | Cost | Movement | Prereq | Class |
|---|---|---|---|---|---|---|
| Swordsman | 35 | — | 90 | 2 | Iron Working | Melee |
| Man-at-Arms | 45 | — | 160 | 2 | Apprenticeship | Melee |
| Crossbowman | 30 | 40 | 180 | 2 | Machinery | Ranged |
| Pikeman | 45 | — | 200 | 2 | Military Tactics | Anti-Cavalry |
| Knight | 50 | — | 180 | 4 | Stirrups | Heavy Cavalry |
| Trebuchet | 35 | — | 200 | 2 | Military Engineering | Siege |
| Warrior Monk | 40 | — | 100 | 3 | (Religion) | Monk |

#### Renaissance Era (6)

| Unit | Combat | Ranged | Cost | Movement | Prereq | Class |
|---|---|---|---|---|---|---|
| Musketman | 55 | — | 240 | 2 | Gunpowder | Melee |
| Bombard | 45 | — | 280 | 2 | Metal Casting | Siege |
| Caravel | 55 | — | 240 | 4 | Cartography | Naval Melee |
| Frigate | 45 | 55 | 280 | 4 | Square Rigging | Naval Ranged |
| Privateer | 40 | 50 | 280 | 4 | Square Rigging | Naval Raider |

#### Industrial Era (8)

| Unit | Combat | Ranged | Cost | Movement | Prereq | Class |
|---|---|---|---|---|---|---|
| Line Infantry | 65 | — | 360 | 2 | Rifling | Melee |
| Cavalry | 62 | — | 330 | 5 | Military Science | Light Cavalry |
| Field Cannon | 50 | 60 | 330 | 2 | Ballistics | Siege |
| Ranger | 45 | 60 | 380 | 3 | Rifling | Recon |
| Ironclad | 70 | — | 380 | 5 | Steam Power | Naval Melee |
| AT Crew | 75 | — | 400 | 2 | Rifling | Anti-Cavalry |

#### Modern Era (10)

| Unit | Combat | Ranged | Cost | Movement | Prereq | Class |
|---|---|---|---|---|---|---|
| Infantry | 75 | — | 430 | 2 | Replaceable Parts | Melee |
| Artillery | 60 | — | 430 | 2 | Steel | Siege |
| Machine Gun | 70 | 85 | 540 | 2 | Advanced Ballistics | Ranged |
| Tank | 85 | — | 480 | 4 | Combustion | Heavy Cavalry |
| Biplane | 80 | 75 | 430 | 6 | Flight | Air Fighter |
| Bomber | 85 | — | 560 | 10 | Advanced Flight | Air Bomber |
| Battleship | 60 | 70 | 430 | 5 | Steel | Naval Ranged |
| Submarine | 65 | 75 | 480 | 3 | Electricity | Naval Raider |
| Destroyer | 85 | — | 540 | 4 | Combined Arms | Naval Melee |
| Aircraft Carrier | 65 | — | 540 | 3 | — | Naval Carrier |

#### Atomic Era (6)

| Unit | Combat | Ranged | Cost | Movement | Prereq | Class |
|---|---|---|---|---|---|---|
| Mechanized Infantry | 85 | — | 650 | 3 | Combined Arms | Melee |
| Modern Armor | 95 | — | 680 | 4 | Combined Arms | Heavy Cavalry |
| Modern AT | 85 | — | 580 | 3 | Composites | Anti-Cavalry |
| Helicopter | 86 | — | 600 | 4 | Synthetic Materials | Light Cavalry |
| Rocket Artillery | 70 | — | 680 | 3 | Guidance Systems | Siege |
| Nuclear Submarine | 80 | 85 | 680 | 4 | Telecommunications | Naval Raider |
| Fighter | 100 | 100 | 520 | 8 | Advanced Flight | Air Fighter |

#### Information Era (3)

| Unit | Combat | Ranged | Cost | Movement | Prereq | Class |
|---|---|---|---|---|---|---|
| Jet Fighter | 110 | 110 | 650 | 10 | Lasers | Air Fighter |
| Jet Bomber | 90 | — | 700 | 15 | Stealth Technology | Air Bomber |
| Missile Cruiser | 75 | 90 | 680 | 5 | Lasers | Naval Ranged |

#### Support Units (7)

| Unit | Cost | Movement | Prereq | Effect |
|---|---|---|---|---|
| Battering Ram | 65 | 2 | — | Anti-wall melee support |
| Siege Tower | 100 | 2 | Construction | Bypass walls |
| Military Engineer | 170 | 2 | Military Engineering | Build forts/roads |
| Observation Balloon | 240 | 2 | Flight | +1 Range for siege |
| Medic | 370 | 2 | — | Heal adjacent |
| Anti-Air Gun | 455 | 2 | Steel | Anti-air |
| Mobile SAM | 590 | 3 | Guidance Systems | Anti-air |

### Missing Unique Units (13)

| Unit | Civilization | Replaces | Combat | Cost | Special |
|---|---|---|---|---|---|
| War-Cart | Sumeria | — | 30 | 55 | No penalties vs anti-cavalry |
| Crouching Tiger | China | — | 30R:50 | 140 | Short-range high-damage ranged |
| Berserker | Norway | Swordsman | 48 | 160 | +7 attack, -7 defense |
| Ngao Mbeba | Kongo | Swordsman | 38 | 110 | Immune to forest/jungle penalty |
| Cossack | Russia | Cavalry | 67 | 340 | +5 CS in own territory |
| Redcoat | England | Line Infantry | 70 | 360 | +10 CS on other continents |
| Garde Impériale | France | Line Infantry | 70 | 360 | +10 CS near capital |
| Conquistador | Spain | Musketman | 58 | 250 | +10 CS adj. apostle/missionary |
| Rough Rider | America | Cavalry | 67 | 385 | +5 CS on hills |
| P-51 Mustang | America | Fighter | 105R:105 | 520 | +5 CS, +2 range |
| Sea Dog | England | Privateer | 40R:55 | 280 | Can capture enemy ships |
| Minas Geraes | Brazil | Battleship | 70R:80 | 430 | +10 CS |
| Scythian Horse Archer | Scythia | — | 20R:25 | 100 | Mobile ranged cavalry |

### Implementation notes (Units)

- `UnitTypeDef` struct exists; add entries to `unit_type_defs` registry
- Each unit needs: id, name, cost, combat, ranged_combat, movement, domain,
  category, era, prereq_tech, resource_cost, replaces, exclusive_to
- Support units need a new `FormationClass::Support` or `UnitCategory::Support`
- Air units need `Domain::Air` (may not exist yet)
