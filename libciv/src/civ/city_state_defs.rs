//! Static definitions for all 24 base-game city-states.

use crate::civ::city_state::CityStateType;
use crate::rules::modifier::*;
use crate::YieldType;

/// Static definition of a city-state before placement on the map.
#[derive(Debug, Clone)]
pub struct CityStateDef {
    pub name: &'static str,
    pub state_type: CityStateType,
    pub suzerain_bonus_description: &'static str,
    pub suzerain_modifiers: Vec<Modifier>,
    /// Envoy bonuses at 1, 3, and 6 envoys.
    pub envoy_1_modifiers: Vec<Modifier>,
    pub envoy_3_modifiers: Vec<Modifier>,
    pub envoy_6_modifiers: Vec<Modifier>,
}

// ── Helper constructors ─────────────────────────────────────────────────────

fn envoy_yield_flat(city_state: &'static str, yield_type: YieldType, amount: i32) -> Modifier {
    Modifier::new(
        ModifierSource::Custom(city_state),
        TargetSelector::Global,
        EffectType::YieldFlat(yield_type, amount),
        StackingRule::Additive,
    )
}

fn suzerain_yield_flat(city_state: &'static str, yield_type: YieldType, amount: i32) -> Modifier {
    Modifier::new(
        ModifierSource::Custom(city_state),
        TargetSelector::Global,
        EffectType::YieldFlat(yield_type, amount),
        StackingRule::Additive,
    )
}

fn suzerain_yield_percent(city_state: &'static str, yield_type: YieldType, percent: i32) -> Modifier {
    Modifier::new(
        ModifierSource::Custom(city_state),
        TargetSelector::Global,
        EffectType::YieldPercent(yield_type, percent),
        StackingRule::Additive,
    )
}

fn suzerain_production_percent(city_state: &'static str, percent: i32) -> Modifier {
    Modifier::new(
        ModifierSource::Custom(city_state),
        TargetSelector::ProductionQueue,
        EffectType::ProductionPercent(percent),
        StackingRule::Additive,
    )
}

// ── Trade envoy modifiers ───────────────────────────────────────────────────

fn trade_envoy_1(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Gold, 4)]
}

fn trade_envoy_3(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Gold, 4)]
}

fn trade_envoy_6(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Gold, 4)]
}

// ── Cultural envoy modifiers ────────────────────────────────────────────────

fn cultural_envoy_1(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Culture, 2)]
}

fn cultural_envoy_3(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Culture, 2)]
}

fn cultural_envoy_6(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Culture, 2)]
}

// ── Scientific envoy modifiers ──────────────────────────────────────────────

fn scientific_envoy_1(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Science, 2)]
}

fn scientific_envoy_3(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Science, 2)]
}

fn scientific_envoy_6(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Science, 2)]
}

// ── Religious envoy modifiers ───────────────────────────────────────────────

fn religious_envoy_1(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Faith, 2)]
}

fn religious_envoy_3(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Faith, 2)]
}

fn religious_envoy_6(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Faith, 2)]
}

// ── Militaristic envoy modifiers ────────────────────────────────────────────

fn militaristic_envoy_1(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Production, 2)]
}

fn militaristic_envoy_3(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Production, 2)]
}

fn militaristic_envoy_6(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Production, 2)]
}

// ── Industrial envoy modifiers ──────────────────────────────────────────────

fn industrial_envoy_1(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Production, 2)]
}

fn industrial_envoy_3(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Production, 2)]
}

fn industrial_envoy_6(city_state: &'static str) -> Vec<Modifier> {
    vec![envoy_yield_flat(city_state, YieldType::Production, 2)]
}

// ── City-state definitions ──────────────────────────────────────────────────

/// Returns all built-in city-state definitions.
pub fn builtin_city_state_defs() -> Vec<CityStateDef> {
    vec![
        // ── Trade city-states ───────────────────────────────────────────
        CityStateDef {
            name: "Amsterdam",
            state_type: CityStateType::Trade,
            suzerain_bonus_description: "+2 Gold per luxury resource",
            suzerain_modifiers: vec![
                suzerain_yield_flat("Amsterdam", YieldType::Gold, 2),
            ],
            envoy_1_modifiers: trade_envoy_1("Amsterdam"),
            envoy_3_modifiers: trade_envoy_3("Amsterdam"),
            envoy_6_modifiers: trade_envoy_6("Amsterdam"),
        },
        CityStateDef {
            name: "Jakarta",
            state_type: CityStateType::Trade,
            suzerain_bonus_description: "+1 Trading Post in every city",
            // Complex effect: trading posts are placed automatically; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: trade_envoy_1("Jakarta"),
            envoy_3_modifiers: trade_envoy_3("Jakarta"),
            envoy_6_modifiers: trade_envoy_6("Jakarta"),
        },
        CityStateDef {
            name: "Lisbon",
            state_type: CityStateType::Trade,
            suzerain_bonus_description: "Trader units immune to plunder on water",
            // Complex effect: plunder immunity has no direct modifier representation.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: trade_envoy_1("Lisbon"),
            envoy_3_modifiers: trade_envoy_3("Lisbon"),
            envoy_6_modifiers: trade_envoy_6("Lisbon"),
        },
        CityStateDef {
            name: "Zanzibar",
            state_type: CityStateType::Trade,
            suzerain_bonus_description: "Provides Cinnamon and Cloves luxury resources",
            // Complex effect: grants luxury resources; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: trade_envoy_1("Zanzibar"),
            envoy_3_modifiers: trade_envoy_3("Zanzibar"),
            envoy_6_modifiers: trade_envoy_6("Zanzibar"),
        },
        CityStateDef {
            name: "Muscat",
            state_type: CityStateType::Trade,
            suzerain_bonus_description: "+1 Amenity per luxury improvement",
            suzerain_modifiers: vec![
                Modifier::new(
                    ModifierSource::Custom("Muscat"),
                    TargetSelector::Global,
                    EffectType::AmenityFlat(1),
                    StackingRule::Additive,
                ),
            ],
            envoy_1_modifiers: trade_envoy_1("Muscat"),
            envoy_3_modifiers: trade_envoy_3("Muscat"),
            envoy_6_modifiers: trade_envoy_6("Muscat"),
        },

        // ── Industrial city-states ──────────────────────────────────────
        CityStateDef {
            name: "Brussels",
            state_type: CityStateType::Industrial,
            suzerain_bonus_description: "+15% Production towards wonders",
            suzerain_modifiers: vec![
                suzerain_production_percent("Brussels", 15),
            ],
            envoy_1_modifiers: industrial_envoy_1("Brussels"),
            envoy_3_modifiers: industrial_envoy_3("Brussels"),
            envoy_6_modifiers: industrial_envoy_6("Brussels"),
        },
        CityStateDef {
            name: "Buenos Aires",
            state_type: CityStateType::Industrial,
            suzerain_bonus_description: "Bonus resources behave as luxury (Amenities)",
            // Complex effect: changes resource category behavior; approximate with amenity bonus.
            suzerain_modifiers: vec![
                Modifier::new(
                    ModifierSource::Custom("Buenos Aires"),
                    TargetSelector::Global,
                    EffectType::AmenityFlat(1),
                    StackingRule::Additive,
                ),
            ],
            envoy_1_modifiers: industrial_envoy_1("Buenos Aires"),
            envoy_3_modifiers: industrial_envoy_3("Buenos Aires"),
            envoy_6_modifiers: industrial_envoy_6("Buenos Aires"),
        },
        CityStateDef {
            name: "Hong Kong",
            state_type: CityStateType::Industrial,
            suzerain_bonus_description: "+20% Production for projects",
            suzerain_modifiers: vec![
                suzerain_production_percent("Hong Kong", 20),
            ],
            envoy_1_modifiers: industrial_envoy_1("Hong Kong"),
            envoy_3_modifiers: industrial_envoy_3("Hong Kong"),
            envoy_6_modifiers: industrial_envoy_6("Hong Kong"),
        },
        CityStateDef {
            name: "Toronto",
            state_type: CityStateType::Industrial,
            suzerain_bonus_description: "Regional buildings extend +3 tiles",
            // Complex effect: extends building range; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: industrial_envoy_1("Toronto"),
            envoy_3_modifiers: industrial_envoy_3("Toronto"),
            envoy_6_modifiers: industrial_envoy_6("Toronto"),
        },

        // ── Scientific city-states ──────────────────────────────────────
        CityStateDef {
            name: "Geneva",
            state_type: CityStateType::Scientific,
            suzerain_bonus_description: "+15% Science when not at war",
            suzerain_modifiers: vec![
                suzerain_yield_percent("Geneva", YieldType::Science, 15)
                    .with_condition(Condition::NotAtWar),
            ],
            envoy_1_modifiers: scientific_envoy_1("Geneva"),
            envoy_3_modifiers: scientific_envoy_3("Geneva"),
            envoy_6_modifiers: scientific_envoy_6("Geneva"),
        },
        CityStateDef {
            name: "Seoul",
            state_type: CityStateType::Scientific,
            suzerain_bonus_description: "+3 Science for each tech researched",
            suzerain_modifiers: vec![
                suzerain_yield_flat("Seoul", YieldType::Science, 3),
            ],
            envoy_1_modifiers: scientific_envoy_1("Seoul"),
            envoy_3_modifiers: scientific_envoy_3("Seoul"),
            envoy_6_modifiers: scientific_envoy_6("Seoul"),
        },
        CityStateDef {
            name: "Stockholm",
            state_type: CityStateType::Scientific,
            suzerain_bonus_description: "+1 Great Person point per specialty district",
            suzerain_modifiers: vec![
                suzerain_yield_flat("Stockholm", YieldType::GreatPersonPoints, 1),
            ],
            envoy_1_modifiers: scientific_envoy_1("Stockholm"),
            envoy_3_modifiers: scientific_envoy_3("Stockholm"),
            envoy_6_modifiers: scientific_envoy_6("Stockholm"),
        },
        CityStateDef {
            name: "Hattusa",
            state_type: CityStateType::Scientific,
            suzerain_bonus_description: "Provides 1 copy of each strategic resource you have none of",
            // Complex effect: grants strategic resources; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: scientific_envoy_1("Hattusa"),
            envoy_3_modifiers: scientific_envoy_3("Hattusa"),
            envoy_6_modifiers: scientific_envoy_6("Hattusa"),
        },

        // ── Cultural city-states ────────────────────────────────────────
        CityStateDef {
            name: "Kumasi",
            state_type: CityStateType::Cultural,
            suzerain_bonus_description: "+2 Culture, +1 Gold per trade route to Kumasi",
            suzerain_modifiers: vec![
                Modifier::new(
                    ModifierSource::Custom("Kumasi"),
                    TargetSelector::TradeRoutesOwned,
                    EffectType::TradeRouteYieldFlat(YieldType::Culture, 2),
                    StackingRule::Additive,
                ),
                Modifier::new(
                    ModifierSource::Custom("Kumasi"),
                    TargetSelector::TradeRoutesOwned,
                    EffectType::TradeRouteYieldFlat(YieldType::Gold, 1),
                    StackingRule::Additive,
                ),
            ],
            envoy_1_modifiers: cultural_envoy_1("Kumasi"),
            envoy_3_modifiers: cultural_envoy_3("Kumasi"),
            envoy_6_modifiers: cultural_envoy_6("Kumasi"),
        },
        CityStateDef {
            name: "Mohenjo-Daro",
            state_type: CityStateType::Cultural,
            suzerain_bonus_description: "Full housing from water in all cities",
            // Complex effect: overrides housing from water; approximate with housing bonus.
            suzerain_modifiers: vec![
                Modifier::new(
                    ModifierSource::Custom("Mohenjo-Daro"),
                    TargetSelector::Global,
                    EffectType::HousingFlat(3),
                    StackingRule::Additive,
                ),
            ],
            envoy_1_modifiers: cultural_envoy_1("Mohenjo-Daro"),
            envoy_3_modifiers: cultural_envoy_3("Mohenjo-Daro"),
            envoy_6_modifiers: cultural_envoy_6("Mohenjo-Daro"),
        },
        CityStateDef {
            name: "Nan Madol",
            state_type: CityStateType::Cultural,
            suzerain_bonus_description: "+2 Culture per district on coast",
            suzerain_modifiers: vec![
                Modifier::new(
                    ModifierSource::Custom("Nan Madol"),
                    TargetSelector::Global,
                    EffectType::YieldFlat(YieldType::Culture, 2),
                    StackingRule::Additive,
                ).with_condition(Condition::OnCoast),
            ],
            envoy_1_modifiers: cultural_envoy_1("Nan Madol"),
            envoy_3_modifiers: cultural_envoy_3("Nan Madol"),
            envoy_6_modifiers: cultural_envoy_6("Nan Madol"),
        },
        CityStateDef {
            name: "Vilnius",
            state_type: CityStateType::Cultural,
            suzerain_bonus_description: "+50% Faith bonus to adjacent Holy Site",
            suzerain_modifiers: vec![
                suzerain_yield_percent("Vilnius", YieldType::Faith, 50),
            ],
            envoy_1_modifiers: cultural_envoy_1("Vilnius"),
            envoy_3_modifiers: cultural_envoy_3("Vilnius"),
            envoy_6_modifiers: cultural_envoy_6("Vilnius"),
        },

        // ── Religious city-states ───────────────────────────────────────
        CityStateDef {
            name: "Jerusalem",
            state_type: CityStateType::Religious,
            suzerain_bonus_description: "Holy city cannot lose majority religion",
            // Complex effect: religion lock; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: religious_envoy_1("Jerusalem"),
            envoy_3_modifiers: religious_envoy_3("Jerusalem"),
            envoy_6_modifiers: religious_envoy_6("Jerusalem"),
        },
        CityStateDef {
            name: "Kandy",
            state_type: CityStateType::Religious,
            suzerain_bonus_description: "Relic when discovering natural wonder",
            // Complex effect: one-shot relic grant; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: religious_envoy_1("Kandy"),
            envoy_3_modifiers: religious_envoy_3("Kandy"),
            envoy_6_modifiers: religious_envoy_6("Kandy"),
        },
        CityStateDef {
            name: "La Venta",
            state_type: CityStateType::Religious,
            suzerain_bonus_description: "Can build Colossal Head improvement",
            // Complex effect: unlocks unique improvement; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: religious_envoy_1("La Venta"),
            envoy_3_modifiers: religious_envoy_3("La Venta"),
            envoy_6_modifiers: religious_envoy_6("La Venta"),
        },
        CityStateDef {
            name: "Yerevan",
            state_type: CityStateType::Religious,
            suzerain_bonus_description: "Choose Apostle promotions",
            // Complex effect: apostle promotion choice; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: religious_envoy_1("Yerevan"),
            envoy_3_modifiers: religious_envoy_3("Yerevan"),
            envoy_6_modifiers: religious_envoy_6("Yerevan"),
        },

        // ── Militaristic city-states ────────────────────────────────────
        CityStateDef {
            name: "Kabul",
            state_type: CityStateType::Militaristic,
            suzerain_bonus_description: "Double XP from combat",
            // Complex effect: XP multiplier; no direct modifier.
            suzerain_modifiers: vec![],
            envoy_1_modifiers: militaristic_envoy_1("Kabul"),
            envoy_3_modifiers: militaristic_envoy_3("Kabul"),
            envoy_6_modifiers: militaristic_envoy_6("Kabul"),
        },
        CityStateDef {
            name: "Preslav",
            state_type: CityStateType::Militaristic,
            suzerain_bonus_description: "+50% Production for light and heavy cavalry",
            suzerain_modifiers: vec![
                suzerain_production_percent("Preslav", 50),
            ],
            envoy_1_modifiers: militaristic_envoy_1("Preslav"),
            envoy_3_modifiers: militaristic_envoy_3("Preslav"),
            envoy_6_modifiers: militaristic_envoy_6("Preslav"),
        },
        CityStateDef {
            name: "Valletta",
            state_type: CityStateType::Militaristic,
            suzerain_bonus_description: "Can buy city center buildings with Faith",
            // Complex effect: faith purchase unlock; approximate with worship building cost reduction.
            suzerain_modifiers: vec![
                Modifier::new(
                    ModifierSource::Custom("Valletta"),
                    TargetSelector::Global,
                    EffectType::WorshipBuildingCostPercent(-50),
                    StackingRule::Additive,
                ),
            ],
            envoy_1_modifiers: militaristic_envoy_1("Valletta"),
            envoy_3_modifiers: militaristic_envoy_3("Valletta"),
            envoy_6_modifiers: militaristic_envoy_6("Valletta"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_city_state_defs_count() {
        let defs = builtin_city_state_defs();
        assert_eq!(defs.len(), 24, "Expected 24 base-game city-state definitions");
    }

    #[test]
    fn test_all_names_unique() {
        let defs = builtin_city_state_defs();
        let mut names: Vec<&str> = defs.iter().map(|d| d.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 24, "All city-state names must be unique");
    }

    #[test]
    fn test_type_distribution() {
        let defs = builtin_city_state_defs();
        let trade = defs.iter().filter(|d| d.state_type == CityStateType::Trade).count();
        let industrial = defs.iter().filter(|d| d.state_type == CityStateType::Industrial).count();
        let scientific = defs.iter().filter(|d| d.state_type == CityStateType::Scientific).count();
        let cultural = defs.iter().filter(|d| d.state_type == CityStateType::Cultural).count();
        let religious = defs.iter().filter(|d| d.state_type == CityStateType::Religious).count();
        let militaristic = defs.iter().filter(|d| d.state_type == CityStateType::Militaristic).count();

        assert_eq!(trade, 5);
        assert_eq!(industrial, 4);
        assert_eq!(scientific, 4);
        assert_eq!(cultural, 4);
        assert_eq!(religious, 4);
        assert_eq!(militaristic, 3);
    }

    #[test]
    fn test_all_have_envoy_modifiers() {
        let defs = builtin_city_state_defs();
        for def in &defs {
            assert!(
                !def.envoy_1_modifiers.is_empty(),
                "{} should have envoy 1 modifiers",
                def.name,
            );
            assert!(
                !def.envoy_3_modifiers.is_empty(),
                "{} should have envoy 3 modifiers",
                def.name,
            );
            assert!(
                !def.envoy_6_modifiers.is_empty(),
                "{} should have envoy 6 modifiers",
                def.name,
            );
        }
    }

    #[test]
    fn test_geneva_has_not_at_war_condition() {
        let defs = builtin_city_state_defs();
        let geneva = defs.iter().find(|d| d.name == "Geneva").unwrap();
        assert!(!geneva.suzerain_modifiers.is_empty());
        let modifier = &geneva.suzerain_modifiers[0];
        assert!(
            matches!(modifier.condition, Some(Condition::NotAtWar)),
            "Geneva suzerain bonus should require NotAtWar condition",
        );
    }
}
