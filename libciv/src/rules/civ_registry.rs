//! Static data definitions for all 8 starter civilizations.

use crate::civ::civ_ability::*;
use crate::civ::civ_identity::*;
use crate::civ::district::BuiltinDistrict;
use crate::rules::modifier::*;
use crate::rules::unique::*;
use crate::{PolicyType, UnitCategory, UnitDomain, YieldBundle, YieldType};

/// Return ability bundles for all 8 starter civilizations.
pub fn all_civ_bundles() -> Vec<CivAbilityBundle> {
    vec![rome(), greece(), egypt(), babylon(), germany(), japan(), india(), arabia()]
}

pub fn rome() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Rome, leader: BuiltinLeader::Trajan,
        civ_name: "Rome", adjective: "Roman", leader_name: "Trajan",
        civ_ability_name: "All Roads Lead to Rome",
        civ_ability_description: "All cities start with a Trading Post. Trade Routes earn +1 Gold for passing through Trading Posts in your own cities.",
        leader_ability_name: "Trajan's Column",
        leader_ability_description: "All cities start with a free Monument.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("All Roads Lead to Rome"),
                TargetSelector::TradeRoutesOwned,
                EffectType::TradeRouteYieldFlat(YieldType::Gold, 1),
                StackingRule::Additive,
            ).with_condition(Condition::PerTradingPostInRoute),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Rome, name: "legion", replaces: Some("swordsman"),
            production_cost: 110, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(40), ranged_strength: None,
            range: 0, vision_range: 2,
            resource_cost: Some((crate::world::resource::BuiltinResource::Iron, 10)),
            abilities: vec![UniqueUnitAbility::CanBuildFort],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Rome, name: "Bath",
            replaces: BuiltinDistrict::Aqueduct,
            base_cost: 36, extra_yields: YieldBundle::default(),
            extra_housing: 2, extra_amenities: 1,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![
            CityFoundedHook::FreeTradingPost,
            CityFoundedHook::RoadToCapital,
            CityFoundedHook::FreeBuilding("monument"),
        ],
        rule_overrides: vec![],
    }
}

pub fn greece() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Greece, leader: BuiltinLeader::Pericles,
        civ_name: "Greece", adjective: "Greek", leader_name: "Pericles",
        civ_ability_name: "Plato's Republic",
        civ_ability_description: "One extra Wildcard policy slot in any government.",
        leader_ability_name: "Surrounded by Glory",
        leader_ability_description: "+5% Culture per city-state you are the Suzerain of.",
        civ_modifiers: vec![],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Pericles"),
                TargetSelector::Global,
                EffectType::YieldPercent(YieldType::Culture, 5),
                StackingRule::Additive,
            ).with_condition(Condition::PerCityStateSuzerain),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Greece, name: "hoplite", replaces: Some("spearman"),
            production_cost: 65, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(28), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![UniqueUnitAbility::BonusAdjacentSameType(10)],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Greece, name: "Acropolis",
            replaces: BuiltinDistrict::TheaterSquare,
            base_cost: 54, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: Some(DistrictPlacementReq::MustBeOnHills),
            adjacency_overrides: vec![
                AdjacencyOverride {
                    source: AdjacencySource::PerAdjacentDistrict,
                    yield_type: YieldType::Culture,
                    amount: 1,
                },
            ],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::ExtraPolicySlot(PolicyType::Wildcard)],
    }
}

pub fn egypt() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Egypt, leader: BuiltinLeader::Cleopatra,
        civ_name: "Egypt", adjective: "Egyptian", leader_name: "Cleopatra",
        civ_ability_name: "Iteru",
        civ_ability_description: "+15% Production towards districts and wonders if placed next to a River.",
        leader_ability_name: "Mediterranean's Bride",
        leader_ability_description: "Trade Routes to other civilizations provide +4 Gold for Egypt.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Iteru"),
                TargetSelector::ProductionQueue,
                EffectType::ProductionPercent(15),
                StackingRule::Additive,
            ).with_condition(Condition::And(
                Box::new(Condition::ProducingDistrictOrWonder),
                Box::new(Condition::AdjacentToRiver),
            )),
        ],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Cleopatra"),
                TargetSelector::TradeRoutesOwned,
                EffectType::TradeRouteYieldFlat(YieldType::Gold, 4),
                StackingRule::Additive,
            ),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Egypt, name: "maryannu_chariot_archer", replaces: None,
            production_cost: 90, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(25), ranged_strength: Some(35),
            range: 2, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Egypt, name: "Sphinx",
            base_yields: YieldBundle::new().with(YieldType::Culture, 1).with(YieldType::Faith, 1),
            appeal_modifier: 2,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::ImmunityToFloodDamage],
    }
}

pub fn babylon() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Babylon, leader: BuiltinLeader::Hammurabi,
        civ_name: "Babylon", adjective: "Babylonian", leader_name: "Hammurabi",
        civ_ability_name: "Enuma Anu Enlil",
        civ_ability_description: "Eurekas provide all of the Science for technologies. -50% Science per turn.",
        leader_ability_name: "Ninu Ilu Sirum",
        leader_ability_description: "First specialty district of each type gives the lowest-cost building for free.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Babylon, name: "sabum_kibittum", replaces: Some("warrior"),
            production_cost: 35, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 300, combat_strength: Some(17), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Babylon, name: "palgum", replaces: Some("water_mill"),
            cost: 80, maintenance: 0,
            yields: YieldBundle::new().with(YieldType::Production, 2),
            requires_district: None, extra_housing: 1,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![
            RuleOverride::EurekaGivesFullTech,
            RuleOverride::SciencePerTurnMultiplier(-50),
            RuleOverride::FirstDistrictGivesFreeBuildingAndEnvoy,
        ],
    }
}

pub fn germany() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Germany, leader: BuiltinLeader::Barbarossa,
        civ_name: "Germany", adjective: "German", leader_name: "Barbarossa",
        civ_ability_name: "Free Imperial Cities",
        civ_ability_description: "Can build one more district than the population limit normally allows.",
        leader_ability_name: "Holy Roman Emperor",
        leader_ability_description: "+7 Combat Strength when attacking city-states. Additional Military policy slot.",
        civ_modifiers: vec![],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Barbarossa"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(7),
                StackingRule::Additive,
            ).with_condition(Condition::TargetIsCityState),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Germany, name: "u_boat", replaces: Some("submarine"),
            production_cost: 430, domain: UnitDomain::Sea, category: UnitCategory::Combat,
            max_movement: 300, combat_strength: Some(65), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Germany, name: "Hansa",
            replaces: BuiltinDistrict::IndustrialZone,
            base_cost: 54, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None,
            adjacency_overrides: vec![
                AdjacencyOverride {
                    source: AdjacencySource::SpecificDistrict(BuiltinDistrict::CommercialHub),
                    yield_type: YieldType::Production,
                    amount: 2,
                },
            ],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![
            RuleOverride::ExtraDistrictSlot(1),
            RuleOverride::ExtraPolicySlot(PolicyType::Military),
        ],
    }
}

pub fn japan() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Japan, leader: BuiltinLeader::Hojo,
        civ_name: "Japan", adjective: "Japanese", leader_name: "Hojo Tokimune",
        civ_ability_name: "Meiji Restoration",
        civ_ability_description: "Districts receive a +1 adjacency bonus for each adjacent district.",
        leader_ability_name: "Divine Wind",
        leader_ability_description: "+5 Combat Strength in land tiles adjacent to Coast. Encampment, Holy Site, and Theater Square districts are built in half the time.",
        civ_modifiers: vec![],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Hojo Tokimune"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(5),
                StackingRule::Additive,
            ).with_condition(Condition::OnCoast),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Japan, name: "samurai", replaces: None,
            production_cost: 160, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(48), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![UniqueUnitAbility::NoCombatPenaltyWhenDamaged],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Japan, name: "electronics_factory", replaces: Some("factory"),
            cost: 330, maintenance: 2,
            yields: YieldBundle::new().with(YieldType::Production, 3),
            requires_district: Some("Industrial Zone"),
            extra_housing: 0,
            abilities: vec![UniqueBuildingAbility::CultureToNearbyWhenPowered(4)],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![],
    }
}

pub fn india() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::India, leader: BuiltinLeader::Gandhi,
        civ_name: "India", adjective: "Indian", leader_name: "Gandhi",
        civ_ability_name: "Dharma",
        civ_ability_description: "Receive follower beliefs of all religions with at least 1 follower in your cities.",
        leader_ability_name: "Satyagraha",
        leader_ability_description: "+5 Faith per civ met that has founded a religion and is not at war. Enemies receive double war weariness.",
        civ_modifiers: vec![],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Gandhi"),
                TargetSelector::Global,
                EffectType::YieldFlat(YieldType::Faith, 5),
                StackingRule::Additive,
            ).with_condition(Condition::PerCivMetWithReligionNotAtWar),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::India, name: "varu", replaces: None,
            production_cost: 120, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(40), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![UniqueUnitAbility::DebuffAdjacentEnemies(5)],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::India, name: "Stepwell",
            base_yields: YieldBundle::new().with(YieldType::Food, 1).with(YieldType::Housing, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![
                ImprovementAdjacencyBonus {
                    adjacent_to: ImprovementAdjacencySource::HolySite,
                    yield_type: YieldType::Faith,
                    amount: 1,
                },
                ImprovementAdjacencyBonus {
                    adjacent_to: ImprovementAdjacencySource::Farm,
                    yield_type: YieldType::Food,
                    amount: 1,
                },
            ],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![
            RuleOverride::FollowerBeliefsFromAllReligions,
            RuleOverride::DoubleWarWearinessToEnemies,
        ],
    }
}

pub fn arabia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Arabia, leader: BuiltinLeader::Saladin,
        civ_name: "Arabia", adjective: "Arabian", leader_name: "Saladin",
        civ_ability_name: "The Last Prophet",
        civ_ability_description: "Automatically receive the last Great Prophet. +1 Science per foreign city following Arabia's religion.",
        leader_ability_name: "Righteousness of the Faith",
        leader_ability_description: "Worship buildings cost 90% less Faith. +10% Science, Culture, and Faith in cities with a worship building.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("The Last Prophet"),
                TargetSelector::Global,
                EffectType::YieldFlat(YieldType::Science, 1),
                StackingRule::Additive,
            ).with_condition(Condition::PerForeignCityWithWorshipBuilding),
        ],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Saladin"),
                TargetSelector::Global,
                EffectType::WorshipBuildingCostPercent(-90),
                StackingRule::Additive,
            ),
            Modifier::new(
                ModifierSource::Leader("Saladin"),
                TargetSelector::Global,
                EffectType::YieldPercent(YieldType::Science, 10),
                StackingRule::Additive,
            ).with_condition(Condition::CityHasWorshipBuilding),
            Modifier::new(
                ModifierSource::Leader("Saladin"),
                TargetSelector::Global,
                EffectType::YieldPercent(YieldType::Culture, 10),
                StackingRule::Additive,
            ).with_condition(Condition::CityHasWorshipBuilding),
            Modifier::new(
                ModifierSource::Leader("Saladin"),
                TargetSelector::Global,
                EffectType::YieldPercent(YieldType::Faith, 10),
                StackingRule::Additive,
            ).with_condition(Condition::CityHasWorshipBuilding),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Arabia, name: "mamluk", replaces: Some("knight"),
            production_cost: 220, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(50), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![UniqueUnitAbility::HealEveryTurn],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Arabia, name: "madrasa", replaces: Some("university"),
            cost: 250, maintenance: 2,
            yields: YieldBundle::new().with(YieldType::Science, 5),
            requires_district: Some("Campus"),
            extra_housing: 1,
            abilities: vec![UniqueBuildingAbility::FaithEqualsCampusAdjacency],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::AutoLastGreatProphet],
    }
}
