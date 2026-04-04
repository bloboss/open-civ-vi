// Ancient Era -- tech tree definition.
// Included verbatim inside `build_tech_tree`; `tree`, `ids`, `TechRefs`, and `OneShotEffect::*`
// are all in scope from the enclosing function. This file must be a single block expression
// that evaluates to `TechRefs`.
{
// ── Generate IDs in a fixed order (never reorder) ────────────────────────────
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

// ── Node definitions ──────────────────────────────────────────────────────────

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

// ── Return named ID handles ───────────────────────────────────────────────────

TechRefs {
    pottery:          pottery_id,
    animal_husbandry: animal_id,
    mining:           mining_id,
    sailing:          sailing_id,
    archery:          archery_id,
    astrology:        astrology_id,
    writing:          writing_id,
    irrigation:       irrigation_id,
    bronze_working:   bronze_working_id,
    the_wheel:        the_wheel_id,
    masonry:          masonry_id,
    unreachable:      unreachable_id,
}
}
