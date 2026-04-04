# P1 — Complete Tech & Civic Trees

The implementation currently has 12 Ancient-era technologies and 5 Ancient-era
civics. Civ VI base game has **67 technologies** and **50 civics** spanning 8
eras. This document lists every missing node with its cost, era, and
prerequisites, taken directly from `Technologies.xml` and `Civics.xml`.

---

## Technologies

### Currently implemented (Ancient Era — 11 correct + 1 misplaced)

| Tech | Cost | Prereqs | Status |
|---|---|---|---|
| Pottery | 25 | — | ✓ |
| Animal Husbandry | 25 | — | ✓ |
| Mining | 25 | — | ✓ |
| Sailing | 50 | — | ✓ |
| Astrology | 50 | — | ✓ |
| Irrigation | 50 | Pottery | ✓ |
| Archery | 50 | Animal Husbandry | ✓ |
| Writing | 50 | — | ✓ (fix: remove Pottery prereq — see P0) |
| Masonry | 80 | Mining | ✓ |
| Bronze Working | 80 | Mining | ✓ |
| The Wheel | 80 | Mining | ✓ |
| Theology | 120 | Astrology | **WRONG** — this is a Civic, not a Tech (move to civics, see P0) |

### Missing: Classical Era (8 techs)

| Tech | Cost | Prerequisites |
|---|---|---|
| Celestial Navigation | 120 | Sailing, Astrology |
| Currency | 120 | Writing |
| Horseback Riding | 120 | Archery |
| Iron Working | 120 | Bronze Working |
| Shipbuilding | 200 | Sailing |
| Mathematics | 200 | Currency |
| Construction | 200 | Masonry, Horseback Riding |
| Engineering | 200 | The Wheel |

### Missing: Medieval Era (7 techs)

| Tech | Cost | Prerequisites |
|---|---|---|
| Military Tactics | 300 | Mathematics |
| Apprenticeship | 300 | Currency, Horseback Riding |
| Machinery | 300 | Iron Working, Engineering |
| Education | 390 | Mathematics, Apprenticeship |
| Stirrups | 390 | Horseback Riding |
| Military Engineering | 390 | Construction |
| Castles | 390 | Construction |

### Missing: Renaissance Era (8 techs)

| Tech | Cost | Prerequisites |
|---|---|---|
| Cartography | 540 | Shipbuilding |
| Mass Production | 540 | Education, Shipbuilding |
| Banking | 540 | Education, Stirrups |
| Gunpowder | 540 | Apprenticeship, Stirrups, Military Engineering |
| Printing | 540 | Machinery |
| Square Rigging | 660 | Cartography |
| Astronomy | 660 | Education |
| Metal Casting | 660 | Gunpowder |
| Siege Tactics | 660 | Castles |

### Missing: Industrial Era (8 techs)

| Tech | Cost | Prerequisites |
|---|---|---|
| Industrialization | 845 | Square Rigging, Mass Production |
| Scientific Theory | 845 | Astronomy, Banking |
| Ballistics | 845 | Metal Casting |
| Military Science | 845 | Siege Tactics, Printing |
| Steam Power | 970 | Industrialization |
| Sanitation | 970 | Scientific Theory |
| Economics | 970 | Scientific Theory, Metal Casting |
| Rifling | 970 | Ballistics, Military Science |

### Missing: Modern Era (8 techs)

| Tech | Cost | Prerequisites |
|---|---|---|
| Flight | 1140 | Industrialization, Scientific Theory |
| Replaceable Parts | 1140 | Economics |
| Steel | 1140 | Rifling |
| Electricity | 1250 | Steam Power |
| Radio | 1250 | Steam Power, Flight |
| Chemistry | 1250 | Sanitation |
| Combustion | 1250 | Steel, Rifling |

### Missing: Atomic Era (8 techs)

| Tech | Cost | Prerequisites |
|---|---|---|
| Advanced Flight | 1410 | Radio |
| Rocketry | 1410 | Radio, Chemistry |
| Advanced Ballistics | 1410 | Replaceable Parts, Steel |
| Combined Arms | 1410 | Steel, Combustion |
| Plastics | 1410 | Combustion |
| Computers | 1580 | Electricity, Radio |
| Nuclear Fission | 1580 | Advanced Ballistics, Combined Arms |
| Synthetic Materials | 1580 | Plastics |

### Missing: Information Era (10 techs)

| Tech | Cost | Prerequisites |
|---|---|---|
| Telecommunications | 1850 | Computers |
| Satellites | 1850 | Advanced Flight, Rocketry |
| Guidance Systems | 1850 | Rocketry, Advanced Ballistics |
| Lasers | 1850 | Nuclear Fission |
| Composites | 1850 | Synthetic Materials |
| Stealth Technology | 1850 | Synthetic Materials |
| Robotics | 2155 | Computers |
| Nanotechnology | 2155 | Composites |
| Nuclear Fusion | 2155 | Lasers |
| Future Tech | 2500 | Satellites, Robotics, Nanotechnology, Nuclear Fusion |

**Total missing technologies: 56** (8 Classical + 7 Medieval + 9 Renaissance + 8 Industrial + 7 Modern + 8 Atomic + 10 Information)

---

## Civics

### Currently implemented (Ancient Era — 5 nodes)

| Civic | Cost | Prereqs | Status |
|---|---|---|---|
| Code of Laws | 20 | — | ✓ |
| Craftsmanship | 40 | Code of Laws | ✓ |
| Foreign Trade | 40 | Code of Laws | ✓ |
| Early Empire | 70 | Foreign Trade | ✓ (fix: prereqs currently include Craftsmanship — see P0) |
| Mysticism | 50 | Foreign Trade | ✓ (fix: prereq currently Code of Laws — see P0) |

### Missing Ancient-era civics (added as part of P0)

| Civic | Cost | Prerequisites |
|---|---|---|
| Military Tradition | 50 | Craftsmanship |
| State Workforce | 70 | Craftsmanship |

### Missing: Classical Era (7 civics)

| Civic | Cost | Prerequisites |
|---|---|---|
| Games & Recreation | 110 | State Workforce |
| Political Philosophy | 110 | State Workforce, Early Empire |
| Drama & Poetry | 110 | Early Empire |
| Theology | 120 | Drama & Poetry, Mysticism |
| Military Training | 120 | Military Tradition, Games & Recreation |
| Defensive Tactics | 175 | Games & Recreation, Political Philosophy |
| Recorded History | 175 | Political Philosophy, Drama & Poetry |

### Missing: Medieval Era (7 civics)

| Civic | Cost | Prerequisites |
|---|---|---|
| Naval Tradition | 200 | Defensive Tactics |
| Feudalism | 275 | Defensive Tactics |
| Civil Service | 275 | Defensive Tactics, Recorded History |
| Divine Right | 290 | Civil Service, Theology |
| Mercenaries | 290 | Military Training, Feudalism |
| Medieval Faires | 385 | Feudalism |
| Guilds | 385 | Feudalism, Civil Service |

### Missing: Renaissance Era (5 civics)

| Civic | Cost | Prerequisites |
|---|---|---|
| Exploration | 400 | Mercenaries, Medieval Faires |
| Reformed Church | 400 | Guilds, Divine Right |
| Humanism | 540 | Medieval Faires, Guilds |
| Diplomatic Service | 540 | Guilds |
| Mercantilism | 655 | Humanism |
| The Enlightenment | 655 | Humanism, Diplomatic Service |

### Missing: Industrial Era (7 civics)

| Civic | Cost | Prerequisites |
|---|---|---|
| Colonialism | 725 | Mercantilism |
| Opera & Ballet | 725 | The Enlightenment |
| Natural History | 870 | Colonialism |
| Civil Engineering | 920 | Mercantilism |
| Nationalism | 920 | The Enlightenment |
| Scorched Earth | 1060 | Nationalism |
| Urbanization | 1060 | Civil Engineering, Nationalism |

### Missing: Modern Era (8 civics)

| Civic | Cost | Prerequisites |
|---|---|---|
| Conservation | 1255 | Natural History |
| Mass Media | 1410 | Natural History, Urbanization |
| Mobilization | 1410 | Urbanization |
| Ideology | 660 | Mass Media, Mobilization |
| Capitalism | 1560 | Mass Media |
| Nuclear Program | 1715 | Ideology |
| Suffrage | 1715 | Ideology |
| Totalitarianism | 1715 | Ideology |
| Class Struggle | 1715 | Ideology |

### Missing: Atomic Era (5 civics)

| Civic | Cost | Prerequisites |
|---|---|---|
| Cultural Heritage | 1955 | Conservation |
| Cold War | 2185 | Ideology |
| Professional Sports | 2185 | Ideology |
| Rapid Deployment | 2415 | Cold War |
| Space Race | 2415 | Cold War |

### Missing: Information Era (3 civics)

| Civic | Cost | Prerequisites |
|---|---|---|
| Globalization | 2880 | Rapid Deployment, Space Race |
| Social Media | 2880 | Space Race, Professional Sports |
| Future Civic | 3200 | Globalization, Social Media |

**Total missing civics: 46** (2 Ancient [P0] + 7 Classical + 7 Medieval + 6 Renaissance + 7 Industrial + 9 Modern + 5 Atomic + 3 Information)

---

## Implementation notes

### File structure

The current `tech_tree_def.rs` and `civic_tree_def.rs` use an `include!()` block
pattern. Each era's nodes can be appended to the existing block. The `TechId` /
`CivicId` type is `&'static str`, so new nodes just need unique string literals.

### Era enum

`GameState.eras` already contains all 9 era definitions (Ancient through Future).
No changes needed to the era system.

### Unlock effects

Each tech/civic node carries `Vec<Effect>` — the effects will reference buildings,
units, improvements, and governments that may not exist yet. Use placeholder
string IDs; the downstream phases (P5–P8) will create the actual definitions.
The engine already tolerates unresolved unlock references at runtime.

### Suggested implementation order

1. Add Classical-era techs and civics first (unblocks district prereq fixes).
2. Add Medieval and Renaissance together (they are tightly coupled via prereqs).
3. Add Industrial through Information as a single batch.
4. Add `Future Tech` and `Future Civic` last (they are repeatable techs).
