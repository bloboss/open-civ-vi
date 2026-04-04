// Built-in belief definitions.
// Included verbatim inside `build_beliefs`; `ids`, `beliefs`, `BeliefRefs`,
// `BuiltinBelief`, `BeliefCategory`, `Modifier`, `ModifierSource`, `TargetSelector`,
// `EffectType`, `StackingRule`, `YieldType`, and `Condition` are all in scope
// from the enclosing function. This file must be a single block expression
// that evaluates to `BeliefRefs`.
{
// ── Generate IDs in a fixed order (never reorder) ────────────────────────────
// Existing IDs (do not reorder):
let church_property_id     = BeliefId::from_ulid(ids.next_ulid());
let tithe_id               = BeliefId::from_ulid(ids.next_ulid());
let papal_primacy_id       = BeliefId::from_ulid(ids.next_ulid());
let religious_unity_id     = BeliefId::from_ulid(ids.next_ulid());
let divine_inspiration_id  = BeliefId::from_ulid(ids.next_ulid());
let choral_music_id        = BeliefId::from_ulid(ids.next_ulid());
let religious_community_id = BeliefId::from_ulid(ids.next_ulid());
let feed_the_world_id      = BeliefId::from_ulid(ids.next_ulid());
let cathedral_id           = BeliefId::from_ulid(ids.next_ulid());
let gurdwara_id            = BeliefId::from_ulid(ids.next_ulid());
let mosque_id              = BeliefId::from_ulid(ids.next_ulid());
let pagoda_id              = BeliefId::from_ulid(ids.next_ulid());
let synagogue_id           = BeliefId::from_ulid(ids.next_ulid());
let wat_id                 = BeliefId::from_ulid(ids.next_ulid());
let missionary_zeal_id     = BeliefId::from_ulid(ids.next_ulid());
let holy_order_id          = BeliefId::from_ulid(ids.next_ulid());
let itinerant_preachers_id = BeliefId::from_ulid(ids.next_ulid());
let scripture_id           = BeliefId::from_ulid(ids.next_ulid());
// Missing worship building IDs (appended, never reorder above):
let meeting_house_id                = BeliefId::from_ulid(ids.next_ulid());
let stupa_id                        = BeliefId::from_ulid(ids.next_ulid());
let dar_e_mehr_id                   = BeliefId::from_ulid(ids.next_ulid());
// Pantheon belief IDs (appended, never reorder above):
let dance_of_the_aurora_id          = BeliefId::from_ulid(ids.next_ulid());
let desert_folklore_id              = BeliefId::from_ulid(ids.next_ulid());
let sacred_path_id                  = BeliefId::from_ulid(ids.next_ulid());
let stone_circles_id                = BeliefId::from_ulid(ids.next_ulid());
let religious_idols_id              = BeliefId::from_ulid(ids.next_ulid());
let earth_goddess_id                = BeliefId::from_ulid(ids.next_ulid());
let god_of_the_sea_id               = BeliefId::from_ulid(ids.next_ulid());
let god_of_the_forge_id             = BeliefId::from_ulid(ids.next_ulid());
let divine_spark_id                 = BeliefId::from_ulid(ids.next_ulid());
let lady_of_the_reeds_and_marshes_id = BeliefId::from_ulid(ids.next_ulid());
let oral_tradition_id               = BeliefId::from_ulid(ids.next_ulid());
let monument_to_the_gods_id         = BeliefId::from_ulid(ids.next_ulid());
let river_goddess_id                = BeliefId::from_ulid(ids.next_ulid());
let city_patron_goddess_id          = BeliefId::from_ulid(ids.next_ulid());
let fertility_rites_id              = BeliefId::from_ulid(ids.next_ulid());
let god_of_war_id                   = BeliefId::from_ulid(ids.next_ulid());
let god_of_craftsmen_id             = BeliefId::from_ulid(ids.next_ulid());
let initiation_rites_id             = BeliefId::from_ulid(ids.next_ulid());
let religious_settlements_id        = BeliefId::from_ulid(ids.next_ulid());
let goddess_of_the_hunt_id          = BeliefId::from_ulid(ids.next_ulid());

// ── Pantheon beliefs ─────────────────────────────────────────────────────────

beliefs.push(BuiltinBelief {
    id: dance_of_the_aurora_id,
    name: "Dance of the Aurora",
    description: "Holy Site districts get +1 Faith from each adjacent Tundra tile.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Dance of the Aurora"),
            TargetSelector::DistrictAdjacency(BuiltinDistrict::HolySite),
            EffectType::YieldFlat(YieldType::Faith, 1),
            StackingRule::Additive,
        ).with_condition(Condition::PerAdjacentTerrain(BuiltinTerrain::Tundra)),
    ],
});

beliefs.push(BuiltinBelief {
    id: desert_folklore_id,
    name: "Desert Folklore",
    description: "Holy Site districts get +1 Faith from each adjacent Desert tile.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Desert Folklore"),
            TargetSelector::DistrictAdjacency(BuiltinDistrict::HolySite),
            EffectType::YieldFlat(YieldType::Faith, 1),
            StackingRule::Additive,
        ).with_condition(Condition::PerAdjacentTerrain(BuiltinTerrain::Desert)),
    ],
});

beliefs.push(BuiltinBelief {
    id: sacred_path_id,
    name: "Sacred Path",
    description: "Holy Site districts get +1 Faith from each adjacent Rainforest tile.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Sacred Path"),
            TargetSelector::DistrictAdjacency(BuiltinDistrict::HolySite),
            EffectType::YieldFlat(YieldType::Faith, 1),
            StackingRule::Additive,
        ).with_condition(Condition::PerAdjacentFeature(BuiltinFeature::Rainforest)),
    ],
});

beliefs.push(BuiltinBelief {
    id: stone_circles_id,
    name: "Stone Circles",
    description: "+2 Faith from Quarries.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Stone Circles"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Faith, 2),
            StackingRule::Additive,
        ).with_condition(Condition::TileHasImprovement(BuiltinImprovement::Quarry)),
    ],
});

beliefs.push(BuiltinBelief {
    id: religious_idols_id,
    name: "Religious Idols",
    description: "+2 Faith from Mines over luxury and bonus resources.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Religious Idols"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Faith, 2),
            StackingRule::Additive,
        ).with_condition(Condition::And(
            Box::new(Condition::TileHasImprovement(BuiltinImprovement::Mine)),
            Box::new(Condition::Or(
                Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Luxury)),
                Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Bonus)),
            )),
        )),
    ],
});

beliefs.push(BuiltinBelief {
    id: earth_goddess_id,
    name: "Earth Goddess",
    description: "+1 Faith from tiles with Charming or better Appeal.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Earth Goddess"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Faith, 1),
            StackingRule::Additive,
        ).with_condition(Condition::TileMinAppeal(2)), // Charming = 2+
    ],
});

beliefs.push(BuiltinBelief {
    id: god_of_the_sea_id,
    name: "God of the Sea",
    description: "+1 Production from Fishing Boats.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("God of the Sea"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Production, 1),
            StackingRule::Additive,
        ).with_condition(Condition::TileHasImprovement(BuiltinImprovement::FishingBoats)),
    ],
});

beliefs.push(BuiltinBelief {
    id: god_of_the_forge_id,
    name: "God of the Forge",
    description: "+25% Production toward Ancient and Classical military units.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("God of the Forge"),
            TargetSelector::ProductionQueue,
            EffectType::ProductionPercent(25),
            StackingRule::Additive,
        ).with_condition(Condition::Or(
            Box::new(Condition::ProducingMilitaryUnitOfEra(crate::AgeType::Ancient)),
            Box::new(Condition::ProducingMilitaryUnitOfEra(crate::AgeType::Classical)),
        )),
    ],
});

beliefs.push(BuiltinBelief {
    id: divine_spark_id,
    name: "Divine Spark",
    description: "+1 Great Person points from Holy Site, Campus, and Theater Square districts.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![], // Requires Great Person point system integration.
});

beliefs.push(BuiltinBelief {
    id: lady_of_the_reeds_and_marshes_id,
    name: "Lady of the Reeds and Marshes",
    description: "+2 Production from Marsh, Oasis, and Desert Floodplains.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Lady of the Reeds and Marshes"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Production, 2),
            StackingRule::Additive,
        ).with_condition(Condition::TileHasFeature(BuiltinFeature::Marsh)),
        Modifier::new(
            ModifierSource::Religion("Lady of the Reeds and Marshes"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Production, 2),
            StackingRule::Additive,
        ).with_condition(Condition::TileHasFeature(BuiltinFeature::Oasis)),
        Modifier::new(
            ModifierSource::Religion("Lady of the Reeds and Marshes"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Production, 2),
            StackingRule::Additive,
        ).with_condition(Condition::TileHasFeature(BuiltinFeature::Floodplain)),
    ],
});

beliefs.push(BuiltinBelief {
    id: oral_tradition_id,
    name: "Oral Tradition",
    description: "+1 Culture from Plantations.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Oral Tradition"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Culture, 1),
            StackingRule::Additive,
        ).with_condition(Condition::TileHasImprovement(BuiltinImprovement::Plantation)),
    ],
});

beliefs.push(BuiltinBelief {
    id: monument_to_the_gods_id,
    name: "Monument to the Gods",
    description: "+15% Production toward Ancient and Classical era wonders.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Monument to the Gods"),
            TargetSelector::ProductionQueue,
            EffectType::ProductionPercent(15),
            StackingRule::Additive,
        ).with_condition(Condition::Or(
            Box::new(Condition::ProducingWonderOfEra(crate::AgeType::Ancient)),
            Box::new(Condition::ProducingWonderOfEra(crate::AgeType::Classical)),
        )),
    ],
});

beliefs.push(BuiltinBelief {
    id: river_goddess_id,
    name: "River Goddess",
    description: "+2 Amenities and +2 Housing to Holy Site districts adjacent to a River.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("River Goddess"),
            TargetSelector::Global,
            EffectType::AmenityFlat(2),
            StackingRule::Additive,
        ).with_condition(Condition::AdjacentToRiver),
        Modifier::new(
            ModifierSource::Religion("River Goddess"),
            TargetSelector::Global,
            EffectType::HousingFlat(2),
            StackingRule::Additive,
        ).with_condition(Condition::AdjacentToRiver),
    ],
});

beliefs.push(BuiltinBelief {
    id: city_patron_goddess_id,
    name: "City Patron Goddess",
    description: "+25% Production toward districts in cities with no specialty districts.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("City Patron Goddess"),
            TargetSelector::ProductionQueue,
            EffectType::ProductionPercent(25),
            StackingRule::Additive,
        ).with_condition(Condition::ProducingDistrictOrWonder),
    ],
});

beliefs.push(BuiltinBelief {
    id: fertility_rites_id,
    name: "Fertility Rites",
    description: "City growth rate increased by 10%.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Fertility Rites"),
            TargetSelector::Global,
            EffectType::YieldPercent(YieldType::Food, 10),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: god_of_war_id,
    name: "God of War",
    description: "Bonus Faith equal to 50% of the combat strength of defeated units within 8 tiles of a Holy Site.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![], // Requires combat event integration.
});

beliefs.push(BuiltinBelief {
    id: god_of_craftsmen_id,
    name: "God of Craftsmen",
    description: "+1 Production and +1 Faith from improved strategic resources.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("God of Craftsmen"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Production, 1),
            StackingRule::Additive,
        ).with_condition(Condition::And(
            Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Strategic)),
            Box::new(Condition::TileHasAnyImprovement),
        )),
        Modifier::new(
            ModifierSource::Religion("God of Craftsmen"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Faith, 1),
            StackingRule::Additive,
        ).with_condition(Condition::And(
            Box::new(Condition::TileHasResourceOfCategory(ResourceCategory::Strategic)),
            Box::new(Condition::TileHasAnyImprovement),
        )),
    ],
});

beliefs.push(BuiltinBelief {
    id: initiation_rites_id,
    name: "Initiation Rites",
    description: "+50 Faith for each Barbarian Outpost cleared.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![], // Requires barbarian event integration.
});

beliefs.push(BuiltinBelief {
    id: religious_settlements_id,
    name: "Religious Settlements",
    description: "Border expansion rate increased by 15%.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![], // Requires border expansion system integration.
});

beliefs.push(BuiltinBelief {
    id: goddess_of_the_hunt_id,
    name: "Goddess of the Hunt",
    description: "+1 Food from Camps.",
    category: BeliefCategory::Pantheon,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Goddess of the Hunt"),
            TargetSelector::AllTiles,
            EffectType::YieldFlat(YieldType::Food, 1),
            StackingRule::Additive,
        ).with_condition(Condition::TileHasImprovement(BuiltinImprovement::Camp)),
    ],
});

// ── Founder beliefs ──────────────────────────────────────────────────────────

beliefs.push(BuiltinBelief {
    id: church_property_id,
    name: "Church Property",
    description: "+2 Gold for each city following this religion.",
    category: BeliefCategory::Founder,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Church Property"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Gold, 2),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: tithe_id,
    name: "Tithe",
    description: "+1 Gold for every 4 followers of this religion.",
    category: BeliefCategory::Founder,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Tithe"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Gold, 1),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: papal_primacy_id,
    name: "Papal Primacy",
    description: "+2 Faith for each city-state following this religion.",
    category: BeliefCategory::Founder,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Papal Primacy"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 2),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: religious_unity_id,
    name: "Religious Unity",
    description: "+1 Era Score for each new city converted to this religion.",
    category: BeliefCategory::Founder,
    modifiers: vec![],
});

// ── Follower beliefs ─────────────────────────────────────────────────────────

beliefs.push(BuiltinBelief {
    id: divine_inspiration_id,
    name: "Divine Inspiration",
    description: "+4 Faith from world wonders.",
    category: BeliefCategory::Follower,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Divine Inspiration"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 4),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: choral_music_id,
    name: "Choral Music",
    description: "+2 Culture from Shrines and Temples.",
    category: BeliefCategory::Follower,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Choral Music"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Culture, 2),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: religious_community_id,
    name: "Religious Community",
    description: "+1% Production per follower (max 15%).",
    category: BeliefCategory::Follower,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Religious Community"),
            TargetSelector::Global,
            EffectType::ProductionPercent(1),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: feed_the_world_id,
    name: "Feed the World",
    description: "+3 Food and +1 Housing from Shrines and Temples.",
    category: BeliefCategory::Follower,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Feed the World"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Food, 3),
            StackingRule::Additive,
        ),
        Modifier::new(
            ModifierSource::Religion("Feed the World"),
            TargetSelector::Global,
            EffectType::HousingFlat(1),
            StackingRule::Additive,
        ),
    ],
});

// ── Worship beliefs ──────────────────────────────────────────────────────────

beliefs.push(BuiltinBelief {
    id: cathedral_id,
    name: "Cathedral",
    description: "Allows Cathedrals (+4 Faith, +1 Great Work of Religious Art slot).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Cathedral"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 4),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: gurdwara_id,
    name: "Gurdwara",
    description: "Allows Gurdwaras (+3 Faith, +2 Food, +1 Housing).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Gurdwara"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 3),
            StackingRule::Additive,
        ),
        Modifier::new(
            ModifierSource::Religion("Gurdwara"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Food, 2),
            StackingRule::Additive,
        ),
        Modifier::new(
            ModifierSource::Religion("Gurdwara"),
            TargetSelector::Global,
            EffectType::HousingFlat(1),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: mosque_id,
    name: "Mosque",
    description: "Allows Mosques (+3 Faith, +1 Missionary spread charge).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Mosque"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 3),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: pagoda_id,
    name: "Pagoda",
    description: "Allows Pagodas (+3 Faith, +1 Housing).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Pagoda"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 3),
            StackingRule::Additive,
        ),
        Modifier::new(
            ModifierSource::Religion("Pagoda"),
            TargetSelector::Global,
            EffectType::HousingFlat(1),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: synagogue_id,
    name: "Synagogue",
    description: "Allows Synagogues (+5 Faith).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Synagogue"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 5),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: wat_id,
    name: "Wat",
    description: "Allows Wats (+3 Faith, +2 Science).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Wat"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 3),
            StackingRule::Additive,
        ),
        Modifier::new(
            ModifierSource::Religion("Wat"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Science, 2),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: meeting_house_id,
    name: "Meeting House",
    description: "Allows Meeting Houses (+3 Faith, +2 Production).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Meeting House"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 3),
            StackingRule::Additive,
        ),
        Modifier::new(
            ModifierSource::Religion("Meeting House"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Production, 2),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: stupa_id,
    name: "Stupa",
    description: "Allows Stupas (+3 Faith, +1 Amenity).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Stupa"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 3),
            StackingRule::Additive,
        ),
        Modifier::new(
            ModifierSource::Religion("Stupa"),
            TargetSelector::Global,
            EffectType::AmenityFlat(1),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: dar_e_mehr_id,
    name: "Dar-e Mehr",
    description: "Allows Dar-e Mehrs (+3 Faith, +1 Faith per era since construction).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Dar-e Mehr"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 3),
            StackingRule::Additive,
        ),
    ],
});

// ── Enhancer beliefs ─────────────────────────────────────────────────────────

beliefs.push(BuiltinBelief {
    id: missionary_zeal_id,
    name: "Missionary Zeal",
    description: "Missionaries gain +2 spread charges.",
    category: BeliefCategory::Enhancer,
    modifiers: vec![],
});

beliefs.push(BuiltinBelief {
    id: holy_order_id,
    name: "Holy Order",
    description: "Missionaries and Apostles are 30% cheaper to purchase with Faith.",
    category: BeliefCategory::Enhancer,
    modifiers: vec![],
});

beliefs.push(BuiltinBelief {
    id: itinerant_preachers_id,
    name: "Itinerant Preachers",
    description: "Religion spreads to cities 3 tiles further away.",
    category: BeliefCategory::Enhancer,
    modifiers: vec![],
});

beliefs.push(BuiltinBelief {
    id: scripture_id,
    name: "Scripture",
    description: "+25% religious combat strength.",
    category: BeliefCategory::Enhancer,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Scripture"),
            TargetSelector::Global,
            EffectType::CombatStrengthPercent(25),
            StackingRule::Additive,
        ),
    ],
});

// ── Return named ID handles ──────────────────────────────────────────────────

BeliefRefs {
    // Pantheon beliefs
    dance_of_the_aurora:          dance_of_the_aurora_id,
    desert_folklore:              desert_folklore_id,
    sacred_path:                  sacred_path_id,
    stone_circles:                stone_circles_id,
    religious_idols:              religious_idols_id,
    earth_goddess:                earth_goddess_id,
    god_of_the_sea:               god_of_the_sea_id,
    god_of_the_forge:             god_of_the_forge_id,
    divine_spark:                 divine_spark_id,
    lady_of_the_reeds_and_marshes: lady_of_the_reeds_and_marshes_id,
    oral_tradition:               oral_tradition_id,
    monument_to_the_gods:         monument_to_the_gods_id,
    river_goddess:                river_goddess_id,
    city_patron_goddess:          city_patron_goddess_id,
    fertility_rites:              fertility_rites_id,
    god_of_war:                   god_of_war_id,
    god_of_craftsmen:             god_of_craftsmen_id,
    initiation_rites:             initiation_rites_id,
    religious_settlements:        religious_settlements_id,
    goddess_of_the_hunt:          goddess_of_the_hunt_id,
    // Founder beliefs
    church_property:     church_property_id,
    tithe:               tithe_id,
    papal_primacy:       papal_primacy_id,
    religious_unity:     religious_unity_id,
    divine_inspiration:  divine_inspiration_id,
    choral_music:        choral_music_id,
    religious_community: religious_community_id,
    feed_the_world:      feed_the_world_id,
    cathedral:           cathedral_id,
    gurdwara:            gurdwara_id,
    mosque:              mosque_id,
    pagoda:              pagoda_id,
    synagogue:           synagogue_id,
    wat:                 wat_id,
    meeting_house:       meeting_house_id,
    stupa:               stupa_id,
    dar_e_mehr:          dar_e_mehr_id,
    missionary_zeal:     missionary_zeal_id,
    holy_order:          holy_order_id,
    itinerant_preachers: itinerant_preachers_id,
    scripture:           scripture_id,
}
}
