// Built-in belief definitions.
// Included verbatim inside `build_beliefs`; `ids`, `beliefs`, `BeliefRefs`,
// `BuiltinBelief`, `BeliefCategory`, `Modifier`, `ModifierSource`, `TargetSelector`,
// `EffectType`, `StackingRule`, `YieldType`, and `Condition` are all in scope
// from the enclosing function. This file must be a single block expression
// that evaluates to `BeliefRefs`.
{
// ── Generate IDs in a fixed order (never reorder) ────────────────────────────
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
    description: "Allows Cathedrals (+3 Faith, +1 Great Work of Religious Art slot).",
    category: BeliefCategory::Worship,
    modifiers: vec![
        Modifier::new(
            ModifierSource::Religion("Cathedral"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Faith, 3),
            StackingRule::Additive,
        ),
    ],
});

beliefs.push(BuiltinBelief {
    id: gurdwara_id,
    name: "Gurdwara",
    description: "Allows Gurdwaras (+3 Faith, +2 Food).",
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
    missionary_zeal:     missionary_zeal_id,
    holy_order:          holy_order_id,
    itinerant_preachers: itinerant_preachers_id,
    scripture:           scripture_id,
}
}
