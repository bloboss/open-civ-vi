// Full tech tree definition (Ancient → Information era).
// Included verbatim inside `build_tech_tree`; `tree`, `ids`, `TechRefs`, and `OneShotEffect::*`
// are all in scope from the enclosing function. This file must be a single block expression
// that evaluates to `TechRefs`.
{
// ── Generate IDs in a fixed order (never reorder) ────────────────────────────
// Ancient Era
let pottery_id        = TechId::from_ulid(ids.next_ulid());
let animal_id         = TechId::from_ulid(ids.next_ulid());
let mining_id         = TechId::from_ulid(ids.next_ulid());
let sailing_id        = TechId::from_ulid(ids.next_ulid());
let archery_id        = TechId::from_ulid(ids.next_ulid());
let astrology_id      = TechId::from_ulid(ids.next_ulid());
let writing_id        = TechId::from_ulid(ids.next_ulid());
let irrigation_id     = TechId::from_ulid(ids.next_ulid());
let bronze_working_id = TechId::from_ulid(ids.next_ulid());
let the_wheel_id      = TechId::from_ulid(ids.next_ulid());
let masonry_id        = TechId::from_ulid(ids.next_ulid());
let unreachable_id    = TechId::from_ulid(ids.next_ulid());
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
let stirrups_id             = TechId::from_ulid(ids.next_ulid());
let machinery_id            = TechId::from_ulid(ids.next_ulid());
let education_id            = TechId::from_ulid(ids.next_ulid());
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
let nuclear_fusion_id       = TechId::from_ulid(ids.next_ulid());
let nanotechnology_id       = TechId::from_ulid(ids.next_ulid());
let future_tech_id          = TechId::from_ulid(ids.next_ulid());

// ══════════════════════════════════════════════════════════════════════════════
// ── Ancient Era node definitions ─────────────────────────────────────────────
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
    prerequisites:      vec![],
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

let writing = TechNode {
    id:                 writing_id,
    name:               "Writing",
    cost:               50,
    prerequisites:      vec![pottery_id],
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

// Sentinel tech: self-referential prerequisite means prerequisites_met() is always false.
let unreachable = TechNode {
    id:                 unreachable_id,
    name:               "Unreachable",
    cost:               u32::MAX,
    prerequisites:      vec![unreachable_id],
    effects:            vec![],
    eureka_description: "",
    eureka_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// ── Classical Era ────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

let celestial_navigation = TechNode {
    id: celestial_navigation_id, name: "Celestial Navigation", cost: 120,
    prerequisites: vec![sailing_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let currency = TechNode {
    id: currency_id, name: "Currency", cost: 120,
    prerequisites: vec![writing_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let horseback_riding = TechNode {
    id: horseback_riding_id, name: "Horseback Riding", cost: 120,
    prerequisites: vec![animal_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let iron_working = TechNode {
    id: iron_working_id, name: "Iron Working", cost: 120,
    prerequisites: vec![bronze_working_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let shipbuilding = TechNode {
    id: shipbuilding_id, name: "Shipbuilding", cost: 200,
    prerequisites: vec![sailing_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let mathematics = TechNode {
    id: mathematics_id, name: "Mathematics", cost: 200,
    prerequisites: vec![writing_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let construction = TechNode {
    id: construction_id, name: "Construction", cost: 200,
    prerequisites: vec![masonry_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let engineering = TechNode {
    id: engineering_id, name: "Engineering", cost: 200,
    prerequisites: vec![the_wheel_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// ── Medieval Era ─────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

let military_tactics = TechNode {
    id: military_tactics_id, name: "Military Tactics", cost: 300,
    prerequisites: vec![construction_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let apprenticeship = TechNode {
    id: apprenticeship_id, name: "Apprenticeship", cost: 300,
    prerequisites: vec![mining_id, currency_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let stirrups = TechNode {
    id: stirrups_id, name: "Stirrups", cost: 390,
    prerequisites: vec![horseback_riding_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let machinery = TechNode {
    id: machinery_id, name: "Machinery", cost: 300,
    prerequisites: vec![archery_id, engineering_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let education = TechNode {
    id: education_id, name: "Education", cost: 390,
    prerequisites: vec![mathematics_id, apprenticeship_id],
    effects: vec![UnlockBuilding("University")],
    eureka_description: "", eureka_effects: vec![],
};

let military_engineering = TechNode {
    id: military_engineering_id, name: "Military Engineering", cost: 390,
    prerequisites: vec![engineering_id, construction_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let castles = TechNode {
    id: castles_id, name: "Castles", cost: 390,
    prerequisites: vec![construction_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// ── Renaissance Era ──────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

let cartography = TechNode {
    id: cartography_id, name: "Cartography", cost: 540,
    prerequisites: vec![shipbuilding_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let mass_production = TechNode {
    id: mass_production_id, name: "Mass Production", cost: 540,
    prerequisites: vec![apprenticeship_id, shipbuilding_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let banking = TechNode {
    id: banking_id, name: "Banking", cost: 540,
    prerequisites: vec![currency_id, education_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let gunpowder = TechNode {
    id: gunpowder_id, name: "Gunpowder", cost: 540,
    prerequisites: vec![military_tactics_id, apprenticeship_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let printing = TechNode {
    id: printing_id, name: "Printing", cost: 540,
    prerequisites: vec![education_id, machinery_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let square_rigging = TechNode {
    id: square_rigging_id, name: "Square Rigging", cost: 660,
    prerequisites: vec![shipbuilding_id, cartography_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let astronomy = TechNode {
    id: astronomy_id, name: "Astronomy", cost: 660,
    prerequisites: vec![education_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let metal_casting = TechNode {
    id: metal_casting_id, name: "Metal Casting", cost: 660,
    prerequisites: vec![apprenticeship_id, gunpowder_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let siege_tactics = TechNode {
    id: siege_tactics_id, name: "Siege Tactics", cost: 660,
    prerequisites: vec![military_engineering_id, castles_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// ── Industrial Era ───────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

let industrialization = TechNode {
    id: industrialization_id, name: "Industrialization", cost: 805,
    prerequisites: vec![mass_production_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let scientific_theory = TechNode {
    id: scientific_theory_id, name: "Scientific Theory", cost: 805,
    prerequisites: vec![astronomy_id, banking_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let ballistics = TechNode {
    id: ballistics_id, name: "Ballistics", cost: 805,
    prerequisites: vec![metal_casting_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let military_science = TechNode {
    id: military_science_id, name: "Military Science", cost: 805,
    prerequisites: vec![stirrups_id, printing_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let steam_power = TechNode {
    id: steam_power_id, name: "Steam Power", cost: 925,
    prerequisites: vec![square_rigging_id, industrialization_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let sanitation = TechNode {
    id: sanitation_id, name: "Sanitation", cost: 925,
    prerequisites: vec![scientific_theory_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let economics = TechNode {
    id: economics_id, name: "Economics", cost: 925,
    prerequisites: vec![banking_id, scientific_theory_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let rifling = TechNode {
    id: rifling_id, name: "Rifling", cost: 925,
    prerequisites: vec![gunpowder_id, ballistics_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// ── Modern Era ───────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

let flight = TechNode {
    id: flight_id, name: "Flight", cost: 1035,
    prerequisites: vec![steam_power_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let replaceable_parts = TechNode {
    id: replaceable_parts_id, name: "Replaceable Parts", cost: 1035,
    prerequisites: vec![rifling_id, economics_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let steel = TechNode {
    id: steel_id, name: "Steel", cost: 1035,
    prerequisites: vec![rifling_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let electricity = TechNode {
    id: electricity_id, name: "Electricity", cost: 1135,
    prerequisites: vec![scientific_theory_id, steam_power_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let radio = TechNode {
    id: radio_id, name: "Radio", cost: 1135,
    prerequisites: vec![electricity_id, flight_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let chemistry = TechNode {
    id: chemistry_id, name: "Chemistry", cost: 1135,
    prerequisites: vec![scientific_theory_id, sanitation_id],
    effects: vec![UnlockBuilding("Research Lab")],
    eureka_description: "", eureka_effects: vec![],
};

let combustion = TechNode {
    id: combustion_id, name: "Combustion", cost: 1135,
    prerequisites: vec![steel_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// ── Atomic Era ───────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

let advanced_flight = TechNode {
    id: advanced_flight_id, name: "Advanced Flight", cost: 1225,
    prerequisites: vec![flight_id, radio_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let rocketry = TechNode {
    id: rocketry_id, name: "Rocketry", cost: 1225,
    prerequisites: vec![chemistry_id, radio_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let advanced_ballistics = TechNode {
    id: advanced_ballistics_id, name: "Advanced Ballistics", cost: 1225,
    prerequisites: vec![ballistics_id, replaceable_parts_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let combined_arms = TechNode {
    id: combined_arms_id, name: "Combined Arms", cost: 1225,
    prerequisites: vec![combustion_id, steel_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let plastics = TechNode {
    id: plastics_id, name: "Plastics", cost: 1225,
    prerequisites: vec![electricity_id, combustion_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let computers = TechNode {
    id: computers_id, name: "Computers", cost: 1375,
    prerequisites: vec![electricity_id, radio_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let nuclear_fission = TechNode {
    id: nuclear_fission_id, name: "Nuclear Fission", cost: 1375,
    prerequisites: vec![chemistry_id, combined_arms_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let synthetic_materials = TechNode {
    id: synthetic_materials_id, name: "Synthetic Materials", cost: 1375,
    prerequisites: vec![combustion_id, plastics_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// ── Information Era ──────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

let telecommunications = TechNode {
    id: telecommunications_id, name: "Telecommunications", cost: 1540,
    prerequisites: vec![computers_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let satellites = TechNode {
    id: satellites_id, name: "Satellites", cost: 1540,
    prerequisites: vec![rocketry_id, advanced_flight_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let guidance_systems = TechNode {
    id: guidance_systems_id, name: "Guidance Systems", cost: 1540,
    prerequisites: vec![advanced_ballistics_id, rocketry_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let lasers = TechNode {
    id: lasers_id, name: "Lasers", cost: 1540,
    prerequisites: vec![advanced_flight_id, nuclear_fission_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let composites = TechNode {
    id: composites_id, name: "Composites", cost: 1540,
    prerequisites: vec![synthetic_materials_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let stealth_technology = TechNode {
    id: stealth_technology_id, name: "Stealth Technology", cost: 1540,
    prerequisites: vec![lasers_id, synthetic_materials_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let robotics = TechNode {
    id: robotics_id, name: "Robotics", cost: 1795,
    prerequisites: vec![satellites_id, computers_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let nuclear_fusion = TechNode {
    id: nuclear_fusion_id, name: "Nuclear Fusion", cost: 1795,
    prerequisites: vec![nuclear_fission_id, lasers_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let nanotechnology = TechNode {
    id: nanotechnology_id, name: "Nanotechnology", cost: 1795,
    prerequisites: vec![synthetic_materials_id, composites_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
};

let future_tech = TechNode {
    id: future_tech_id, name: "Future Tech", cost: 2050,
    prerequisites: vec![nanotechnology_id, robotics_id, nuclear_fusion_id],
    effects: vec![], eureka_description: "", eureka_effects: vec![],
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
tree.add_node(unreachable);
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
tree.add_node(stirrups);
tree.add_node(machinery);
tree.add_node(education);
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
tree.add_node(nuclear_fusion);
tree.add_node(nanotechnology);
tree.add_node(future_tech);

// ── Return named ID handles ───────────────────────────────────────────────────

TechRefs {
    // Ancient
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
    // Classical
    celestial_navigation: celestial_navigation_id,
    currency:             currency_id,
    horseback_riding:     horseback_riding_id,
    iron_working:         iron_working_id,
    shipbuilding:         shipbuilding_id,
    mathematics:          mathematics_id,
    construction:         construction_id,
    engineering:          engineering_id,
    // Medieval
    military_tactics:     military_tactics_id,
    apprenticeship:       apprenticeship_id,
    stirrups:             stirrups_id,
    machinery:            machinery_id,
    education:            education_id,
    military_engineering: military_engineering_id,
    castles:              castles_id,
    // Renaissance
    cartography:          cartography_id,
    mass_production:      mass_production_id,
    banking:              banking_id,
    gunpowder:            gunpowder_id,
    printing:             printing_id,
    square_rigging:       square_rigging_id,
    astronomy:            astronomy_id,
    metal_casting:        metal_casting_id,
    siege_tactics:        siege_tactics_id,
    // Industrial
    industrialization:    industrialization_id,
    scientific_theory:    scientific_theory_id,
    ballistics:           ballistics_id,
    military_science:     military_science_id,
    steam_power:          steam_power_id,
    sanitation:           sanitation_id,
    economics:            economics_id,
    rifling:              rifling_id,
    // Modern
    flight:               flight_id,
    replaceable_parts:    replaceable_parts_id,
    steel:                steel_id,
    electricity:          electricity_id,
    radio:                radio_id,
    chemistry:            chemistry_id,
    combustion:           combustion_id,
    // Atomic
    advanced_flight:      advanced_flight_id,
    rocketry:             rocketry_id,
    advanced_ballistics:  advanced_ballistics_id,
    combined_arms:        combined_arms_id,
    plastics:             plastics_id,
    computers:            computers_id,
    nuclear_fission:      nuclear_fission_id,
    synthetic_materials:  synthetic_materials_id,
    // Information
    telecommunications:   telecommunications_id,
    satellites:           satellites_id,
    guidance_systems:     guidance_systems_id,
    lasers:               lasers_id,
    composites:           composites_id,
    stealth_technology:   stealth_technology_id,
    robotics:             robotics_id,
    nuclear_fusion:       nuclear_fusion_id,
    nanotechnology:       nanotechnology_id,
    future_tech:          future_tech_id,
    unreachable:          unreachable_id,
}
}
