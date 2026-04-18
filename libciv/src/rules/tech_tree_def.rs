// Full tech tree definition — Ancient through Information era.
// Included verbatim inside `build_tech_tree`; `tree`, `ids`, `TechRefs`, and `OneShotEffect::*`
// are all in scope from the enclosing function. This file must be a single block expression
// that evaluates to `TechRefs`.
{
// ── Generate IDs in a fixed order (never reorder) ────────────────────────────

// Ancient Era
let pottery_id              = TechId::from_ulid(ids.next_ulid());
let animal_id               = TechId::from_ulid(ids.next_ulid());
let mining_id               = TechId::from_ulid(ids.next_ulid());
let sailing_id              = TechId::from_ulid(ids.next_ulid());
let archery_id              = TechId::from_ulid(ids.next_ulid());
let astrology_id            = TechId::from_ulid(ids.next_ulid());
let writing_id              = TechId::from_ulid(ids.next_ulid());
let irrigation_id           = TechId::from_ulid(ids.next_ulid());
let bronze_working_id       = TechId::from_ulid(ids.next_ulid());
let the_wheel_id            = TechId::from_ulid(ids.next_ulid());
let masonry_id              = TechId::from_ulid(ids.next_ulid());

// Classical Era
let celestial_navigation_id = TechId::from_ulid(ids.next_ulid());
let currency_id             = TechId::from_ulid(ids.next_ulid());
let horseback_riding_id     = TechId::from_ulid(ids.next_ulid());
let iron_working_id         = TechId::from_ulid(ids.next_ulid());
let shipbuilding_id         = TechId::from_ulid(ids.next_ulid());
let mathematics_id          = TechId::from_ulid(ids.next_ulid());
let construction_id         = TechId::from_ulid(ids.next_ulid());
let engineering_id          = TechId::from_ulid(ids.next_ulid());

// Medieval Era
let military_tactics_id     = TechId::from_ulid(ids.next_ulid());
let apprenticeship_id       = TechId::from_ulid(ids.next_ulid());
let machinery_id            = TechId::from_ulid(ids.next_ulid());
let education_id            = TechId::from_ulid(ids.next_ulid());
let stirrups_id             = TechId::from_ulid(ids.next_ulid());
let military_engineering_id = TechId::from_ulid(ids.next_ulid());
let castles_id              = TechId::from_ulid(ids.next_ulid());

// Renaissance Era
let cartography_id          = TechId::from_ulid(ids.next_ulid());
let mass_production_id      = TechId::from_ulid(ids.next_ulid());
let banking_id              = TechId::from_ulid(ids.next_ulid());
let gunpowder_id            = TechId::from_ulid(ids.next_ulid());
let printing_id             = TechId::from_ulid(ids.next_ulid());
let square_rigging_id       = TechId::from_ulid(ids.next_ulid());
let astronomy_id            = TechId::from_ulid(ids.next_ulid());
let metal_casting_id        = TechId::from_ulid(ids.next_ulid());
let siege_tactics_id        = TechId::from_ulid(ids.next_ulid());

// Industrial Era
let industrialization_id    = TechId::from_ulid(ids.next_ulid());
let scientific_theory_id    = TechId::from_ulid(ids.next_ulid());
let ballistics_id           = TechId::from_ulid(ids.next_ulid());
let military_science_id     = TechId::from_ulid(ids.next_ulid());
let steam_power_id          = TechId::from_ulid(ids.next_ulid());
let sanitation_id           = TechId::from_ulid(ids.next_ulid());
let economics_id            = TechId::from_ulid(ids.next_ulid());
let rifling_id              = TechId::from_ulid(ids.next_ulid());

// Modern Era
let flight_id               = TechId::from_ulid(ids.next_ulid());
let replaceable_parts_id    = TechId::from_ulid(ids.next_ulid());
let steel_id                = TechId::from_ulid(ids.next_ulid());
let electricity_id          = TechId::from_ulid(ids.next_ulid());
let radio_id                = TechId::from_ulid(ids.next_ulid());
let chemistry_id            = TechId::from_ulid(ids.next_ulid());
let combustion_id           = TechId::from_ulid(ids.next_ulid());

// Atomic Era
let advanced_flight_id      = TechId::from_ulid(ids.next_ulid());
let rocketry_id             = TechId::from_ulid(ids.next_ulid());
let advanced_ballistics_id  = TechId::from_ulid(ids.next_ulid());
let combined_arms_id        = TechId::from_ulid(ids.next_ulid());
let plastics_id             = TechId::from_ulid(ids.next_ulid());
let computers_id            = TechId::from_ulid(ids.next_ulid());
let nuclear_fission_id      = TechId::from_ulid(ids.next_ulid());
let synthetic_materials_id  = TechId::from_ulid(ids.next_ulid());

// Information Era
let telecommunications_id   = TechId::from_ulid(ids.next_ulid());
let satellites_id           = TechId::from_ulid(ids.next_ulid());
let guidance_systems_id     = TechId::from_ulid(ids.next_ulid());
let lasers_id               = TechId::from_ulid(ids.next_ulid());
let composites_id           = TechId::from_ulid(ids.next_ulid());
let stealth_technology_id   = TechId::from_ulid(ids.next_ulid());
let robotics_id             = TechId::from_ulid(ids.next_ulid());
let nanotechnology_id       = TechId::from_ulid(ids.next_ulid());
let nuclear_fusion_id       = TechId::from_ulid(ids.next_ulid());
let future_tech_id          = TechId::from_ulid(ids.next_ulid());

// Future Era (GS)
let seasteads_id            = TechId::from_ulid(ids.next_ulid());
let advanced_ai_id          = TechId::from_ulid(ids.next_ulid());
let advanced_power_cells_id = TechId::from_ulid(ids.next_ulid());
let cybernetics_id          = TechId::from_ulid(ids.next_ulid());
let smart_materials_id      = TechId::from_ulid(ids.next_ulid());
let predictive_systems_id   = TechId::from_ulid(ids.next_ulid());
let offworld_mission_id     = TechId::from_ulid(ids.next_ulid());

// Sentinel
let unreachable_id          = TechId::from_ulid(ids.next_ulid());

// ══════════════════════════════════════════════════════════════════════════════
// Ancient Era
// ══════════════════════════════════════════════════════════════════════════════

let pottery = TechNode {
    id:                 pottery_id,
    name:               "Pottery",
    cost:               25,
    prerequisites:      vec![],
    effects:            vec![UnlockBuilding("Granary"), UnlockImprovement("Farm")],
    eureka_description: "Harvest any resource.",
    eureka_effects:     vec![],
};

let animal = TechNode {
    id:                 animal_id,
    name:               "Animal Husbandry",
    cost:               25,
    prerequisites:      vec![],
    effects:            vec![UnlockImprovement("Pasture")],
    eureka_description: "Find a tribal village.",
    eureka_effects:     vec![],
};

let mining = TechNode {
    id:                 mining_id,
    name:               "Mining",
    cost:               25,
    prerequisites:      vec![],
    effects:            vec![UnlockImprovement("Mine")],
    eureka_description: "Clear a Forest or Rainforest.",
    eureka_effects:     vec![],
};

let sailing = TechNode {
    id:                 sailing_id,
    name:               "Sailing",
    cost:               50,
    prerequisites:      vec![],
    effects:            vec![UnlockUnit("Galley")],
    eureka_description: "Found a city on the coast.",
    eureka_effects:     vec![],
};

let archery = TechNode {
    id:                 archery_id,
    name:               "Archery",
    cost:               50,
    prerequisites:      vec![animal_id],
    effects:            vec![UnlockUnit("Archer")],
    eureka_description: "Kill a unit.",
    eureka_effects:     vec![],
};

let astrology = TechNode {
    id:                 astrology_id,
    name:               "Astrology",
    cost:               50,
    prerequisites:      vec![],
    effects:            vec![UnlockBuilding("Shrine")],
    eureka_description: "Find a Natural Wonder.",
    eureka_effects:     vec![],
};

// P0 fix: Writing has no prerequisites in base Civ VI (removed Pottery prereq).
let writing = TechNode {
    id:                 writing_id,
    name:               "Writing",
    cost:               50,
    prerequisites:      vec![],
    effects:            vec![UnlockBuilding("Library")],
    eureka_description: "Meet another civilization.",
    eureka_effects:     vec![],
};

let irrigation = TechNode {
    id:                 irrigation_id,
    name:               "Irrigation",
    cost:               50,
    prerequisites:      vec![pottery_id],
    effects:            vec![UnlockImprovement("Irrigation")],
    eureka_description: "Find a Floodplain or River.",
    eureka_effects:     vec![],
};

let bronze_working = TechNode {
    id:                 bronze_working_id,
    name:               "Bronze Working",
    cost:               80,
    prerequisites:      vec![mining_id],
    effects:            vec![UnlockUnit("Spearman")],
    eureka_description: "Kill 3 units.",
    eureka_effects:     vec![],
};

let the_wheel = TechNode {
    id:                 the_wheel_id,
    name:               "The Wheel",
    cost:               80,
    prerequisites:      vec![mining_id],
    effects:            vec![UnlockUnit("Heavy Chariot")],
    eureka_description: "Build a Quarry.",
    eureka_effects:     vec![],
};

let masonry = TechNode {
    id:                 masonry_id,
    name:               "Masonry",
    cost:               80,
    prerequisites:      vec![mining_id],
    effects:            vec![UnlockBuilding("Walls"), UnlockImprovement("Quarry")],
    eureka_description: "Build a Quarry.",
    eureka_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Classical Era
// ══════════════════════════════════════════════════════════════════════════════

let celestial_navigation = TechNode {
    id:                 celestial_navigation_id,
    name:               "Celestial Navigation",
    cost:               120,
    prerequisites:      vec![sailing_id, astrology_id],
    effects:            vec![UnlockBuilding("Lighthouse")],
    eureka_description: "Improve 2 sea resources.",
    eureka_effects:     vec![],
};

let currency = TechNode {
    id:                 currency_id,
    name:               "Currency",
    cost:               120,
    prerequisites:      vec![writing_id],
    effects:            vec![UnlockBuilding("Market")],
    eureka_description: "Make a trade route.",
    eureka_effects:     vec![],
};

let horseback_riding = TechNode {
    id:                 horseback_riding_id,
    name:               "Horseback Riding",
    cost:               120,
    prerequisites:      vec![archery_id],
    effects:            vec![UnlockUnit("Horseman")],
    eureka_description: "Build a Pasture.",
    eureka_effects:     vec![],
};

let iron_working = TechNode {
    id:                 iron_working_id,
    name:               "Iron Working",
    cost:               120,
    prerequisites:      vec![bronze_working_id],
    effects:            vec![UnlockUnit("Swordsman")],
    eureka_description: "Build an Iron Mine.",
    eureka_effects:     vec![],
};

let shipbuilding = TechNode {
    id:                 shipbuilding_id,
    name:               "Shipbuilding",
    cost:               200,
    prerequisites:      vec![sailing_id],
    effects:            vec![UnlockUnit("Quadrireme"), EnableEmbarkCoast],
    eureka_description: "Own 2 Galleys.",
    eureka_effects:     vec![],
};

let mathematics = TechNode {
    id:                 mathematics_id,
    name:               "Mathematics",
    cost:               200,
    prerequisites:      vec![currency_id],
    effects:            vec![],
    eureka_description: "Build 3 different specialty districts.",
    eureka_effects:     vec![],
};

let construction = TechNode {
    id:                 construction_id,
    name:               "Construction",
    cost:               200,
    prerequisites:      vec![masonry_id, horseback_riding_id],
    effects:            vec![UnlockBuilding("Arena")],
    eureka_description: "Build a Water Mill.",
    eureka_effects:     vec![],
};

let engineering = TechNode {
    id:                 engineering_id,
    name:               "Engineering",
    cost:               200,
    prerequisites:      vec![the_wheel_id],
    effects:            vec![UnlockImprovement("Aqueduct")],
    eureka_description: "Build Ancient Walls.",
    eureka_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Medieval Era
// ══════════════════════════════════════════════════════════════════════════════

let military_tactics = TechNode {
    id:                 military_tactics_id,
    name:               "Military Tactics",
    cost:               300,
    prerequisites:      vec![mathematics_id],
    effects:            vec![],
    eureka_description: "Kill a unit with a Spearman.",
    eureka_effects:     vec![],
};

let apprenticeship = TechNode {
    id:                 apprenticeship_id,
    name:               "Apprenticeship",
    cost:               300,
    prerequisites:      vec![currency_id, horseback_riding_id],
    effects:            vec![],
    eureka_description: "Build 3 Mines.",
    eureka_effects:     vec![],
};

let machinery = TechNode {
    id:                 machinery_id,
    name:               "Machinery",
    cost:               300,
    prerequisites:      vec![iron_working_id, engineering_id],
    effects:            vec![UnlockUnit("Crossbowman")],
    eureka_description: "Own 3 Archers.",
    eureka_effects:     vec![],
};

let education = TechNode {
    id:                 education_id,
    name:               "Education",
    cost:               390,
    prerequisites:      vec![mathematics_id, apprenticeship_id],
    effects:            vec![UnlockBuilding("University")],
    eureka_description: "Earn a Great Scientist.",
    eureka_effects:     vec![],
};

let stirrups = TechNode {
    id:                 stirrups_id,
    name:               "Stirrups",
    cost:               390,
    prerequisites:      vec![horseback_riding_id],
    effects:            vec![UnlockUnit("Knight")],
    eureka_description: "Build a Stable.",
    eureka_effects:     vec![],
};

let military_engineering = TechNode {
    id:                 military_engineering_id,
    name:               "Military Engineering",
    cost:               390,
    prerequisites:      vec![construction_id],
    effects:            vec![],
    eureka_description: "Build an Aqueduct.",
    eureka_effects:     vec![],
};

let castles = TechNode {
    id:                 castles_id,
    name:               "Castles",
    cost:               390,
    prerequisites:      vec![construction_id],
    effects:            vec![],
    eureka_description: "Have a government with 6 policy slots.",
    eureka_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Renaissance Era
// ══════════════════════════════════════════════════════════════════════════════

let cartography = TechNode {
    id:                 cartography_id,
    name:               "Cartography",
    cost:               540,
    prerequisites:      vec![shipbuilding_id],
    effects:            vec![UnlockUnit("Caravel"), EnableEmbarkOcean],
    eureka_description: "Build 2 Harbors.",
    eureka_effects:     vec![],
};

let mass_production = TechNode {
    id:                 mass_production_id,
    name:               "Mass Production",
    cost:               540,
    prerequisites:      vec![education_id, shipbuilding_id],
    effects:            vec![],
    eureka_description: "Build a Lumber Mill.",
    eureka_effects:     vec![],
};

let banking = TechNode {
    id:                 banking_id,
    name:               "Banking",
    cost:               540,
    prerequisites:      vec![education_id, stirrups_id],
    effects:            vec![UnlockBuilding("Bank")],
    eureka_description: "Have the Feudalism civic.",
    eureka_effects:     vec![],
};

let gunpowder = TechNode {
    id:                 gunpowder_id,
    name:               "Gunpowder",
    cost:               540,
    prerequisites:      vec![apprenticeship_id, stirrups_id, military_engineering_id],
    effects:            vec![UnlockUnit("Musketman")],
    eureka_description: "Build an Armory.",
    eureka_effects:     vec![],
};

let printing = TechNode {
    id:                 printing_id,
    name:               "Printing",
    cost:               540,
    prerequisites:      vec![machinery_id],
    effects:            vec![],
    eureka_description: "Build 2 Universities.",
    eureka_effects:     vec![],
};

let square_rigging = TechNode {
    id:                 square_rigging_id,
    name:               "Square Rigging",
    cost:               660,
    prerequisites:      vec![cartography_id],
    effects:            vec![UnlockUnit("Frigate")],
    eureka_description: "Kill a unit with a Musketman.",
    eureka_effects:     vec![],
};

let astronomy = TechNode {
    id:                 astronomy_id,
    name:               "Astronomy",
    cost:               660,
    prerequisites:      vec![education_id],
    effects:            vec![],
    eureka_description: "Build a University adjacent to a Mountain.",
    eureka_effects:     vec![],
};

let metal_casting = TechNode {
    id:                 metal_casting_id,
    name:               "Metal Casting",
    cost:               660,
    prerequisites:      vec![gunpowder_id],
    effects:            vec![UnlockUnit("Bombard")],
    eureka_description: "Own 2 Crossbowmen.",
    eureka_effects:     vec![],
};

let siege_tactics = TechNode {
    id:                 siege_tactics_id,
    name:               "Siege Tactics",
    cost:               660,
    prerequisites:      vec![castles_id],
    effects:            vec![],
    eureka_description: "Earn a Great Engineer.",
    eureka_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Industrial Era
// ══════════════════════════════════════════════════════════════════════════════

let industrialization = TechNode {
    id:                 industrialization_id,
    name:               "Industrialization",
    cost:               845,
    prerequisites:      vec![square_rigging_id, mass_production_id],
    effects:            vec![],
    eureka_description: "Build 3 Workshops.",
    eureka_effects:     vec![],
};

let scientific_theory = TechNode {
    id:                 scientific_theory_id,
    name:               "Scientific Theory",
    cost:               845,
    prerequisites:      vec![astronomy_id, banking_id],
    effects:            vec![],
    eureka_description: "Have the Enlightenment civic.",
    eureka_effects:     vec![],
};

let ballistics = TechNode {
    id:                 ballistics_id,
    name:               "Ballistics",
    cost:               845,
    prerequisites:      vec![metal_casting_id],
    effects:            vec![],
    eureka_description: "Have 2 Forts in your territory.",
    eureka_effects:     vec![],
};

let military_science = TechNode {
    id:                 military_science_id,
    name:               "Military Science",
    cost:               845,
    prerequisites:      vec![siege_tactics_id, printing_id],
    effects:            vec![UnlockUnit("Cavalry")],
    eureka_description: "Kill a unit with a Knight.",
    eureka_effects:     vec![],
};

let steam_power = TechNode {
    id:                 steam_power_id,
    name:               "Steam Power",
    cost:               970,
    prerequisites:      vec![industrialization_id],
    effects:            vec![UnlockUnit("Ironclad")],
    eureka_description: "Build 2 Shipyards.",
    eureka_effects:     vec![],
};

let sanitation = TechNode {
    id:                 sanitation_id,
    name:               "Sanitation",
    cost:               970,
    prerequisites:      vec![scientific_theory_id],
    effects:            vec![UnlockBuilding("Sewer")],
    eureka_description: "Build 2 Neighborhoods.",
    eureka_effects:     vec![],
};

let economics = TechNode {
    id:                 economics_id,
    name:               "Economics",
    cost:               970,
    prerequisites:      vec![scientific_theory_id, metal_casting_id],
    effects:            vec![UnlockBuilding("Stock Exchange")],
    eureka_description: "Build 2 Banks.",
    eureka_effects:     vec![],
};

let rifling = TechNode {
    id:                 rifling_id,
    name:               "Rifling",
    cost:               970,
    prerequisites:      vec![ballistics_id, military_science_id],
    effects:            vec![UnlockUnit("Line Infantry")],
    eureka_description: "Build a Niter Mine.",
    eureka_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Modern Era
// ══════════════════════════════════════════════════════════════════════════════

let flight = TechNode {
    id:                 flight_id,
    name:               "Flight",
    cost:               1140,
    prerequisites:      vec![industrialization_id, scientific_theory_id],
    effects:            vec![UnlockUnit("Biplane")],
    eureka_description: "Build an Industrial-era or later wonder.",
    eureka_effects:     vec![],
};

let replaceable_parts = TechNode {
    id:                 replaceable_parts_id,
    name:               "Replaceable Parts",
    cost:               1140,
    prerequisites:      vec![economics_id],
    effects:            vec![UnlockUnit("Infantry")],
    eureka_description: "Own 3 Musketmen.",
    eureka_effects:     vec![],
};

let steel = TechNode {
    id:                 steel_id,
    name:               "Steel",
    cost:               1140,
    prerequisites:      vec![rifling_id],
    effects:            vec![],
    eureka_description: "Build a Coal Mine.",
    eureka_effects:     vec![],
};

let electricity = TechNode {
    id:                 electricity_id,
    name:               "Electricity",
    cost:               1250,
    prerequisites:      vec![steam_power_id],
    effects:            vec![],
    eureka_description: "Own 3 Privateers.",
    eureka_effects:     vec![],
};

let radio = TechNode {
    id:                 radio_id,
    name:               "Radio",
    cost:               1250,
    prerequisites:      vec![steam_power_id, flight_id],
    effects:            vec![],
    eureka_description: "Build a National Park.",
    eureka_effects:     vec![],
};

let chemistry = TechNode {
    id:                 chemistry_id,
    name:               "Chemistry",
    cost:               1250,
    prerequisites:      vec![sanitation_id],
    effects:            vec![],
    eureka_description: "Complete a Research Agreement.",
    eureka_effects:     vec![],
};

let combustion = TechNode {
    id:                 combustion_id,
    name:               "Combustion",
    cost:               1250,
    prerequisites:      vec![steel_id, rifling_id],
    effects:            vec![UnlockUnit("Artillery")],
    eureka_description: "Extract an Artifact.",
    eureka_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Atomic Era
// ══════════════════════════════════════════════════════════════════════════════

let advanced_flight = TechNode {
    id:                 advanced_flight_id,
    name:               "Advanced Flight",
    cost:               1410,
    prerequisites:      vec![radio_id],
    effects:            vec![UnlockUnit("Fighter"), UnlockUnit("Bomber")],
    eureka_description: "Build 3 Biplanes.",
    eureka_effects:     vec![],
};

let rocketry = TechNode {
    id:                 rocketry_id,
    name:               "Rocketry",
    cost:               1410,
    prerequisites:      vec![radio_id, chemistry_id],
    effects:            vec![],
    eureka_description: "Boost through a Great Scientist.",
    eureka_effects:     vec![],
};

let advanced_ballistics = TechNode {
    id:                 advanced_ballistics_id,
    name:               "Advanced Ballistics",
    cost:               1410,
    prerequisites:      vec![replaceable_parts_id, steel_id],
    effects:            vec![],
    eureka_description: "Build 2 Power Plants.",
    eureka_effects:     vec![],
};

let combined_arms = TechNode {
    id:                 combined_arms_id,
    name:               "Combined Arms",
    cost:               1410,
    prerequisites:      vec![steel_id, combustion_id],
    effects:            vec![UnlockUnit("Tank")],
    eureka_description: "Build an Airstrip.",
    eureka_effects:     vec![],
};

let plastics = TechNode {
    id:                 plastics_id,
    name:               "Plastics",
    cost:               1410,
    prerequisites:      vec![combustion_id],
    effects:            vec![],
    eureka_description: "Build a Research Lab.",
    eureka_effects:     vec![],
};

let computers = TechNode {
    id:                 computers_id,
    name:               "Computers",
    cost:               1580,
    prerequisites:      vec![electricity_id, radio_id],
    effects:            vec![],
    eureka_description: "Have a government with 8 policy slots.",
    eureka_effects:     vec![],
};

let nuclear_fission = TechNode {
    id:                 nuclear_fission_id,
    name:               "Nuclear Fission",
    cost:               1580,
    prerequisites:      vec![advanced_ballistics_id, combined_arms_id],
    effects:            vec![],
    eureka_description: "Boost through a Great Scientist.",
    eureka_effects:     vec![],
};

let synthetic_materials = TechNode {
    id:                 synthetic_materials_id,
    name:               "Synthetic Materials",
    cost:               1580,
    prerequisites:      vec![plastics_id],
    effects:            vec![],
    eureka_description: "Build an Aerodrome.",
    eureka_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Information Era
// ══════════════════════════════════════════════════════════════════════════════

let telecommunications = TechNode {
    id:                 telecommunications_id,
    name:               "Telecommunications",
    cost:               1850,
    prerequisites:      vec![computers_id],
    effects:            vec![],
    eureka_description: "Build 2 Broadcast Centers.",
    eureka_effects:     vec![],
};

let satellites = TechNode {
    id:                 satellites_id,
    name:               "Satellites",
    cost:               1850,
    prerequisites:      vec![advanced_flight_id, rocketry_id],
    effects:            vec![],
    eureka_description: "Boost through a Great Scientist.",
    eureka_effects:     vec![],
};

let guidance_systems = TechNode {
    id:                 guidance_systems_id,
    name:               "Guidance Systems",
    cost:               1850,
    prerequisites:      vec![rocketry_id, advanced_ballistics_id],
    effects:            vec![UnlockUnit("Rocket Artillery")],
    eureka_description: "Kill a Fighter.",
    eureka_effects:     vec![],
};

let lasers = TechNode {
    id:                 lasers_id,
    name:               "Lasers",
    cost:               1850,
    prerequisites:      vec![nuclear_fission_id],
    effects:            vec![],
    eureka_description: "Boost through a Great Scientist.",
    eureka_effects:     vec![],
};

let composites = TechNode {
    id:                 composites_id,
    name:               "Composites",
    cost:               1850,
    prerequisites:      vec![synthetic_materials_id],
    effects:            vec![],
    eureka_description: "Build 3 Tanks.",
    eureka_effects:     vec![],
};

let stealth_technology = TechNode {
    id:                 stealth_technology_id,
    name:               "Stealth Technology",
    cost:               1850,
    prerequisites:      vec![synthetic_materials_id],
    effects:            vec![UnlockUnit("Jet Fighter"), UnlockUnit("Jet Bomber")],
    eureka_description: "Boost through a Great Scientist.",
    eureka_effects:     vec![],
};

let robotics = TechNode {
    id:                 robotics_id,
    name:               "Robotics",
    cost:               2155,
    prerequisites:      vec![computers_id],
    effects:            vec![],
    eureka_description: "Have the Globalization civic.",
    eureka_effects:     vec![],
};

let nanotechnology = TechNode {
    id:                 nanotechnology_id,
    name:               "Nanotechnology",
    cost:               2155,
    prerequisites:      vec![composites_id],
    effects:            vec![],
    eureka_description: "Build an Aluminum Mine.",
    eureka_effects:     vec![],
};

let nuclear_fusion = TechNode {
    id:                 nuclear_fusion_id,
    name:               "Nuclear Fusion",
    cost:               2155,
    prerequisites:      vec![lasers_id],
    effects:            vec![],
    eureka_description: "Boost through a Great Scientist.",
    eureka_effects:     vec![],
};

// ── Future Era (Gathering Storm) ─────────────────────────────────────────────

let seasteads = TechNode {
    id:                 seasteads_id,
    name:               "Seasteads",
    cost:               2155,
    prerequisites:      vec![composites_id],
    effects:            vec![],
    eureka_description: "Build a Seastead.",
    eureka_effects:     vec![],
};

let advanced_ai = TechNode {
    id:                 advanced_ai_id,
    name:               "Advanced AI",
    cost:               2155,
    prerequisites:      vec![computers_id],
    effects:            vec![],
    eureka_description: "Build a Robot Factory.",
    eureka_effects:     vec![],
};

let advanced_power_cells = TechNode {
    id:                 advanced_power_cells_id,
    name:               "Advanced Power Cells",
    cost:               2155,
    prerequisites:      vec![nuclear_fusion_id],
    effects:            vec![],
    eureka_description: "Build a Solar Farm.",
    eureka_effects:     vec![],
};

let cybernetics = TechNode {
    id:                 cybernetics_id,
    name:               "Cybernetics",
    cost:               2500,
    prerequisites:      vec![advanced_ai_id],
    effects:            vec![UnlockUnit("Giant Death Robot")],
    eureka_description: "Build the Giant Death Robot.",
    eureka_effects:     vec![],
};

let smart_materials = TechNode {
    id:                 smart_materials_id,
    name:               "Smart Materials",
    cost:               2500,
    prerequisites:      vec![nanotechnology_id],
    effects:            vec![],
    eureka_description: "Build 2 Solar Farms.",
    eureka_effects:     vec![],
};

let predictive_systems = TechNode {
    id:                 predictive_systems_id,
    name:               "Predictive Systems",
    cost:               2500,
    prerequisites:      vec![telecommunications_id],
    effects:            vec![],
    eureka_description: "Build a Broadcast Center.",
    eureka_effects:     vec![],
};

let offworld_mission = TechNode {
    id:                 offworld_mission_id,
    name:               "Offworld Mission",
    cost:               2500,
    prerequisites:      vec![robotics_id, nuclear_fusion_id],
    effects:            vec![],
    eureka_description: "Launch an Exoplanet Expedition.",
    eureka_effects:     vec![],
};

let future_tech = TechNode {
    id:                 future_tech_id,
    name:               "Future Tech",
    cost:               2500,
    prerequisites:      vec![satellites_id, robotics_id, nanotechnology_id, nuclear_fusion_id],
    effects:            vec![],
    eureka_description: "",
    eureka_effects:     vec![],
};

// Sentinel tech: self-referential prerequisite means prerequisites_met() is always false.
// Used by improvements not yet tied to a real tech (required_tech: Some(tech_refs.unreachable)).
let unreachable = TechNode {
    id:                 unreachable_id,
    name:               "Unreachable",
    cost:               u32::MAX,
    prerequisites:      vec![unreachable_id],
    effects:            vec![],
    eureka_description: "",
    eureka_effects:     vec![],
};

// ── Add all nodes to the tree ─────────────────────────────────────────────────

// Ancient
tree.add_node(pottery);
tree.add_node(animal);
tree.add_node(mining);
tree.add_node(sailing);
tree.add_node(archery);
tree.add_node(astrology);
tree.add_node(writing);
tree.add_node(irrigation);
tree.add_node(bronze_working);
tree.add_node(the_wheel);
tree.add_node(masonry);

// Classical
tree.add_node(celestial_navigation);
tree.add_node(currency);
tree.add_node(horseback_riding);
tree.add_node(iron_working);
tree.add_node(shipbuilding);
tree.add_node(mathematics);
tree.add_node(construction);
tree.add_node(engineering);

// Medieval
tree.add_node(military_tactics);
tree.add_node(apprenticeship);
tree.add_node(machinery);
tree.add_node(education);
tree.add_node(stirrups);
tree.add_node(military_engineering);
tree.add_node(castles);

// Renaissance
tree.add_node(cartography);
tree.add_node(mass_production);
tree.add_node(banking);
tree.add_node(gunpowder);
tree.add_node(printing);
tree.add_node(square_rigging);
tree.add_node(astronomy);
tree.add_node(metal_casting);
tree.add_node(siege_tactics);

// Industrial
tree.add_node(industrialization);
tree.add_node(scientific_theory);
tree.add_node(ballistics);
tree.add_node(military_science);
tree.add_node(steam_power);
tree.add_node(sanitation);
tree.add_node(economics);
tree.add_node(rifling);

// Modern
tree.add_node(flight);
tree.add_node(replaceable_parts);
tree.add_node(steel);
tree.add_node(electricity);
tree.add_node(radio);
tree.add_node(chemistry);
tree.add_node(combustion);

// Atomic
tree.add_node(advanced_flight);
tree.add_node(rocketry);
tree.add_node(advanced_ballistics);
tree.add_node(combined_arms);
tree.add_node(plastics);
tree.add_node(computers);
tree.add_node(nuclear_fission);
tree.add_node(synthetic_materials);

// Information
tree.add_node(telecommunications);
tree.add_node(satellites);
tree.add_node(guidance_systems);
tree.add_node(lasers);
tree.add_node(composites);
tree.add_node(stealth_technology);
tree.add_node(robotics);
tree.add_node(nanotechnology);
tree.add_node(nuclear_fusion);

// Future (Gathering Storm)
tree.add_node(seasteads);
tree.add_node(advanced_ai);
tree.add_node(advanced_power_cells);
tree.add_node(cybernetics);
tree.add_node(smart_materials);
tree.add_node(predictive_systems);
tree.add_node(offworld_mission);
tree.add_node(future_tech);

// Sentinel
tree.add_node(unreachable);

// ── Return named ID handles ───────────────────────────────────────────────────

TechRefs {
    pottery:              pottery_id,
    animal_husbandry:     animal_id,
    mining:               mining_id,
    sailing:              sailing_id,
    archery:              archery_id,
    astrology:            astrology_id,
    writing:              writing_id,
    irrigation:           irrigation_id,
    bronze_working:       bronze_working_id,
    the_wheel:            the_wheel_id,
    masonry:              masonry_id,
    celestial_navigation: celestial_navigation_id,
    currency:             currency_id,
    horseback_riding:     horseback_riding_id,
    iron_working:         iron_working_id,
    shipbuilding:         shipbuilding_id,
    mathematics:          mathematics_id,
    construction:         construction_id,
    engineering:          engineering_id,
    military_tactics:     military_tactics_id,
    apprenticeship:       apprenticeship_id,
    machinery:            machinery_id,
    education:            education_id,
    stirrups:             stirrups_id,
    military_engineering: military_engineering_id,
    castles:              castles_id,
    cartography:          cartography_id,
    mass_production:      mass_production_id,
    banking:              banking_id,
    gunpowder:            gunpowder_id,
    printing:             printing_id,
    square_rigging:       square_rigging_id,
    astronomy:            astronomy_id,
    metal_casting:        metal_casting_id,
    siege_tactics:        siege_tactics_id,
    industrialization:    industrialization_id,
    scientific_theory:    scientific_theory_id,
    ballistics:           ballistics_id,
    military_science:     military_science_id,
    steam_power:          steam_power_id,
    sanitation:           sanitation_id,
    economics:            economics_id,
    rifling:              rifling_id,
    flight:               flight_id,
    replaceable_parts:    replaceable_parts_id,
    steel:                steel_id,
    electricity:          electricity_id,
    radio:                radio_id,
    chemistry:            chemistry_id,
    combustion:           combustion_id,
    advanced_flight:      advanced_flight_id,
    rocketry:             rocketry_id,
    advanced_ballistics:  advanced_ballistics_id,
    combined_arms:        combined_arms_id,
    plastics:             plastics_id,
    computers:            computers_id,
    nuclear_fission:      nuclear_fission_id,
    synthetic_materials:  synthetic_materials_id,
    telecommunications:   telecommunications_id,
    satellites:           satellites_id,
    guidance_systems:     guidance_systems_id,
    lasers:               lasers_id,
    composites:           composites_id,
    stealth_technology:   stealth_technology_id,
    robotics:             robotics_id,
    nanotechnology:       nanotechnology_id,
    nuclear_fusion:       nuclear_fusion_id,
    future_tech:          future_tech_id,
    unreachable:          unreachable_id,
}
}
