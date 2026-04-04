//! Static definitions for all 10 base-game governments.

use crate::rules::modifier::*;
use crate::rules::policy::*;
use crate::{AgeType, GovernmentId, YieldType};

/// Intermediate definition for a government before ID assignment.
pub struct GovernmentDef {
    pub name: &'static str,
    pub era: AgeType,
    pub prereq_civic: &'static str,
    pub slots: PolicySlots,
    pub inherent_modifiers: Vec<Modifier>,
    pub legacy_bonus: Option<&'static str>,
}

/// Returns definitions for all 13 base-game governments (10 original + 3 Future Era).
pub fn builtin_government_defs() -> Vec<GovernmentDef> {
    vec![
        // ── Ancient ────────────────────────────────────────────────────────
        GovernmentDef {
            name: "Chiefdom",
            era: AgeType::Ancient,
            prereq_civic: "Code of Laws",
            slots: PolicySlots { military: 1, economic: 1, diplomatic: 0, wildcard: 0 },
            inherent_modifiers: vec![],
            legacy_bonus: None,
        },
        // ── Classical ──────────────────────────────────────────────────────
        GovernmentDef {
            name: "Autocracy",
            era: AgeType::Classical,
            prereq_civic: "Political Philosophy",
            slots: PolicySlots { military: 1, economic: 1, diplomatic: 1, wildcard: 1 },
            inherent_modifiers: vec![
                // +1 to all yields in capital
                Modifier::new(
                    ModifierSource::Custom("Autocracy"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Gold, 1),
                    StackingRule::Additive,
                ),
                Modifier::new(
                    ModifierSource::Custom("Autocracy"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Production, 1),
                    StackingRule::Additive,
                ),
                Modifier::new(
                    ModifierSource::Custom("Autocracy"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Science, 1),
                    StackingRule::Additive,
                ),
                Modifier::new(
                    ModifierSource::Custom("Autocracy"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Culture, 1),
                    StackingRule::Additive,
                ),
                Modifier::new(
                    ModifierSource::Custom("Autocracy"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Faith, 1),
                    StackingRule::Additive,
                ),
                Modifier::new(
                    ModifierSource::Custom("Autocracy"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Food, 1),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+1 to all yields in capital"),
        },
        GovernmentDef {
            name: "Oligarchy",
            era: AgeType::Classical,
            prereq_civic: "Political Philosophy",
            slots: PolicySlots { military: 2, economic: 1, diplomatic: 0, wildcard: 1 },
            inherent_modifiers: vec![
                // +4 Combat Strength for melee/anti-cavalry
                Modifier::new(
                    ModifierSource::Custom("Oligarchy"),
                    TargetSelector::AllUnits,
                    EffectType::CombatStrengthFlat(4),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+4 Combat Strength for melee and anti-cavalry units"),
        },
        GovernmentDef {
            name: "Classical Republic",
            era: AgeType::Classical,
            prereq_civic: "Political Philosophy",
            slots: PolicySlots { military: 0, economic: 2, diplomatic: 1, wildcard: 1 },
            inherent_modifiers: vec![
                // +1 Amenity per district with buildings
                Modifier::new(
                    ModifierSource::Custom("Classical Republic"),
                    TargetSelector::Global,
                    EffectType::AmenityFlat(1),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+1 Amenity per district with buildings"),
        },
        // ── Medieval ───────────────────────────────────────────────────────
        GovernmentDef {
            name: "Monarchy",
            era: AgeType::Medieval,
            prereq_civic: "Divine Right",
            slots: PolicySlots { military: 2, economic: 1, diplomatic: 1, wildcard: 2 },
            inherent_modifiers: vec![
                // +50% influence per turn
                Modifier::new(
                    ModifierSource::Custom("Monarchy"),
                    TargetSelector::Global,
                    EffectType::YieldPercent(YieldType::Culture, 50),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+50% influence per turn"),
        },
        GovernmentDef {
            name: "Theocracy",
            era: AgeType::Medieval,
            prereq_civic: "Reformed Church",
            slots: PolicySlots { military: 2, economic: 2, diplomatic: 1, wildcard: 1 },
            inherent_modifiers: vec![
                // +5 Religious Strength, can buy land units with Faith
                Modifier::new(
                    ModifierSource::Custom("Theocracy"),
                    TargetSelector::Global,
                    EffectType::CombatStrengthFlat(5),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+5 Religious Strength; can buy land units with Faith"),
        },
        GovernmentDef {
            name: "Merchant Republic",
            era: AgeType::Medieval,
            prereq_civic: "Exploration",
            slots: PolicySlots { military: 1, economic: 2, diplomatic: 2, wildcard: 1 },
            inherent_modifiers: vec![
                // +2 Trade Route capacity
                Modifier::new(
                    ModifierSource::Custom("Merchant Republic"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Gold, 2),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+2 Trade Route capacity"),
        },
        // ── Modern ─────────────────────────────────────────────────────────
        GovernmentDef {
            name: "Fascism",
            era: AgeType::Modern,
            prereq_civic: "Totalitarianism",
            slots: PolicySlots { military: 4, economic: 1, diplomatic: 1, wildcard: 2 },
            inherent_modifiers: vec![
                // +5 Combat Strength to all units
                Modifier::new(
                    ModifierSource::Custom("Fascism"),
                    TargetSelector::AllUnits,
                    EffectType::CombatStrengthFlat(5),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+5 Combat Strength to all units"),
        },
        GovernmentDef {
            name: "Communism",
            era: AgeType::Modern,
            prereq_civic: "Class Struggle",
            slots: PolicySlots { military: 3, economic: 3, diplomatic: 1, wildcard: 1 },
            inherent_modifiers: vec![
                // +0.6 Production per citizen (approximated as +1 production flat)
                Modifier::new(
                    ModifierSource::Custom("Communism"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Production, 1),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+0.6 Production per citizen"),
        },
        GovernmentDef {
            name: "Democracy",
            era: AgeType::Modern,
            prereq_civic: "Suffrage",
            slots: PolicySlots { military: 1, economic: 3, diplomatic: 2, wildcard: 2 },
            inherent_modifiers: vec![
                // +2 Gold, +2 Production per trade route to allies
                Modifier::new(
                    ModifierSource::Custom("Democracy"),
                    TargetSelector::TradeRoutesOwned,
                    EffectType::TradeRouteYieldFlat(YieldType::Gold, 2),
                    StackingRule::Additive,
                ),
                Modifier::new(
                    ModifierSource::Custom("Democracy"),
                    TargetSelector::TradeRoutesOwned,
                    EffectType::TradeRouteYieldFlat(YieldType::Production, 2),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+2 Gold, +2 Production per trade route to allies"),
        },
        // ── Future ─────────────────────────────────────────────────────────
        GovernmentDef {
            name: "Corporate Libertarianism",
            era: AgeType::Future,
            prereq_civic: "Corporate Libertarianism",
            slots: PolicySlots { military: 2, economic: 4, diplomatic: 0, wildcard: 2 },
            inherent_modifiers: vec![
                // +50% tourism from trade routes
                Modifier::new(
                    ModifierSource::Custom("Corporate Libertarianism"),
                    TargetSelector::TradeRoutesOwned,
                    EffectType::YieldPercent(YieldType::Culture, 50),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+50% tourism from trade routes"),
        },
        GovernmentDef {
            name: "Digital Democracy",
            era: AgeType::Future,
            prereq_civic: "Digital Democracy",
            slots: PolicySlots { military: 1, economic: 1, diplomatic: 4, wildcard: 2 },
            inherent_modifiers: vec![
                // +2 diplomatic favor per suzerainty
                Modifier::new(
                    ModifierSource::Custom("Digital Democracy"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Gold, 2),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+2 diplomatic favor per suzerainty"),
        },
        GovernmentDef {
            name: "Synthetic Technocracy",
            era: AgeType::Future,
            prereq_civic: "Synthetic Technocracy",
            slots: PolicySlots { military: 0, economic: 3, diplomatic: 1, wildcard: 4 },
            inherent_modifiers: vec![
                // +100% district adjacency bonuses (all districts)
                Modifier::new(
                    ModifierSource::Custom("Synthetic Technocracy"),
                    TargetSelector::Global,
                    EffectType::YieldPercent(YieldType::Science, 100),
                    StackingRule::Additive,
                ),
            ],
            legacy_bonus: Some("+100% district adjacency bonuses"),
        },
    ]
}

/// Create concrete `Government` instances from the built-in definitions,
/// assigning unique IDs from the given generator.
pub fn register_builtin_governments(id_gen: &mut crate::game::state::IdGenerator) -> Vec<Government> {
    builtin_government_defs()
        .into_iter()
        .map(|def| Government {
            id: GovernmentId::from_ulid(id_gen.next_ulid()),
            name: def.name,
            era: def.era,
            prereq_civic: def.prereq_civic,
            slots: def.slots,
            inherent_modifiers: def.inherent_modifiers,
            legacy_bonus: def.legacy_bonus,
        })
        .collect()
}
