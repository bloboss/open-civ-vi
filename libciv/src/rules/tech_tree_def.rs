// Ancient Era -- tech tree definition.
// Included verbatim inside `build_tech_tree`; `tree`, `ids`, and `OneShotEffect::*`
// are all in scope from the enclosing function. This file must be a single block expression.
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

// ── Node definitions ──────────────────────────────────────────────────────────

tree.add_node(TechNode {
    id:                 pottery_id,
    name:               "Pottery",
    cost:               25,
    prerequisites:      vec![],
    effects:            vec![UnlockBuilding("Granary"), UnlockImprovement("Farm")],
    eureka_description: "Harvest any resource.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 animal_id,
    name:               "Animal Husbandry",
    cost:               25,
    prerequisites:      vec![],
    effects:            vec![UnlockImprovement("Pasture")],
    eureka_description: "Find a tribal village.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 mining_id,
    name:               "Mining",
    cost:               25,
    prerequisites:      vec![],
    effects:            vec![UnlockImprovement("Mine")],
    eureka_description: "Clear a Forest or Rainforest.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 sailing_id,
    name:               "Sailing",
    cost:               50,
    prerequisites:      vec![],
    effects:            vec![UnlockUnit("Galley")],
    eureka_description: "Found a city on the coast.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 archery_id,
    name:               "Archery",
    cost:               50,
    prerequisites:      vec![],
    effects:            vec![UnlockUnit("Archer")],
    eureka_description: "Kill a unit.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 astrology_id,
    name:               "Astrology",
    cost:               50,
    prerequisites:      vec![],
    effects:            vec![UnlockBuilding("Shrine")],
    eureka_description: "Find a Natural Wonder.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 writing_id,
    name:               "Writing",
    cost:               50,
    prerequisites:      vec![pottery_id],
    effects:            vec![UnlockBuilding("Library")],
    eureka_description: "Meet another civilization.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 irrigation_id,
    name:               "Irrigation",
    cost:               50,
    prerequisites:      vec![pottery_id],
    effects:            vec![UnlockImprovement("Irrigation")],
    eureka_description: "Find a Floodplain or River.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 bronze_working_id,
    name:               "Bronze Working",
    cost:               80,
    prerequisites:      vec![mining_id],
    effects:            vec![UnlockUnit("Spearman")],
    eureka_description: "Kill 3 units.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 the_wheel_id,
    name:               "The Wheel",
    cost:               80,
    prerequisites:      vec![mining_id],
    effects:            vec![UnlockUnit("Heavy Chariot")],
    eureka_description: "Build a Quarry.",
    eureka_effects:     vec![],
});

tree.add_node(TechNode {
    id:                 masonry_id,
    name:               "Masonry",
    cost:               80,
    prerequisites:      vec![mining_id],
    effects:            vec![UnlockBuilding("Walls"), UnlockImprovement("Quarry")],
    eureka_description: "Build a Quarry.",
    eureka_effects:     vec![],
});

// Sentinel tech: self-referential prerequisite means prerequisites_met() is always false.
// Used by improvements not yet tied to a real tech (required_tech: Some("Unreachable")).
let unreachable_id = TechId::from_ulid(ids.next_ulid());
tree.add_node(TechNode {
    id:                 unreachable_id,
    name:               "Unreachable",
    cost:               u32::MAX,
    prerequisites:      vec![unreachable_id],
    effects:            vec![],
    eureka_description: "",
    eureka_effects:     vec![],
});
}
