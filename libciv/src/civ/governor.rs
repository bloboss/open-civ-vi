use crate::{CityId, CivId, GovernorId, YieldType};
use crate::rules::modifier::{
    EffectType, Modifier, ModifierSource, StackingRule, TargetSelector,
};

pub trait GovernorDef: std::fmt::Debug {
    fn id(&self) -> GovernorId;
    fn name(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn base_ability_description(&self) -> &'static str;
}

pub trait GovernorPromotion: std::fmt::Debug {
    fn id(&self) -> crate::PromotionId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn requires(&self) -> Vec<crate::PromotionId>;
}

#[derive(Debug, Clone)]
pub struct Governor {
    pub id: GovernorId,
    pub def_name: &'static str,
    pub owner: CivId,
    pub assigned_city: Option<CityId>,
    pub promotions: Vec<&'static str>,
    pub turns_to_establish: u32,
}

impl Governor {
    pub fn new(id: GovernorId, def_name: &'static str, owner: CivId) -> Self {
        Self {
            id,
            def_name,
            owner,
            assigned_city: None,
            promotions: Vec::new(),
            turns_to_establish: 5,
        }
    }

    pub fn is_established(&self) -> bool {
        self.turns_to_establish == 0
    }

    pub fn has_promotion(&self, name: &str) -> bool {
        self.promotions.contains(&name)
    }
}

// ---- Seven built-in governor definitions ----

macro_rules! define_governor {
    ($name:ident, $title:literal, $ability:literal) => {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $name;

        impl GovernorDef for $name {
            fn id(&self) -> GovernorId {
                // Stable deterministic ID based on governor name.
                let hash = {
                    let bytes = stringify!($name).as_bytes();
                    let mut h: u128 = 0;
                    let mut i = 0;
                    while i < bytes.len() {
                        h = h.wrapping_mul(31).wrapping_add(bytes[i] as u128);
                        i += 1;
                    }
                    h
                };
                GovernorId::from_ulid(::ulid::Ulid::from(hash))
            }
            fn name(&self) -> &'static str { stringify!($name) }
            fn title(&self) -> &'static str { $title }
            fn base_ability_description(&self) -> &'static str { $ability }
        }
    };
}

define_governor!(
    Liang,
    "The Surveyor",
    "Guildmaster: +1 Builder charge for Builders trained in this city."
);
define_governor!(
    Magnus,
    "The Steward",
    "Groundbreaker: Settlers trained in this city do not reduce city population."
);
define_governor!(
    Amani,
    "The Diplomat",
    "Messenger: +2 Envoys when established in a city-state."
);
define_governor!(
    Victor,
    "The Castellan",
    "Redoubt: +5 Combat Strength to units within the city's territory."
);
define_governor!(
    Pingala,
    "The Educator",
    "Connoisseur: +1 Culture per turn for each Citizen in the city."
);
define_governor!(
    Reyna,
    "The Financier",
    "Land Acquisition: Tiles in the city can be purchased with Gold at a reduced cost."
);
define_governor!(
    Ibrahim,
    "The Grand Vizier",
    "Head Falconer: +1 Movement for military units trained in this city on their first turn."
);

// ---- Promotion definitions ----

/// A static promotion definition for a governor.
pub struct GovernorPromotionDef {
    pub name: &'static str,
    pub governor: &'static str,
    pub description: &'static str,
    pub tier: u8,
    pub requires: &'static [&'static str],
    pub modifiers: fn() -> Vec<Modifier>,
}

fn no_modifiers() -> Vec<Modifier> { Vec::new() }

// ── Reyna promotions ────────────────────────────────────────────────────────

fn reyna_harbormaster_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Harbormaster"),
        target: TargetSelector::Global,
        effect: EffectType::YieldFlat(YieldType::Gold, 2),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn reyna_tax_collector_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Tax Collector"),
        target: TargetSelector::Global,
        effect: EffectType::YieldFlat(YieldType::Gold, 5),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn reyna_forestry_management_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Forestry Management"),
        target: TargetSelector::Global,
        effect: EffectType::YieldFlat(YieldType::Gold, 2),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn reyna_contractor_modifiers() -> Vec<Modifier> {
    // "Purchase districts and buildings with Gold in the city" - represented
    // as a gold cost reduction. Actual purchase action is not yet implemented,
    // so we model it as a production percent boost.
    vec![Modifier {
        source: ModifierSource::Governor("Contractor"),
        target: TargetSelector::ProductionQueue,
        effect: EffectType::ProductionPercent(20),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

static REYNA_PROMOTIONS: &[GovernorPromotionDef] = &[
    GovernorPromotionDef {
        name: "Land Acquisition",
        governor: "Reyna",
        description: "Tiles in the city can be purchased with Gold at a reduced cost.",
        tier: 0,
        requires: &[],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Harbormaster",
        governor: "Reyna",
        description: "+2 Gold per turn for each improved sea resource in the city.",
        tier: 1,
        requires: &["Land Acquisition"],
        modifiers: reyna_harbormaster_modifiers,
    },
    GovernorPromotionDef {
        name: "Forestry Management",
        governor: "Reyna",
        description: "+2 Gold per turn for each unimproved feature tile in the city.",
        tier: 1,
        requires: &["Land Acquisition"],
        modifiers: reyna_forestry_management_modifiers,
    },
    GovernorPromotionDef {
        name: "Tax Collector",
        governor: "Reyna",
        description: "+2 Gold per turn for each Citizen in the city.",
        tier: 2,
        requires: &["Harbormaster", "Forestry Management"],
        modifiers: reyna_tax_collector_modifiers,
    },
    GovernorPromotionDef {
        name: "Contractor",
        governor: "Reyna",
        description: "Allows purchasing of districts and buildings with Gold in the city.",
        tier: 2,
        requires: &["Harbormaster", "Forestry Management"],
        modifiers: reyna_contractor_modifiers,
    },
    GovernorPromotionDef {
        name: "Renewable Subsidizer",
        governor: "Reyna",
        description: "+2 Gold and +1 Production for Power Plants in the city.",
        tier: 3,
        requires: &["Tax Collector", "Contractor"],
        modifiers: no_modifiers,
    },
];

// ── Magnus promotions ───────────────────────────────────────────────────────

fn magnus_surplus_logistics_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Surplus Logistics"),
        target: TargetSelector::Global,
        effect: EffectType::YieldPercent(YieldType::Food, 20),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn magnus_industrialist_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Industrialist"),
        target: TargetSelector::Global,
        effect: EffectType::YieldFlat(YieldType::Production, 3),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn magnus_black_marketeer_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Black Marketeer"),
        target: TargetSelector::TradeRoutesOwned,
        effect: EffectType::TradeRouteYieldFlat(YieldType::Gold, 4),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn magnus_vertical_integration_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Vertical Integration"),
        target: TargetSelector::ProductionQueue,
        effect: EffectType::ProductionPercent(20),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

static MAGNUS_PROMOTIONS: &[GovernorPromotionDef] = &[
    GovernorPromotionDef {
        name: "Groundbreaker",
        governor: "Magnus",
        description: "Settlers trained in the city do not consume a Citizen Population.",
        tier: 0,
        requires: &[],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Surplus Logistics",
        governor: "Magnus",
        description: "+20% Food surplus in the city.",
        tier: 1,
        requires: &["Groundbreaker"],
        modifiers: magnus_surplus_logistics_modifiers,
    },
    GovernorPromotionDef {
        name: "Provision",
        governor: "Magnus",
        description: "City is not subject to war weariness.",
        tier: 1,
        requires: &["Groundbreaker"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Industrialist",
        governor: "Magnus",
        description: "+1 Production per Mine and Quarry in the city.",
        tier: 2,
        requires: &["Surplus Logistics", "Provision"],
        modifiers: magnus_industrialist_modifiers,
    },
    GovernorPromotionDef {
        name: "Black Marketeer",
        governor: "Magnus",
        description: "+4 Gold from international Trade Routes to or from the city.",
        tier: 2,
        requires: &["Surplus Logistics", "Provision"],
        modifiers: magnus_black_marketeer_modifiers,
    },
    GovernorPromotionDef {
        name: "Vertical Integration",
        governor: "Magnus",
        description: "This city receives Production from all Industrial Zones within 6 tiles.",
        tier: 3,
        requires: &["Industrialist", "Black Marketeer"],
        modifiers: magnus_vertical_integration_modifiers,
    },
];

// ── Liang promotions ────────────────────────────────────────────────────────

fn liang_zoning_commissioner_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Zoning Commissioner"),
        target: TargetSelector::ProductionQueue,
        effect: EffectType::BuildTimePercent(-20),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn liang_aquaculture_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Aquaculture"),
        target: TargetSelector::Global,
        effect: EffectType::YieldFlat(YieldType::Food, 2),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn liang_reinforced_materials_modifiers() -> Vec<Modifier> {
    // "100% of wall damage is repaired each turn" is a special effect;
    // for now we model it as a flat combat strength bonus to the city.
    vec![Modifier {
        source: ModifierSource::Governor("Reinforced Materials"),
        target: TargetSelector::Global,
        effect: EffectType::CombatStrengthFlat(5),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

static LIANG_PROMOTIONS: &[GovernorPromotionDef] = &[
    GovernorPromotionDef {
        name: "Guildmaster",
        governor: "Liang",
        description: "+1 Builder charge for Builders trained in this city.",
        tier: 0,
        requires: &[],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Zoning Commissioner",
        governor: "Liang",
        description: "+20% Production towards district construction in the city.",
        tier: 1,
        requires: &["Guildmaster"],
        modifiers: liang_zoning_commissioner_modifiers,
    },
    GovernorPromotionDef {
        name: "Aquaculture",
        governor: "Liang",
        description: "Fisheries improvement can be built on coastal tiles; +1 Food from sea resources.",
        tier: 1,
        requires: &["Guildmaster"],
        modifiers: liang_aquaculture_modifiers,
    },
    GovernorPromotionDef {
        name: "Reinforced Materials",
        governor: "Liang",
        description: "100% of wall damage is repaired each turn in the city.",
        tier: 2,
        requires: &["Zoning Commissioner", "Aquaculture"],
        modifiers: liang_reinforced_materials_modifiers,
    },
    GovernorPromotionDef {
        name: "Water Electrification",
        governor: "Liang",
        description: "+4 Production from dams and hydroelectric sources in the city.",
        tier: 2,
        requires: &["Zoning Commissioner", "Aquaculture"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Parks and Recreation",
        governor: "Liang",
        description: "Can build City Parks in the city (+2 Culture, +1 Appeal).",
        tier: 3,
        requires: &["Reinforced Materials", "Water Electrification"],
        modifiers: no_modifiers,
    },
];

// ── Victor promotions ───────────────────────────────────────────────────────

fn victor_redoubt_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Redoubt"),
        target: TargetSelector::AllUnits,
        effect: EffectType::CombatStrengthFlat(5),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn victor_garrison_commander_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Garrison Commander"),
        target: TargetSelector::Global,
        effect: EffectType::CombatStrengthFlat(5),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn victor_defense_logistics_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Defense Logistics"),
        target: TargetSelector::AllUnits,
        effect: EffectType::HealEveryTurn,
        stacking: StackingRule::Max,
        condition: None,
    }]
}

static VICTOR_PROMOTIONS: &[GovernorPromotionDef] = &[
    GovernorPromotionDef {
        name: "Redoubt",
        governor: "Victor",
        description: "+5 Combat Strength to units within the city's territory.",
        tier: 0,
        requires: &[],
        modifiers: victor_redoubt_modifiers,
    },
    GovernorPromotionDef {
        name: "Garrison Commander",
        governor: "Victor",
        description: "+5 City Combat Strength; units in the city do not lose HP from enemy attacks.",
        tier: 1,
        requires: &["Redoubt"],
        modifiers: victor_garrison_commander_modifiers,
    },
    GovernorPromotionDef {
        name: "Embrasure",
        governor: "Victor",
        description: "Military units trained in this city start with a free promotion.",
        tier: 1,
        requires: &["Redoubt"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Arms Race Proponent",
        governor: "Victor",
        description: "+30% Production towards nuclear weapons projects in the city.",
        tier: 2,
        requires: &["Garrison Commander", "Embrasure"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Air Defense Initiative",
        governor: "Victor",
        description: "+25 anti-air combat strength in the city.",
        tier: 2,
        requires: &["Garrison Commander", "Embrasure"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Defense Logistics",
        governor: "Victor",
        description: "Units within the city heal fully each turn.",
        tier: 3,
        requires: &["Arms Race Proponent", "Air Defense Initiative"],
        modifiers: victor_defense_logistics_modifiers,
    },
];

// ── Pingala promotions ──────────────────────────────────────────────────────

fn pingala_connoisseur_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Connoisseur"),
        target: TargetSelector::Global,
        effect: EffectType::YieldFlat(YieldType::Culture, 3),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn pingala_librarian_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Librarian"),
        target: TargetSelector::Global,
        effect: EffectType::YieldFlat(YieldType::Science, 3),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn pingala_researcher_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Researcher"),
        target: TargetSelector::Global,
        effect: EffectType::YieldPercent(YieldType::Science, 20),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn pingala_grants_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Grants"),
        target: TargetSelector::Global,
        effect: EffectType::YieldPercent(YieldType::GreatPersonPoints, 100),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn pingala_curator_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Curator"),
        target: TargetSelector::Global,
        effect: EffectType::YieldPercent(YieldType::Tourism, 100),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn pingala_space_initiative_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Space Initiative"),
        target: TargetSelector::ProductionQueue,
        effect: EffectType::ProductionPercent(15),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

static PINGALA_PROMOTIONS: &[GovernorPromotionDef] = &[
    GovernorPromotionDef {
        name: "Connoisseur",
        governor: "Pingala",
        description: "+1 Culture per turn for each Citizen in the city.",
        tier: 0,
        requires: &[],
        modifiers: pingala_connoisseur_modifiers,
    },
    GovernorPromotionDef {
        name: "Librarian",
        governor: "Pingala",
        description: "+1 Science per turn for each Citizen in the city.",
        tier: 1,
        requires: &["Connoisseur"],
        modifiers: pingala_librarian_modifiers,
    },
    GovernorPromotionDef {
        name: "Researcher",
        governor: "Pingala",
        description: "+20% Science in the city.",
        tier: 1,
        requires: &["Connoisseur"],
        modifiers: pingala_researcher_modifiers,
    },
    GovernorPromotionDef {
        name: "Grants",
        governor: "Pingala",
        description: "+100% Great People points in the city.",
        tier: 2,
        requires: &["Librarian", "Researcher"],
        modifiers: pingala_grants_modifiers,
    },
    GovernorPromotionDef {
        name: "Curator",
        governor: "Pingala",
        description: "+100% Tourism from Great Works in the city.",
        tier: 2,
        requires: &["Librarian", "Researcher"],
        modifiers: pingala_curator_modifiers,
    },
    GovernorPromotionDef {
        name: "Space Initiative",
        governor: "Pingala",
        description: "+15% Production towards Space Race projects in the city.",
        tier: 3,
        requires: &["Grants", "Curator"],
        modifiers: pingala_space_initiative_modifiers,
    },
];

// ── Amani promotions ────────────────────────────────────────────────────────

fn amani_affluence_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Affluence"),
        target: TargetSelector::Global,
        effect: EffectType::YieldFlat(YieldType::Gold, 4),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

static AMANI_PROMOTIONS: &[GovernorPromotionDef] = &[
    GovernorPromotionDef {
        name: "Messenger",
        governor: "Amani",
        description: "Gain 2 Envoys when established in a city-state.",
        tier: 0,
        requires: &[],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Emissary",
        governor: "Amani",
        description: "Other civilizations cannot earn Envoys to the city-state she is established in.",
        tier: 1,
        requires: &["Messenger"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Affluence",
        governor: "Amani",
        description: "+4 Gold per turn while established in a city-state.",
        tier: 1,
        requires: &["Messenger"],
        modifiers: amani_affluence_modifiers,
    },
    GovernorPromotionDef {
        name: "Local Informants",
        governor: "Amani",
        description: "Enemy spy operations in the city always fail.",
        tier: 2,
        requires: &["Emissary", "Affluence"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Puppeteer",
        governor: "Amani",
        description: "Doubles the bonus from being Suzerain of the city-state.",
        tier: 2,
        requires: &["Emissary", "Affluence"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Prestige",
        governor: "Amani",
        description: "Other governors in cities within 8 tiles gain +3 Loyalty per turn.",
        tier: 3,
        requires: &["Local Informants", "Puppeteer"],
        modifiers: no_modifiers,
    },
];

// ── Ibrahim promotions ──────────────────────────────────────────────────────

fn ibrahim_khass_oda_bashi_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Khass-Oda-Bashi"),
        target: TargetSelector::ProductionQueue,
        effect: EffectType::ProductionPercent(20),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn ibrahim_capou_agha_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Capou Agha"),
        target: TargetSelector::AllUnits,
        effect: EffectType::CombatStrengthFlat(10),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

fn ibrahim_nisanci_modifiers() -> Vec<Modifier> {
    vec![Modifier {
        source: ModifierSource::Governor("Nisanci"),
        target: TargetSelector::AllUnits,
        effect: EffectType::CombatStrengthFlat(5),
        stacking: StackingRule::Additive,
        condition: None,
    }]
}

static IBRAHIM_PROMOTIONS: &[GovernorPromotionDef] = &[
    GovernorPromotionDef {
        name: "Head Falconer",
        governor: "Ibrahim",
        description: "Military units trained in this city gain +1 Movement on their first turn.",
        tier: 0,
        requires: &[],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Khass-Oda-Bashi",
        governor: "Ibrahim",
        description: "+20% Production towards military units in the city.",
        tier: 1,
        requires: &["Head Falconer"],
        modifiers: ibrahim_khass_oda_bashi_modifiers,
    },
    GovernorPromotionDef {
        name: "Grand Master of the Hunt",
        governor: "Ibrahim",
        description: "+50% combat experience for units trained in the city.",
        tier: 1,
        requires: &["Head Falconer"],
        modifiers: no_modifiers,
    },
    GovernorPromotionDef {
        name: "Capou Agha",
        governor: "Ibrahim",
        description: "Siege units trained in the city gain +10 Combat Strength.",
        tier: 2,
        requires: &["Khass-Oda-Bashi", "Grand Master of the Hunt"],
        modifiers: ibrahim_capou_agha_modifiers,
    },
    GovernorPromotionDef {
        name: "Nisanci",
        governor: "Ibrahim",
        description: "+5 Combat Strength when defending against attacks in the city.",
        tier: 2,
        requires: &["Khass-Oda-Bashi", "Grand Master of the Hunt"],
        modifiers: ibrahim_nisanci_modifiers,
    },
    GovernorPromotionDef {
        name: "Serasker",
        governor: "Ibrahim",
        description: "25% discount on military unit purchases with Faith or Gold in the city.",
        tier: 3,
        requires: &["Capou Agha", "Nisanci"],
        modifiers: no_modifiers,
    },
];

// ── Promotion lookup helpers ────────────────────────────────────────────────

/// All governor promotion definitions across all seven governors.
pub fn all_promotion_defs() -> Vec<&'static GovernorPromotionDef> {
    let mut defs = Vec::new();
    for table in &[
        REYNA_PROMOTIONS,
        MAGNUS_PROMOTIONS,
        LIANG_PROMOTIONS,
        VICTOR_PROMOTIONS,
        PINGALA_PROMOTIONS,
        AMANI_PROMOTIONS,
        IBRAHIM_PROMOTIONS,
    ] {
        for def in *table {
            defs.push(def);
        }
    }
    defs
}

/// Promotions belonging to a specific governor (by def_name).
pub fn promotions_for(governor_name: &str) -> Vec<&'static GovernorPromotionDef> {
    let table: &[GovernorPromotionDef] = match governor_name {
        "Reyna" => REYNA_PROMOTIONS,
        "Magnus" => MAGNUS_PROMOTIONS,
        "Liang" => LIANG_PROMOTIONS,
        "Victor" => VICTOR_PROMOTIONS,
        "Pingala" => PINGALA_PROMOTIONS,
        "Amani" => AMANI_PROMOTIONS,
        "Ibrahim" => IBRAHIM_PROMOTIONS,
        _ => return Vec::new(),
    };
    table.iter().collect()
}

/// Look up a specific promotion by name.
pub fn promotion_def(name: &str) -> Option<&'static GovernorPromotionDef> {
    all_promotion_defs().into_iter().find(|d| d.name == name)
}

/// Collect all active modifiers for an established governor based on its
/// unlocked promotions. Returns empty if the governor is not established.
pub fn get_governor_modifiers(governor: &Governor) -> Vec<Modifier> {
    if !governor.is_established() {
        return Vec::new();
    }

    let mut modifiers = Vec::new();

    // Base ability modifiers (tier 0 promotion)
    for def in promotions_for(governor.def_name) {
        if def.tier == 0 {
            modifiers.extend((def.modifiers)());
        }
    }

    // Unlocked promotion modifiers
    for promo_name in &governor.promotions {
        if let Some(def) = promotion_def(promo_name) {
            modifiers.extend((def.modifiers)());
        }
    }

    modifiers
}

/// Names of all seven built-in governors.
pub const GOVERNOR_NAMES: &[&str] = &[
    "Reyna", "Magnus", "Liang", "Victor", "Pingala", "Amani", "Ibrahim",
];
