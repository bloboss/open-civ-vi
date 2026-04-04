//! Static data definitions for all 19 base-game civilizations.

use crate::civ::civ_ability::*;
use crate::civ::civ_identity::*;
use crate::civ::district::BuiltinDistrict;
use crate::rules::modifier::*;
use crate::rules::unique::*;
use crate::{PolicyType, UnitCategory, UnitDomain, YieldBundle, YieldType};

/// Return ability bundles for all 19 base-game civilizations.
pub fn all_civ_bundles() -> Vec<CivAbilityBundle> {
    vec![
        rome(), greece(), egypt(), babylon(), germany(), japan(), india(), arabia(),
        america(), brazil(), china(), england(), france(), kongo(), norway(), russia(),
        scythia(), spain(), sumeria(),
    ]
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

// ── New base-game civilizations ─────────────────────────────────────────────

pub fn america() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::America, leader: BuiltinLeader::Roosevelt,
        civ_name: "America", adjective: "American", leader_name: "Teddy Roosevelt",
        civ_ability_name: "Founding Fathers",
        civ_ability_description: "Earn government legacy bonuses in half the usual time.",
        leader_ability_name: "Roosevelt Corollary",
        leader_ability_description: "+5 Combat Strength on your home continent. +1 Appeal to tiles in cities with a National Park.",
        civ_modifiers: vec![],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Teddy Roosevelt"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(5),
                StackingRule::Additive,
            ),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::America, name: "rough_rider", replaces: None,
            production_cost: 340, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 500, combat_strength: Some(67), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::America, name: "film_studio", replaces: Some("broadcast_center"),
            cost: 580, maintenance: 3,
            yields: YieldBundle::new().with(YieldType::Culture, 4),
            requires_district: Some("Theater Square"),
            extra_housing: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::LegacyBonusesFaster],
    }
}

pub fn brazil() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Brazil, leader: BuiltinLeader::Pedro,
        civ_name: "Brazil", adjective: "Brazilian", leader_name: "Pedro II",
        civ_ability_name: "Amazon",
        civ_ability_description: "Rainforest tiles provide +1 adjacency bonus for Campus, Commercial Hub, Holy Site, and Theater Square districts.",
        leader_ability_name: "Magnanimous",
        leader_ability_description: "After recruiting or patronizing a Great Person, 20% of the Great Person point cost is refunded.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Brazil, name: "minas_geraes", replaces: Some("battleship"),
            production_cost: 430, domain: UnitDomain::Sea, category: UnitCategory::Combat,
            max_movement: 500, combat_strength: Some(70), ranged_strength: Some(80),
            range: 3, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Brazil, name: "Street Carnival",
            replaces: BuiltinDistrict::EntertainmentComplex,
            base_cost: 54, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 2,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![],
    }
}

pub fn china() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::China, leader: BuiltinLeader::QinShiHuang,
        civ_name: "China", adjective: "Chinese", leader_name: "Qin Shi Huang",
        civ_ability_name: "Dynastic Cycle",
        civ_ability_description: "Eurekas and Inspirations provide +10% of the civic or technology cost.",
        leader_ability_name: "The First Emperor",
        leader_ability_description: "Builders receive one extra charge. Can use a builder charge to complete 15% of an Ancient or Classical wonder.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::China, name: "crouching_tiger", replaces: None,
            production_cost: 160, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(40), ranged_strength: Some(50),
            range: 1, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::China, name: "Great Wall",
            base_yields: YieldBundle::new().with(YieldType::Gold, 2).with(YieldType::Culture, 2),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::EurekaInspirationBonus(10)],
    }
}

pub fn england() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::England, leader: BuiltinLeader::Victoria,
        civ_name: "England", adjective: "English", leader_name: "Victoria",
        civ_ability_name: "British Museum",
        civ_ability_description: "Each Archaeological Museum holds 6 Artifacts instead of 3 and can hold any artifacts.",
        leader_ability_name: "Pax Britannica",
        leader_ability_description: "The first city founded on each continent other than your home continent receives a free melee unit. Gain the Redcoat unique unit with Military Science.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::England, name: "redcoat", replaces: None,
            production_cost: 280, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(65), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::England, name: "Royal Navy Dockyard",
            replaces: BuiltinDistrict::Harbor,
            base_cost: 54,
            extra_yields: YieldBundle::new().with(YieldType::Gold, 2),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![],
    }
}

pub fn france() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::France, leader: BuiltinLeader::Catherine,
        civ_name: "France", adjective: "French", leader_name: "Catherine de Medici",
        civ_ability_name: "Grand Tour",
        civ_ability_description: "+20% Production towards Medieval, Renaissance, and Industrial era wonders. Tourism is doubled for wonders.",
        leader_ability_name: "Catherine's Flying Squadron",
        leader_ability_description: "Has 1 extra level of Diplomatic Visibility with every civ she has met. Receives a free Spy with Castles.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Grand Tour"),
                TargetSelector::ProductionQueue,
                EffectType::ProductionPercent(20),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::France, name: "garde_imperiale", replaces: None,
            production_cost: 340, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(65), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::France, name: "Chateau",
            base_yields: YieldBundle::new().with(YieldType::Culture, 2).with(YieldType::Gold, 1),
            appeal_modifier: 1,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![],
    }
}

pub fn kongo() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Kongo, leader: BuiltinLeader::Mvemba,
        civ_name: "Kongo", adjective: "Kongolese", leader_name: "Mvemba a Nzinga",
        civ_ability_name: "Nkisi",
        civ_ability_description: "+2 Food, +2 Production, and +4 Gold from each Relic, Artifact, and Great Work of Sculpture. +50% Great Writer and Artist points from Palace and cultural buildings.",
        leader_ability_name: "Religious Convert",
        leader_ability_description: "May not build Holy Sites or earn Great Prophets. Receives all beliefs of any religion that has established itself in a majority of his cities.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Nkisi"),
                TargetSelector::Global,
                EffectType::YieldFlat(YieldType::Food, 2),
                StackingRule::Additive,
            ),
            Modifier::new(
                ModifierSource::CivAbility("Nkisi"),
                TargetSelector::Global,
                EffectType::YieldFlat(YieldType::Production, 2),
                StackingRule::Additive,
            ),
            Modifier::new(
                ModifierSource::CivAbility("Nkisi"),
                TargetSelector::Global,
                EffectType::YieldFlat(YieldType::Gold, 4),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Kongo, name: "ngao_mbeba", replaces: Some("swordsman"),
            production_cost: 110, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(38), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        // Mbanza replaces the Neighborhood district, which is not yet in BuiltinDistrict.
        unique_district: None,
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::NoGreatProphets],
    }
}

pub fn norway() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Norway, leader: BuiltinLeader::Harald,
        civ_name: "Norway", adjective: "Norwegian", leader_name: "Harald Hardrada",
        civ_ability_name: "Knarr",
        civ_ability_description: "Units may enter ocean tiles with Shipbuilding. Naval melee units can heal in neutral territory. +50% Production towards naval melee units.",
        leader_ability_name: "Thunderbolt of the North",
        leader_ability_description: "Naval melee units gain the ability to perform coastal raids. +50% Production for naval melee units.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Knarr"),
                TargetSelector::ProductionQueue,
                EffectType::ProductionPercent(50),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Norway, name: "berserker", replaces: None,
            production_cost: 160, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(40), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Norway, name: "stave_church", replaces: Some("temple"),
            cost: 120, maintenance: 2,
            yields: YieldBundle::new().with(YieldType::Faith, 3),
            requires_district: Some("Holy Site"),
            extra_housing: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::EarlyOceanTravel],
    }
}

pub fn russia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Russia, leader: BuiltinLeader::Peter,
        civ_name: "Russia", adjective: "Russian", leader_name: "Peter",
        civ_ability_name: "Mother Russia",
        civ_ability_description: "Founded cities automatically claim extra territory. +1 Faith and +1 Production from Tundra tiles.",
        leader_ability_name: "The Grand Embassy",
        leader_ability_description: "Trade routes to more advanced civilizations grant +1 Science for every 3 technologies that civ is ahead, and +1 Culture for every 3 civics.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Mother Russia"),
                TargetSelector::AllTiles,
                EffectType::YieldFlat(YieldType::Faith, 1),
                StackingRule::Additive,
            ).with_condition(Condition::TileHasFeature(crate::world::feature::BuiltinFeature::Forest)),
            Modifier::new(
                ModifierSource::CivAbility("Mother Russia"),
                TargetSelector::AllTiles,
                EffectType::YieldFlat(YieldType::Production, 1),
                StackingRule::Additive,
            ).with_condition(Condition::TileHasFeature(crate::world::feature::BuiltinFeature::Forest)),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Russia, name: "cossack", replaces: Some("cavalry"),
            production_cost: 340, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 500, combat_strength: Some(67), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Russia, name: "Lavra",
            replaces: BuiltinDistrict::HolySite,
            base_cost: 54, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::ExtraTerritoryOnFounding(3)],
    }
}

pub fn scythia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Scythia, leader: BuiltinLeader::Tomyris,
        civ_name: "Scythia", adjective: "Scythian", leader_name: "Tomyris",
        civ_ability_name: "People of the Steppe",
        civ_ability_description: "Building a light cavalry unit or Saka Horse Archer grants a second copy of that unit for free.",
        leader_ability_name: "Killer of Cyrus",
        leader_ability_description: "All units receive +5 Combat Strength when attacking wounded units. When they eliminate a unit they heal 30 HP.",
        civ_modifiers: vec![],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Tomyris"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(5),
                StackingRule::Additive,
            ),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Scythia, name: "saka_horse_archer", replaces: None,
            production_cost: 100, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(20), ranged_strength: Some(25),
            range: 1, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Scythia, name: "Kurgan",
            base_yields: YieldBundle::new().with(YieldType::Gold, 3).with(YieldType::Faith, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![
                ImprovementAdjacencyBonus {
                    adjacent_to: ImprovementAdjacencySource::Pasture,
                    yield_type: YieldType::Gold,
                    amount: 1,
                },
            ],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::DoubleUnitProduction("saka_horse_archer")],
    }
}

pub fn spain() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Spain, leader: BuiltinLeader::Philip,
        civ_name: "Spain", adjective: "Spanish", leader_name: "Philip II",
        civ_ability_name: "Treasure Fleet",
        civ_ability_description: "Trade routes between cities on different continents receive bonus Gold. Can build Missions with a Builder.",
        leader_ability_name: "El Escorial",
        leader_ability_description: "Inquisitors gain +1 removal charge. Combat and Religious units gain +4 Combat Strength when on the same continent as their capital.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Treasure Fleet"),
                TargetSelector::TradeRoutesOwned,
                EffectType::TradeRouteYieldFlat(YieldType::Gold, 3),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Philip II"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(4),
                StackingRule::Additive,
            ),
        ],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Spain, name: "conquistador", replaces: None,
            production_cost: 250, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(58), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Spain, name: "Mission",
            base_yields: YieldBundle::new().with(YieldType::Faith, 2).with(YieldType::Science, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::InterContinentalTradeBonus],
    }
}

pub fn sumeria() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Sumeria, leader: BuiltinLeader::Gilgamesh,
        civ_name: "Sumeria", adjective: "Sumerian", leader_name: "Gilgamesh",
        civ_ability_name: "Epic Quest",
        civ_ability_description: "When you capture a Barbarian Outpost, you also receive a Tribal Village reward.",
        leader_ability_name: "Adventures with Enkidu",
        leader_ability_description: "Can declare War of Liberation at any time. Allied units share combat experience when within 5 tiles.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Sumeria, name: "war_cart", replaces: None,
            production_cost: 55, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 300, combat_strength: Some(30), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Sumeria, name: "Ziggurat",
            base_yields: YieldBundle::new().with(YieldType::Science, 2).with(YieldType::Culture, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::BonusOnBarbarianCampCapture],
    }
}
