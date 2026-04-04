# P8–P12 — System-Level Gaps

This document covers gameplay systems where the implementation has the framework
but is missing most or all of the concrete data definitions: governments,
policies, promotions, city-states, civilizations, and great people.

---

## Governments

### Currently implemented

Only **Chiefdom** and **Autocracy** are referenced (unlocked by Code of Laws and
Early Empire civics respectively). No full government definitions exist.

### All 10 Base-Game Governments (from Governments.xml)

| Government | Era | Prereq Civic | Military | Economic | Diplomatic | Wildcard | Total |
|---|---|---|---|---|---|---|---|
| Chiefdom | Ancient | Code of Laws | 1 | 1 | 0 | 0 | 2 |
| Autocracy | Classical | Political Philosophy | 1 | 1 | 1 | 1 | 4 |
| Oligarchy | Classical | Political Philosophy | 2 | 1 | 0 | 1 | 4 |
| Classical Republic | Classical | Political Philosophy | 0 | 2 | 1 | 1 | 4 |
| Monarchy | Medieval | Divine Right | 2 | 1 | 1 | 2 | 6 |
| Theocracy | Medieval | Reformed Church | 2 | 2 | 1 | 1 | 6 |
| Merchant Republic | Medieval | Exploration | 1 | 2 | 2 | 1 | 6 |
| Fascism | Modern | Totalitarianism | 4 | 1 | 1 | 2 | 8 |
| Communism | Modern | Class Struggle | 3 | 3 | 1 | 1 | 8 |
| Democracy | Modern | Suffrage | 1 | 3 | 2 | 2 | 8 |

### Government inherent bonuses (from XML modifiers)

| Government | Bonus |
|---|---|
| Chiefdom | — |
| Autocracy | +1 Capital yield to all yields |
| Oligarchy | +4 Combat Strength for melee/anti-cavalry |
| Classical Republic | +1 Amenity per district with buildings |
| Monarchy | +50% influence per turn |
| Theocracy | +5 Religious Strength, can buy land units with Faith |
| Merchant Republic | +2 Trade Route capacity |
| Fascism | +5 Combat Strength to all units |
| Communism | +0.6 Production per citizen |
| Democracy | +2 Gold, +2 Production per trade route to allies |

### Implementation notes (Governments)

- The `AdoptGovernment` action and policy slot system already exist
- Need concrete `Government` definitions with slot counts and bonuses
- Bonuses use the existing modifier pipeline
- Legacy bonuses (accumulated from previous governments) are a stretch goal

---

## Policies

### Currently implemented (4)

| Policy | Slot | Unlocked By |
|---|---|---|
| Discipline | Military | Code of Laws |
| Ilkum | Economic | Craftsmanship |
| Urban Planning | Economic | Early Empire |
| Revelation | Wildcard | Mysticism |

### All 113 Base-Game Policies by Slot Type

#### Military Policies (31)

| Policy | Prereq Civic | Effect Summary |
|---|---|---|
| Discipline | Code of Laws | +5 CS vs barbarians |
| Survey | Code of Laws | +1 Movement for Recon units |
| Agoge | Craftsmanship | +50% Production for Ancient/Classical melee, ranged, anti-cavalry |
| Maritime Industries | Foreign Trade | +100% Production for Ancient/Classical naval |
| Maneuver | Military Tradition | +50% Production for Ancient/Classical cavalry |
| Conscription | State Workforce | -1 Gold maintenance for all units |
| Raid | Military Training | bonus Gold from pillaging |
| Veterancy | Military Training | +30% Production for Encampment/Harbor buildings |
| Bastions | Defensive Tactics | +6 city defense, +5 wall HP |
| Limes | Defensive Tactics | +100% Production for walls |
| Feudal Contract | Feudalism | +50% Production for Medieval melee, ranged, anti-cavalry |
| Retainers | Civil Service | +1 Amenity per Encampment with building |
| Sack | Mercenaries | bonus Gold from pillaging improvement |
| Professional Army | Mercenaries | -50% Gold cost to upgrade units |
| Chivalry | Divine Right | +50% Production for Medieval/Renaissance cavalry |
| Press Gangs | Exploration | +100% Production for Medieval/Renaissance naval |
| Wars of Religion | Reformed Church | +4 CS vs other religions |
| Logistics | Mercantilism | +1 Movement for all units |
| Native Conquest | Exploration | +50% Production for musketmen/conquistadors |
| Grande Armée | Nationalism | +50% Production for Industrial melee/ranged |
| National Identity | Nationalism | -50% cost to annex captured cities |
| Total War | Mobilization | +100% Production for Modern era units |
| Military Research | Urbanization | -50% Gold cost for army upgrades |
| Propaganda | Mass Media | -50% war weariness |
| Levée en Masse | Mobilization | +100% unit production, -2 Gold/unit |
| Finest Hour | Suffrage | +15 CS for land units in your territory |
| Lightning Warfare | Totalitarianism | +3 Movement for all units |
| Martial Law | Totalitarianism | -25% war weariness, +1 Production/Military slot |
| Patriotic War | Class Struggle | +100% land unit production, +4 CS in friendly territory |
| Defense of the Motherland | Class Struggle | +100% Production for walls and defenses |
| International Waters | Globalization | +100% naval unit production |
| Military First | Nuclear Program | +50% nuclear/thermonuclear production |

#### Economic Policies (37)

| Policy | Prereq Civic | Effect Summary |
|---|---|---|
| God King | Code of Laws | +1 Faith, +1 Gold in capital |
| Urban Planning | Early Empire | +1 Production in all cities |
| Ilkum | Craftsmanship | +30% Production for builders |
| Caravansaries | Foreign Trade | +2 Gold per trade route |
| Corvée | State Workforce | +15% Production for Ancient/Classical wonders |
| Land Surveyors | Early Empire | -20% Gold cost for tile purchases |
| Colonization | Early Empire | +50% Production for settlers |
| Insulae | Games & Recreation | +1 Housing in all cities |
| Natural Philosophy | Drama & Poetry | +100% adjacency bonus for Campus |
| Scripture | Theology | +100% adjacency bonus for Holy Site |
| Naval Infrastructure | Naval Tradition | +100% adjacency for Harbor |
| Serfdom | Feudalism | +2 Builder charges |
| Meritocracy | Civil Service | +1 Culture per specialty district |
| Trade Confederation | Guilds | +1 Culture, +1 Science per foreign trade route |
| Aesthetics | Medieval Faires | +100% Theater Square adjacency |
| Medina Quarter | Medieval Faires | +2 Housing in cities with specialty district |
| Craftsmen | Guilds | +100% Industrial Zone adjacency |
| Town Charters | Guilds | +100% Commercial Hub adjacency |
| Gothic Architecture | Divine Right | +15% Production for Medieval/Renaissance wonders |
| Colonial Offices | Exploration | +15% faster growth in foreign continent cities |
| Simultaneum | Reformed Church | +100% Holy Site adjacency |
| Triangular Trade | Mercantilism | +4 Gold, +1 Faith per trade route |
| Rationalism | The Enlightenment | +100% Campus adjacency |
| Free Market | The Enlightenment | +100% Commercial Hub adjacency |
| Liberalism | The Enlightenment | +1 Amenity per district with buildings |
| Colonial Taxes | Colonialism | +25% Gold from colonies |
| Public Works | Civil Engineering | +30% Production for builders |
| Skyscrapers | Civil Engineering | -50% cost to buy buildings with Gold |
| Grand Opera | Opera & Ballet | +100% Theater Square adjacency |
| Public Transport | Urbanization | +2 Housing, +1 Food per Entertainment Complex |
| New Deal | Suffrage | +4 Housing, +2 Amenities, -8 Gold per city |
| Third Alternative | Totalitarianism | +4 Gold, +2 Culture per city with governor |
| Five-Year Plan | Class Struggle | +100% Industrial Zone adjacency |
| Collectivization | Class Struggle | +4 Food in cities with governors |
| Heritage Tourism | Conservation | +100% Tourism from cultural improvements |
| Ecommerce | Globalization | +5 Gold, +10 Culture per international trade route |
| Online Communities | Social Media | +50% Tourism from Great Works |

#### Diplomatic Policies (13)

| Policy | Prereq Civic | Effect Summary |
|---|---|---|
| Charismatic Leader | Diplomatic Service | +2 Influence per turn |
| Diplomatic League | Political Philosophy | +1 Envoy when first meeting city-state |
| Merchant Confederation | Medieval Faires | +1 Gold per envoy placed |
| Machiavellianism | Diplomatic Service | +50% Spy production, +3 Spy levels |
| Raj | Colonialism | +2 Science, +2 Culture, +1 Gold per city-state suzerainty |
| Nuclear Espionage | Nuclear Program | +100% Spy production |
| Police State | Totalitarianism | -3 Spy levels for enemy spies, +1 Amenity |
| Arsenal of Democracy | Suffrage | +4 influence per turn |
| Gunboat Diplomacy | Totalitarianism | Gain envoys per turn equal to military power |
| Cryptography | Cold War | +3 defensive Spy levels |
| Containment | Rapid Deployment | +2 envoys when meeting new city-state |
| International Space Agency | Globalization | +5% Science per city-state ally |
| Collective Activism | Social Media | +5% Culture per city-state ally |

#### Wildcard / Great Person Policies (13)

| Policy | Prereq Civic | Effect Summary |
|---|---|---|
| Strategos | Military Tradition | +2 Great General points/turn |
| Inspiration | Mysticism | +2 Great Scientist points/turn |
| Revelation | Mysticism | +2 Great Prophet points/turn |
| Literary Tradition | Drama & Poetry | +2 Great Writer points/turn |
| Navigation | Naval Tradition | +2 Great Admiral points/turn |
| Traveling Merchants | Guilds | +2 Great Merchant points/turn |
| Invention | Humanism | +4 Great Engineer points/turn |
| Frescoes | Humanism | +2 Great Artist points/turn |
| Symphonies | Opera & Ballet | +2 Great Musician points/turn |
| Military Organization | Scorched Earth | +4 General, +4 Admiral points/turn |
| Laissez-Faire | Capitalism | +4 Merchant, +4 Engineer points/turn |
| Nobel Prize | Nuclear Program | +4 Scientist, +4 Writer, +4 Artist, +4 Musician |

### Implementation notes (Policies)

- The `assign_policy()` action and slot system already exist
- Each policy needs: name, slot type, prereq civic, modifiers
- Modifiers use the existing pipeline (`get_modifiers()`)
- "Adjacency doubling" policies (Natural Philosophy, etc.) need the modifier to
  reference district adjacency bonuses

---

## Promotions

### Currently implemented: 0 (framework only)

The implementation has no concrete promotion definitions. Civ VI base game has
**122 promotions** across 16 promotion classes.

### Promotion Classes and Counts

| Class | Unit Types | Promotions | Tier Structure |
|---|---|---|---|
| Recon | Scout, Ranger | 7 | 3 tiers (1→2→1→2→1) |
| Melee | Warrior → Mech. Infantry | 7 | 3 tiers |
| Ranged | Slinger → Machine Gun | 7 | 3 tiers |
| Anti-Cavalry | Spearman → Modern AT | 7 | 3 tiers |
| Light Cavalry | Horseman → Helicopter | 7 | 3 tiers |
| Heavy Cavalry | Heavy Chariot → Modern Armor | 7 | 3 tiers |
| Siege | Catapult → Rocket Artillery | 7 | 3 tiers |
| Naval Melee | Galley → Destroyer | 7 | 3 tiers |
| Naval Ranged | Quadrireme → Missile Cruiser | 7 | 3 tiers |
| Naval Raider | Privateer → Nuclear Sub | 7 | 3 tiers |
| Naval Carrier | Aircraft Carrier | 7 | 3 tiers |
| Air Fighter | Biplane → Jet Fighter | 7 | 3 tiers |
| Air Bomber | Bomber → Jet Bomber | 7 | 3 tiers |
| Monk | Warrior Monk | 7 | 3 tiers |
| Apostle | Apostle | 9 | Flat (pick on creation) |
| Spy | Spy | 11 | Flat (gained on mission) |

### Example promotions (Melee class)

| Tier | Name | Effect |
|---|---|---|
| I | Battlecry | +7 CS vs melee and ranged |
| I | Tortoise | +10 CS vs ranged attacks |
| II | Commando | Can scale cliffs, +1 Movement |
| II | Amphibious | No penalty for river/embark attacks |
| III | Zweihander | +7 CS vs anti-cavalry |
| III | Urban Warfare | +10 CS in district tiles |
| IV | Elite Guard | +1 additional attack per turn when defending |

### Implementation notes (Promotions)

- Need `PromotionDef` struct: name, class, tier, prerequisites (within class), effects
- Effects are combat strength modifiers with conditions (vs unit type, in terrain, etc.)
- The modifier pipeline already supports conditional modifiers
- Promotions are gained when a unit earns enough XP (100 base, scaling per level)
- UI: unit detail panel should show available promotions when eligible

---

## City-States

### Currently implemented: 0 (type system exists but no concrete city-states)

The implementation has `CityStateType` categories but no actual city-state
definitions. Civ VI base game has **24 city-states**.

### All 24 Base-Game City-States

| City-State | Type | Suzerain Bonus |
|---|---|---|
| Amsterdam | Trade | +2 Gold per luxury resource |
| Brussels | Industrial | +15% Production towards wonders |
| Buenos Aires | Industrial | Bonus resources behave as luxury (Amenities) |
| Geneva | Scientific | +15% Science when not at war |
| Hong Kong | Industrial | +20% Production for projects |
| Jakarta | Trade | +1 Trading Post in every city |
| Jerusalem | Religious | Holy city cannot lose majority religion |
| Kabul | Militaristic | Double XP from combat |
| Kandy | Religious | Relic when discovering natural wonder |
| Kumasi | Cultural | +2 Culture, +1 Gold per trade route to Kumasi |
| La Venta | Religious | Can build Colossal Head improvement |
| Lisbon | Trade | Trader units immune to plunder on water |
| Mohenjo-Daro | Cultural | Full housing from water in all cities |
| Nan Madol | Cultural | +2 Culture per district on coast/adjacent to coast |
| Preslav | Militaristic | +50% Production for light/heavy cavalry |
| Seoul | Scientific | +3 Science for each tech researched |
| Stockholm | Scientific | +1 Great Person point per specialty district |
| Toronto | Industrial | Regional buildings extend +3 tiles |
| Valletta | Militaristic | Can buy city center buildings with Faith |
| Vilnius | Cultural | +50% Faith bonus to adjacent Holy Site |
| Yerevan | Religious | Choose Apostle promotions |
| Zanzibar | Trade | Provides Cinnamon and Cloves luxury resources |
| Hattusa | Scientific | Provides 1 copy of each strategic resource you have none of |

### City-State Envoy Bonuses (by type)

| Envoys | Trade | Cultural | Scientific | Religious | Militaristic | Industrial |
|---|---|---|---|---|---|---|
| 1 | +4 Gold in capital | +2 Culture in capital | +2 Science in capital | +2 Faith in capital | +2 Production in capital | +2 Production in capital |
| 3 | +4 Gold in Comm. Hub | +2 Culture in Theater | +2 Science in Campus | +2 Faith in Holy Site | +2 Production in Encampment | +2 Production in Ind. Zone |
| 6 | +4 Gold in Comm. Hub cities | +2 Culture in Theater cities | +2 Science in Campus cities | +2 Faith in Holy Site cities | +2 Production in Encampment cities | +2 Production in Ind. Zone cities |

### Implementation notes (City-States)

- City-states are `City` structs with `CityKind::CityState(CityStateData)`
- Need: name, type, suzerain bonus, envoy bonuses (1/3/6 tiers)
- Suzerain bonuses are modifiers applied to the suzerain civilization
- La Venta and Zanzibar grant unique resources/improvements
- Envoy system (assign envoys, track per-civ) partially exists

---

## Civilizations

### Currently implemented (8)

| Civ | Leader | In Base Game? |
|---|---|---|
| Rome | Trajan | ✓ |
| Greece | Pericles | ✓ |
| Egypt | Cleopatra | ✓ |
| **Babylon** | **Hammurabi** | **No — New Frontier Pass DLC** |
| Germany | Barbarossa | ✓ |
| Japan | Hojo Tokimune | ✓ |
| India | Gandhi | ✓ |
| Arabia | Saladin | ✓ |

### Missing Base-Game Civilizations (11)

| Civilization | Leader | Unique Unit | Unique Infrastructure | Civ Ability |
|---|---|---|---|---|
| America | Teddy Roosevelt | Rough Rider, P-51 Mustang | Film Studio (building) | Founding Fathers: earn govt legacy bonuses faster |
| Brazil | Pedro II | Minas Geraes | Street Carnival (district) | Amazon: Rainforest tiles +1 adjacency for districts |
| China | Qin Shi Huang | Crouching Tiger | Great Wall (improvement) | Dynastic Cycle: eurekas/inspirations +10% |
| England | Victoria | Sea Dog, Redcoat | Royal Navy Dockyard (district) | British Museum: +artifact slots |
| France | Catherine de Medici | Garde Impériale | Chateau (improvement) | Grand Tour: +20% Production for wonders |
| Kongo | Mvemba a Nzinga | Ngao Mbeba | Mbanza (district) | Nkisi: +Great Work slots, +food/prod/gold per Great Work |
| Norway | Harald Hardrada | Berserker, Longship | Stave Church (building) | Knarr: ocean travel, +50% XP naval melee |
| Russia | Peter | Cossack | Lavra (district) | Mother Russia: extra territory on city founding |
| Scythia | Tomyris | Saka Horse Archer | Kurgan (improvement) | People of the Steppe: build 2 light cav/Saka at once |
| Spain | Philip II | Conquistador | Mission (improvement) | Treasure Fleet: +yield to intercontinental trade |
| Sumeria | Gilgamesh | War-Cart | Ziggurat (improvement) | Epic Quest: +tribal village reward on barbarian camp capture |

### Implementation notes (Civilizations)

- Each civ needs: ability modifiers, unique unit def, unique building/district/improvement
- Use existing `register_civilization()` pattern in `civ_registry.rs`
- Unique units must reference base units they replace (needs P6 unit defs)
- Unique buildings/districts must reference base buildings/districts (needs P5)
- Some abilities need new modifier types (e.g. "double unit production")

---

## Great People

### Currently implemented: ~72 individuals (Ancient through Renaissance)

The implementation has 16 per era for Ancient, Classical, Medieval, and 14 for
Renaissance = ~62 individuals across all 8 great person types plus ~10 prophets.

### Base-Game Great People: ~177 individuals

| Type | Implemented | Total (XML) | Missing |
|---|---|---|---|
| Great General | ~8 | ~20 | ~12 |
| Great Admiral | ~8 | ~20 | ~12 |
| Great Engineer | ~8 | ~20 | ~12 |
| Great Merchant | ~8 | ~20 | ~12 |
| Great Scientist | ~8 | ~20 | ~12 |
| Great Writer | ~8 | ~12 | ~4 |
| Great Artist | ~8 | ~12 | ~4 |
| Great Musician | ~8 | ~12 | ~4 |
| Great Prophet | ~10 | ~12 | ~2 |

### Missing eras

The implementation covers Ancient through Renaissance. Missing:
- **Industrial Era** great people (~18 individuals)
- **Modern Era** great people (~18 individuals)
- **Atomic Era** great people (~16 individuals)
- **Information Era** great people (~14 individuals)

### Implementation notes (Great People)

- `GreatPersonDef` struct exists with era, type, name, and effects
- Add entries to the `great_person_defs` registry
- Later-era great people have more powerful effects (free techs, unique buildings)
- Great Musicians create Great Works of Music (tourism)
- Some great people have activation abilities that modify game state
