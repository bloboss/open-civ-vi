//! Static data definitions for all 19 base-game civilizations.

use crate::civ::civ_ability::*;
use crate::civ::civ_identity::*;
use crate::civ::district::BuiltinDistrict;
use crate::rules::modifier::*;
use crate::rules::unique::*;
use crate::{PolicyType, UnitCategory, UnitDomain, YieldBundle, YieldType};

/// Return ability bundles for all 50 civilizations (19 base-game + 8 Gathering Storm + 8 Rise & Fall + 15 DLC).
pub fn all_civ_bundles() -> Vec<CivAbilityBundle> {
    vec![
        rome(), greece(), egypt(), babylon(), germany(), japan(), india(), arabia(),
        america(), brazil(), china(), england(), france(), kongo(), norway(), russia(),
        scythia(), spain(), sumeria(),
        // Gathering Storm
        canada(), hungary(), inca(), mali(), maori(), ottoman(), phoenicia(), sweden(),
        // Rise & Fall
        cree(), georgia(), korea(), mapuche(), mongolia(), netherlands(), scotland(), zulu(),
        // DLC Civilization Packs
        australia(), aztec(), byzantium(), gaul(), ethiopia(), gran_colombia(), maya(),
        indonesia(), khmer(), vietnam(), macedon(), persia(), nubia(), poland(), portugal(),
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
            requires_district: None, extra_housing: 1, extra_amenities: 0,
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
            extra_housing: 0, extra_amenities: 0,
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
            extra_housing: 1, extra_amenities: 0,
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
            extra_housing: 0, extra_amenities: 0,
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
            extra_housing: 0, extra_amenities: 0,
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

// ── Gathering Storm civilizations ───────────────────────────────────────────

pub fn canada() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Canada, leader: BuiltinLeader::Laurier,
        civ_name: "Canada", adjective: "Canadian", leader_name: "Wilfrid Laurier",
        civ_ability_name: "Four Faces of Peace",
        civ_ability_description: "Cannot declare or be the target of Surprise Wars. +100% diplomatic favor from suzerainties.",
        leader_ability_name: "The Last Best West",
        leader_ability_description: "Can build Farms on Tundra tiles. Purchasing Snow and Tundra tiles is 50% cheaper.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Canada, name: "mountie", replaces: None,
            production_cost: 290, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 500, combat_strength: Some(62), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Canada, name: "Hockey Rink",
            base_yields: YieldBundle::new().with(YieldType::Culture, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::DiplomaticFavorFromSuzerainties(100)],
    }
}

pub fn hungary() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Hungary, leader: BuiltinLeader::MatthiasCorvinus,
        civ_name: "Hungary", adjective: "Hungarian", leader_name: "Matthias Corvinus",
        civ_ability_name: "Pearl of the Danube",
        civ_ability_description: "+50% Production for buildings and districts built across a river from a City Center.",
        leader_ability_name: "Raven King",
        leader_ability_description: "Levied city-state units gain +2 Movement and +5 Combat Strength. It costs 75% less Gold and resources to upgrade levied units.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Pearl of the Danube"),
                TargetSelector::ProductionQueue,
                EffectType::ProductionPercent(50),
                StackingRule::Additive,
            ).with_condition(Condition::AdjacentToRiver),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Hungary, name: "huszar", replaces: None,
            production_cost: 335, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 500, combat_strength: Some(65), ranged_strength: None,
            range: 0, vision_range: 2,
            resource_cost: Some((crate::world::resource::BuiltinResource::Horses, 10)),
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Hungary, name: "thermal_bath", replaces: Some("zoo"),
            cost: 360, maintenance: 1,
            yields: YieldBundle::default(),
            requires_district: Some("Entertainment Complex"),
            extra_housing: 0, extra_amenities: 2,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::ProductionBonusAcrossRiver(50)],
    }
}

pub fn inca() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Inca, leader: BuiltinLeader::Pachacuti,
        civ_name: "Inca", adjective: "Incan", leader_name: "Pachacuti",
        civ_ability_name: "Mit'a",
        civ_ability_description: "Citizens can work Mountain tiles. Mountain tiles provide +2 Production. Domestic Trade Routes gain +1 Food for every Mountain tile in the origin city.",
        leader_ability_name: "Qhapaq Nan",
        leader_ability_description: "Domestic Trade Routes gain +1 Food for every Mountain tile in the origin city.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Inca, name: "warakaq", replaces: None,
            production_cost: 165, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 300, combat_strength: Some(20), ranged_strength: Some(40),
            range: 1, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Inca, name: "Terrace Farm",
            base_yields: YieldBundle::new().with(YieldType::Food, 1).with(YieldType::Housing, 2),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::CanWorkMountains],
    }
}

pub fn mali() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Mali, leader: BuiltinLeader::MansaMusa,
        civ_name: "Mali", adjective: "Malian", leader_name: "Mansa Musa",
        civ_ability_name: "Songs of the Jeli",
        civ_ability_description: "City Centers gain +1 Faith and +1 Food for every adjacent Desert or Desert Hills tile. Mines receive -1 Production and +4 Gold. -30% Production toward training units and constructing buildings.",
        leader_ability_name: "Sahel Merchants",
        leader_ability_description: "International Trade Routes gain +1 Gold for every flat Desert tile in the origin city.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Mali, name: "mandekalu_cavalry", replaces: None,
            production_cost: 220, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(55), ranged_strength: None,
            range: 0, vision_range: 2,
            resource_cost: Some((crate::world::resource::BuiltinResource::Iron, 10)),
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Mali, name: "Suguba",
            replaces: BuiltinDistrict::CommercialHub,
            base_cost: 27, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::MineGoldBonusProductionMalus { mine_gold: 4, production_percent: -30 }],
    }
}

pub fn maori() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Maori, leader: BuiltinLeader::Kupe,
        civ_name: "Maori", adjective: "Maori", leader_name: "Kupe",
        civ_ability_name: "Mana",
        civ_ability_description: "Unimproved Woods and Rainforest tiles gain +1 Production and +1 Faith. Fishing Boats gain +1 Food. Cannot harvest features or earn Great Writers.",
        leader_ability_name: "Kupe's Voyage",
        leader_ability_description: "Start the game in the Ocean. Gain +2 Science and +2 Culture per turn until you settle your first city.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Maori, name: "toa", replaces: None,
            production_cost: 120, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(38), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Maori, name: "marae", replaces: Some("amphitheater"),
            cost: 150, maintenance: 1,
            yields: YieldBundle::new().with(YieldType::Culture, 2),
            requires_district: Some("Theater Square"),
            extra_housing: 0, extra_amenities: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::UnimprovedFeatureProductionBonus(2)],
    }
}

pub fn ottoman() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Ottoman, leader: BuiltinLeader::Suleiman,
        civ_name: "Ottoman", adjective: "Ottoman", leader_name: "Suleiman",
        civ_ability_name: "Great Turkish Bombard",
        civ_ability_description: "+50% Production toward siege units. Conquered cities do not lose population. +1 Amenity and +4 Loyalty in conquered cities.",
        leader_ability_name: "Grand Vizier",
        leader_ability_description: "Has access to a unique Governor, Ibrahim. The Janissary unique unit is available.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Great Turkish Bombard"),
                TargetSelector::ProductionQueue,
                EffectType::ProductionPercent(50),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Ottoman, name: "barbary_corsair", replaces: None,
            production_cost: 240, domain: UnitDomain::Sea, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(40), ranged_strength: Some(50),
            range: 1, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Ottoman, name: "grand_bazaar", replaces: Some("bank"),
            cost: 220, maintenance: 0,
            yields: YieldBundle::default(),
            requires_district: Some("Commercial Hub"),
            extra_housing: 0, extra_amenities: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::SiegeProductionAndLoyalty { siege_percent: 50 }],
    }
}

pub fn phoenicia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Phoenicia, leader: BuiltinLeader::Dido,
        civ_name: "Phoenicia", adjective: "Phoenician", leader_name: "Dido",
        civ_ability_name: "Mediterranean Colonies",
        civ_ability_description: "Starts with the Eureka for Writing. Coastal cities founded by Phoenicia on the same continent as their Capital always have full Loyalty.",
        leader_ability_name: "Founder of Carthage",
        leader_ability_description: "Can move the Capital to any city with a Cothon. +1 Trade Route capacity after building the Government Plaza.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Phoenicia, name: "bireme", replaces: Some("galley"),
            production_cost: 65, domain: UnitDomain::Sea, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(35), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Phoenicia, name: "Cothon",
            replaces: BuiltinDistrict::Harbor,
            base_cost: 27, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::SameContinentLoyalty],
    }
}

pub fn sweden() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Sweden, leader: BuiltinLeader::Kristina,
        civ_name: "Sweden", adjective: "Swedish", leader_name: "Kristina",
        civ_ability_name: "Nobel Prize",
        civ_ability_description: "+50 Diplomatic Favor whenever a Great Person is earned. +50% Great Person points from each type of district.",
        leader_ability_name: "Minerva of the North",
        leader_ability_description: "Buildings with at least 3 Great Work slots and Wonders with at least 2 Great Work slots are automatically themed when all their slots are filled.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Sweden, name: "carolean", replaces: None,
            production_cost: 250, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 300, combat_strength: Some(55), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Sweden, name: "Open-Air Museum",
            base_yields: YieldBundle::new().with(YieldType::Culture, 2),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::DiplomaticFavorOnGreatPerson(50)],
    }
}

// ── Rise & Fall civilizations ─────────────────────────────────────────────

pub fn cree() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Cree, leader: BuiltinLeader::Poundmaker,
        civ_name: "Cree", adjective: "Cree", leader_name: "Poundmaker",
        civ_ability_name: "Nihithaw",
        civ_ability_description: "Gain a free Trader when Pottery is researched. +1 Food to all camps.",
        leader_ability_name: "Favorable Terms",
        leader_ability_description: "All Alliance types provide Shared Visibility. +1 Food and +1 Gold from Trade Routes per camp or pasture in the sending city.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Cree, name: "okihtcitaw", replaces: Some("scout"),
            production_cost: 40, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 300, combat_strength: Some(20), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Cree, name: "Mekewap",
            base_yields: YieldBundle::new().with(YieldType::Gold, 2).with(YieldType::Housing, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::FreeTraderOnPottery],
    }
}

pub fn georgia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Georgia, leader: BuiltinLeader::Tamar,
        civ_name: "Georgia", adjective: "Georgian", leader_name: "Tamar",
        civ_ability_name: "Strength in Unity",
        civ_ability_description: "+faith from walls. Protectorate wars gain no grievances.",
        leader_ability_name: "Glory of the World, Kingdom, and Faith",
        leader_ability_description: "+100% Faith for 10 turns after declaring a Protectorate War. Each Envoy sent to a city-state of your majority Religion counts as two.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Georgia, name: "khevsureti", replaces: None,
            production_cost: 160, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(48), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Georgia, name: "tsikhe", replaces: Some("renaissance_walls"),
            cost: 240, maintenance: 0,
            yields: YieldBundle::new().with(YieldType::Faith, 3),
            requires_district: None,
            extra_housing: 0, extra_amenities: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::FaithFromWallsNoProtectorateGrievances],
    }
}

pub fn korea() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Korea, leader: BuiltinLeader::Seondeok,
        civ_name: "Korea", adjective: "Korean", leader_name: "Seondeok",
        civ_ability_name: "Three Kingdoms",
        civ_ability_description: "+science per adjacency from Seowon. Farms adjacent to Seowon +1 food.",
        leader_ability_name: "Hwarang",
        leader_ability_description: "+3% Culture and +3% Science from each promoted Governor.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Korea, name: "hwacha", replaces: None,
            production_cost: 250, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(45), ranged_strength: Some(60),
            range: 2, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Korea, name: "Seowon",
            replaces: BuiltinDistrict::Campus,
            base_cost: 27, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::SeowonAdjacentFarmBonus(1)],
    }
}

pub fn mapuche() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Mapuche, leader: BuiltinLeader::Lautaro,
        civ_name: "Mapuche", adjective: "Mapuche", leader_name: "Lautaro",
        civ_ability_name: "Toqui",
        civ_ability_description: "+10 Combat Strength bonus against civilizations that are in a Golden Age. +5 loyalty per turn in cities with an established Governor.",
        leader_ability_name: "Swift Hawk",
        leader_ability_description: "Defeating an enemy unit within the borders of an enemy city reduces that city's Loyalty by 20.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Mapuche, name: "malon_raider", replaces: None,
            production_cost: 230, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(55), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Mapuche, name: "Chemamull",
            base_yields: YieldBundle::new().with(YieldType::Culture, 1).with(YieldType::Production, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::DefeatReducesLoyalty],
    }
}

pub fn mongolia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Mongolia, leader: BuiltinLeader::GenghisKhan,
        civ_name: "Mongolia", adjective: "Mongolian", leader_name: "Genghis Khan",
        civ_ability_name: "Ortoo",
        civ_ability_description: "Sending or receiving a Trade Route immediately grants a level of Diplomatic Visibility. +6 Combat Strength for all cavalry class units for each level of Diplomatic Visibility the Mongolians have over their opponents.",
        leader_ability_name: "Mongol Horde",
        leader_ability_description: "+3 Combat Strength for all cavalry class units. Capturing a cavalry unit has a chance to convert it to your side.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::Leader("Genghis Khan"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(3),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Mongolia, name: "keshig", replaces: None,
            production_cost: 160, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(35), ranged_strength: Some(45),
            range: 2, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Mongolia, name: "ordu", replaces: Some("stable"),
            cost: 120, maintenance: 1,
            yields: YieldBundle::default(),
            requires_district: Some("Encampment"),
            extra_housing: 0, extra_amenities: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![],
    }
}

pub fn netherlands() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Netherlands, leader: BuiltinLeader::Wilhelmina,
        civ_name: "Netherlands", adjective: "Dutch", leader_name: "Wilhelmina",
        civ_ability_name: "Grote Rivieren",
        civ_ability_description: "+50% production for Campus, Harbor, Industrial Zone, and Theater Square districts built on a river.",
        leader_ability_name: "Radio Oranje",
        leader_ability_description: "+2 Culture to domestic trade routes. +4 Culture to international trade routes.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Grote Rivieren"),
                TargetSelector::ProductionQueue,
                EffectType::ProductionPercent(50),
                StackingRule::Additive,
            ).with_condition(Condition::AdjacentToRiver),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Netherlands, name: "de_zeven_provincien", replaces: None,
            production_cost: 280, domain: UnitDomain::Sea, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(50), ranged_strength: Some(60),
            range: 2, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Netherlands, name: "Polder",
            base_yields: YieldBundle::new().with(YieldType::Food, 1).with(YieldType::Production, 1).with(YieldType::Gold, 2),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::RiverDistrictProductionBonus(50)],
    }
}

pub fn scotland() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Scotland, leader: BuiltinLeader::RobertTheBruce,
        civ_name: "Scotland", adjective: "Scottish", leader_name: "Robert the Bruce",
        civ_ability_name: "Scottish Enlightenment",
        civ_ability_description: "+5% Science and +5% Production in happy cities. +10% in ecstatic cities. Great Scientist points from Campuses doubled in happy cities.",
        leader_ability_name: "Bannockburn",
        leader_ability_description: "Can declare a War of Liberation after gaining the Defensive Tactics civic. +100% Production and +2 Movement for 10 turns after declaring a War of Liberation.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Scotland, name: "highlander", replaces: None,
            production_cost: 380, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 300, combat_strength: Some(50), ranged_strength: Some(65),
            range: 1, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Scotland, name: "Golf Course",
            base_yields: YieldBundle::new().with(YieldType::Gold, 2).with(YieldType::Culture, 1),
            appeal_modifier: 1,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::HappyCityBonus { science_percent: 5, production_percent: 5 }],
    }
}

pub fn zulu() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Zulu, leader: BuiltinLeader::Shaka,
        civ_name: "Zulu", adjective: "Zulu", leader_name: "Shaka",
        civ_ability_name: "Isibongo",
        civ_ability_description: "Units can form corps and armies earlier (after Mercenaries and Nationalism instead of Nationalism and Mobilization).",
        leader_ability_name: "Amabutho",
        leader_ability_description: "May form Corps with Mercenaries instead of Nationalism. Armies with Nationalism instead of Mobilization. +5 Combat Strength to Corps and Armies.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Zulu, name: "impi", replaces: None,
            production_cost: 125, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(45), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Zulu, name: "Ikanda",
            replaces: BuiltinDistrict::Encampment,
            base_cost: 27, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::EarlyCorpsAndArmies],
    }
}

// ── DLC Civilization Pack civilizations ──────────────────────────────────────

pub fn australia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Australia, leader: BuiltinLeader::JohnCurtin,
        civ_name: "Australia", adjective: "Australian", leader_name: "John Curtin",
        civ_ability_name: "Land Down Under",
        civ_ability_description: "Pastures trigger a Culture Bomb. Campus, Commercial Hub, Holy Site, and Theater Square districts gain +3 in tiles with Charming Appeal and +6 in Breathtaking.",
        leader_ability_name: "Citadel of Civilization",
        leader_ability_description: "+100% production for 10 turns when liberating cities or when another civ declares war on you.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Australia, name: "digger", replaces: Some("infantry"),
            production_cost: 430, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(78), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Australia, name: "Outback Station",
            base_yields: YieldBundle::new().with(YieldType::Food, 1).with(YieldType::Production, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::ProductionBonusOnLiberation(100)],
    }
}

pub fn aztec() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Aztec, leader: BuiltinLeader::Montezuma,
        civ_name: "Aztec", adjective: "Aztec", leader_name: "Montezuma",
        civ_ability_name: "Legend of the Five Suns",
        civ_ability_description: "Can spend Builder charges to complete 20% of a district's Production cost.",
        leader_ability_name: "Gifts for the Tlatoani",
        leader_ability_description: "Luxury resources provide +1 Amenity to 2 extra cities. Military units receive +1 Combat Strength for each different luxury owned.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Aztec, name: "eagle_warrior", replaces: Some("warrior"),
            production_cost: 65, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(28), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Aztec, name: "tlachtli", replaces: Some("arena"),
            cost: 150, maintenance: 1,
            yields: YieldBundle::new().with(YieldType::Faith, 2).with(YieldType::Culture, 1),
            requires_district: Some("Entertainment Complex"),
            extra_housing: 0, extra_amenities: 1,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::LuxuryExtraAmenity],
    }
}

pub fn byzantium() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Byzantium, leader: BuiltinLeader::BasilII,
        civ_name: "Byzantium", adjective: "Byzantine", leader_name: "Basil II",
        civ_ability_name: "Taxis",
        civ_ability_description: "+3 Combat Strength for each Holy City converted to Byzantium's Religion. Gain the Hippodrome unique district.",
        leader_ability_name: "Porphyrogennetos",
        leader_ability_description: "Heavy and light cavalry units do full damage to cities following Byzantium's Religion. Gain the Tagma unique unit.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Taxis"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(3),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Byzantium, name: "dromon", replaces: Some("quadrireme"),
            production_cost: 120, domain: UnitDomain::Sea, category: UnitCategory::Combat,
            max_movement: 300, combat_strength: Some(25), ranged_strength: Some(40),
            range: 2, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Byzantium, name: "Hippodrome",
            replaces: BuiltinDistrict::EntertainmentComplex,
            base_cost: 27, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::CombatBonusVsDifferentReligion(3)],
    }
}

pub fn gaul() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Gaul, leader: BuiltinLeader::Ambiorix,
        civ_name: "Gaul", adjective: "Gallic", leader_name: "Ambiorix",
        civ_ability_name: "Hallstatt Culture",
        civ_ability_description: "Mines provide a minor adjacency bonus for all specialty districts. Specialty districts cannot be built adjacent to the City Center.",
        leader_ability_name: "King of the Eburones",
        leader_ability_description: "Adjacent combat units gain Culture equal to 20% of the unit's Combat Strength when a non-civilian is trained. +2 Combat Strength to all melee, anti-cavalry, and ranged units for each adjacent combat unit.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Gaul, name: "gaesatae", replaces: Some("warrior"),
            production_cost: 50, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(20), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Gaul, name: "Oppidum",
            replaces: BuiltinDistrict::IndustrialZone,
            base_cost: 27, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::AdjacentUnitBonusAndMineCulture],
    }
}

pub fn ethiopia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Ethiopia, leader: BuiltinLeader::MenelikII,
        civ_name: "Ethiopia", adjective: "Ethiopian", leader_name: "Menelik II",
        civ_ability_name: "Aksumite Legacy",
        civ_ability_description: "International trade routes grant +0.5 Faith per resource at the destination. Ethiopian cities on Hills receive +15% Science and Culture.",
        leader_ability_name: "Council of Ministers",
        leader_ability_description: "+15% Science and Culture in cities built on Hills.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Ethiopia, name: "oromo_cavalry", replaces: None,
            production_cost: 200, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(48), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Ethiopia, name: "Rock-Hewn Church",
            base_yields: YieldBundle::new().with(YieldType::Faith, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![
                ImprovementAdjacencyBonus {
                    adjacent_to: ImprovementAdjacencySource::HolySite,
                    yield_type: YieldType::Faith,
                    amount: 1,
                },
            ],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::HillsYieldBonus { science_percent: 15, culture_percent: 15 }],
    }
}

pub fn gran_colombia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::GranColombia, leader: BuiltinLeader::SimonBolivar,
        civ_name: "Gran Colombia", adjective: "Gran Colombian", leader_name: "Simon Bolivar",
        civ_ability_name: "Ejercito Patriota",
        civ_ability_description: "+1 Movement to all units. Promoting a unit does not end that unit's turn.",
        leader_ability_name: "Campana Admirable",
        leader_ability_description: "Earn a Comandante General, a unique Great General, each time a new era is entered.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::GranColombia, name: "llanero", replaces: None,
            production_cost: 330, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(62), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::GranColombia, name: "Hacienda",
            base_yields: YieldBundle::new().with(YieldType::Gold, 2).with(YieldType::Production, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::ExtraMovementAllUnits(1)],
    }
}

pub fn maya() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Maya, leader: BuiltinLeader::LadySixSky,
        civ_name: "Maya", adjective: "Mayan", leader_name: "Lady Six Sky",
        civ_ability_name: "Mayab",
        civ_ability_description: "Farms grant +1 Housing and +1 Gold. +5% yield for each city within 6 tiles of capital.",
        leader_ability_name: "Ix Mutal Ajaw",
        leader_ability_description: "Non-capital cities within 6 tiles of the Capital gain +10% to all yields. Cities outside of 6 tiles gain -15% to all yields.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Maya, name: "hulche", replaces: Some("archer"),
            production_cost: 60, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(28), ranged_strength: None,
            range: 2, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Maya, name: "Observatory",
            replaces: BuiltinDistrict::Campus,
            base_cost: 27, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::YieldBonusPerNearbyCity { percent_per_city: 5, max_range: 6 }],
    }
}

pub fn indonesia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Indonesia, leader: BuiltinLeader::Gitarja,
        civ_name: "Indonesia", adjective: "Indonesian", leader_name: "Gitarja",
        civ_ability_name: "Great Nusantara",
        civ_ability_description: "Coast and Lake tiles provide +0.5 Faith. Entertainment Complexes built adjacent to Coast or Lake tiles provide +1 Amenity.",
        leader_ability_name: "Exalted Goddess of the Three Worlds",
        leader_ability_description: "May purchase naval units with Faith. Religious units pay no movement to embark/disembark.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Indonesia, name: "jong", replaces: Some("frigate"),
            production_cost: 300, domain: UnitDomain::Sea, category: UnitCategory::Combat,
            max_movement: 500, combat_strength: Some(55), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Indonesia, name: "Kampung",
            base_yields: YieldBundle::new().with(YieldType::Food, 1).with(YieldType::Housing, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::CoastFaithAndNavalFaithPurchase],
    }
}

pub fn khmer() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Khmer, leader: BuiltinLeader::JayavarmanVII,
        civ_name: "Khmer", adjective: "Khmer", leader_name: "Jayavarman VII",
        civ_ability_name: "Grand Barays",
        civ_ability_description: "Aqueducts provide +2 Faith and +100% Faith to adjacent Holy Sites. Farms adjacent to Aqueducts provide +2 Food.",
        leader_ability_name: "Monasteries of the King",
        leader_ability_description: "Holy Sites provide +2 Food and +1 Housing. Culture Bomb adjacent tiles when a Holy Site is completed.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Khmer, name: "domrey", replaces: None,
            production_cost: 180, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(40), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Khmer, name: "prasat", replaces: Some("temple"),
            cost: 120, maintenance: 2,
            yields: YieldBundle::new().with(YieldType::Faith, 3),
            requires_district: Some("Holy Site"),
            extra_housing: 0, extra_amenities: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::HolySiteFoodAndHousing],
    }
}

pub fn vietnam() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Vietnam, leader: BuiltinLeader::BaTrieu,
        civ_name: "Vietnam", adjective: "Vietnamese", leader_name: "Ba Trieu",
        civ_ability_name: "Nine Dragon River Delta",
        civ_ability_description: "Land specialty districts can only be built on Rainforest, Marsh, or Woods. Buildings that remove features provide Culture. Woods cannot be removed.",
        leader_ability_name: "Drive Out the Aggressors",
        leader_ability_description: "Land units gain +5 Combat Strength in Rainforest, Marsh, or Woods tiles. +1 Movement in those features. These bonuses are doubled in Vietnamese territory.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Drive Out the Aggressors"),
                TargetSelector::AllUnits,
                EffectType::CombatStrengthFlat(5),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Vietnam, name: "voi_chien", replaces: None,
            production_cost: 200, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(40), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: Some(UniqueDistrictDef {
            civ: BuiltinCiv::Vietnam, name: "Thanh",
            replaces: BuiltinDistrict::Encampment,
            base_cost: 27, extra_yields: YieldBundle::default(),
            extra_housing: 0, extra_amenities: 0,
            placement: None, adjacency_overrides: vec![],
        }),
        unique_building: None,
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::TerrainDefenseBonus],
    }
}

pub fn macedon() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Macedon, leader: BuiltinLeader::Alexander,
        civ_name: "Macedon", adjective: "Macedonian", leader_name: "Alexander",
        civ_ability_name: "Hellenistic Fusion",
        civ_ability_description: "Conquering a city with an Encampment or Campus triggers a Eureka for each technology at that city's level. Same for Theater Square or Holy Site and Inspirations.",
        leader_ability_name: "To the World's End",
        leader_ability_description: "No war weariness. All military units heal when capturing a city with a Wonder. Cities do not lose loyalty when conquering.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Macedon, name: "hypaspist", replaces: Some("swordsman"),
            production_cost: 100, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(38), ranged_strength: None,
            range: 0, vision_range: 2,
            resource_cost: Some((crate::world::resource::BuiltinResource::Iron, 10)),
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Macedon, name: "basilikoi_paides", replaces: Some("barracks"),
            cost: 90, maintenance: 1,
            yields: YieldBundle::new().with(YieldType::Production, 1),
            requires_district: Some("Encampment"),
            extra_housing: 1, extra_amenities: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::NoWarWearinessNoConquestLoyaltyLoss],
    }
}

pub fn persia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Persia, leader: BuiltinLeader::Cyrus,
        civ_name: "Persia", adjective: "Persian", leader_name: "Cyrus",
        civ_ability_name: "Satrapies",
        civ_ability_description: "+1 Trade Route capacity with Political Philosophy. Domestic Trade Routes provide +2 Gold and +1 Culture. Roads built in your territory are one level more advanced.",
        leader_ability_name: "Fall of Babylon",
        leader_ability_description: "+2 Movement for all units for 10 turns after declaring a Surprise War. +5 Combat Strength for 10 turns after declaring a Surprise War. Declaring a Surprise War only counts as a formal war for the purposes of war weariness and warmongering.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Persia, name: "immortal", replaces: Some("swordsman"),
            production_cost: 100, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(25), ranged_strength: Some(35),
            range: 2, vision_range: 2,
            resource_cost: Some((crate::world::resource::BuiltinResource::Iron, 10)),
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Persia, name: "Pairidaeza",
            base_yields: YieldBundle::new().with(YieldType::Culture, 2).with(YieldType::Gold, 1),
            appeal_modifier: 1,
            adjacency_bonuses: vec![
                ImprovementAdjacencyBonus {
                    adjacent_to: ImprovementAdjacencySource::HolySite,
                    yield_type: YieldType::Culture,
                    amount: 1,
                },
            ],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::SurpriseWarBonus { movement: 1, combat_strength: 5, turns: 10 }],
    }
}

pub fn nubia() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Nubia, leader: BuiltinLeader::Amanitore,
        civ_name: "Nubia", adjective: "Nubian", leader_name: "Amanitore",
        civ_ability_name: "Ta-Seti",
        civ_ability_description: "+50% Production toward Ranged units. Ranged units gain +50% XP. Mines over strategic resources provide +1 Production. Mines over bonus/luxury resources provide +2 Gold.",
        leader_ability_name: "Kandake of Meroe",
        leader_ability_description: "+20% Production toward all districts rising to +40% if there is a Nubian Pyramid adjacent to the City Center.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Ta-Seti"),
                TargetSelector::ProductionQueue,
                EffectType::ProductionPercent(20),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Nubia, name: "pitati", replaces: Some("archer"),
            production_cost: 60, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 200, combat_strength: Some(30), ranged_strength: None,
            range: 2, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: None,
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Nubia, name: "Nubian Pyramid",
            base_yields: YieldBundle::new().with(YieldType::Faith, 2),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::RangedProductionBonusAndMineStrategic { ranged_percent: 20 }],
    }
}

pub fn poland() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Poland, leader: BuiltinLeader::Jadwiga,
        civ_name: "Poland", adjective: "Polish", leader_name: "Jadwiga",
        civ_ability_name: "Golden Liberty",
        civ_ability_description: "Culture Bomb adjacent tiles when completing an Encampment or Fort. One Military policy slot is converted to a Wildcard slot.",
        leader_ability_name: "Lithuanian Union",
        leader_ability_description: "Taking territory from another civ with a Culture Bomb converts that city to Poland's Religion. Holy Sites receive a +1 adjacency bonus from districts.",
        civ_modifiers: vec![],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Poland, name: "hussar", replaces: None,
            production_cost: 330, domain: UnitDomain::Land, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(64), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Poland, name: "sukiennice", replaces: Some("market"),
            cost: 120, maintenance: 0,
            yields: YieldBundle::new().with(YieldType::Gold, 3).with(YieldType::Production, 2),
            requires_district: Some("Commercial Hub"),
            extra_housing: 0, extra_amenities: 0,
            abilities: vec![],
        }),
        unique_improvement: None,
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::CultureBombOnEncampmentOrFort],
    }
}

pub fn portugal() -> CivAbilityBundle {
    CivAbilityBundle {
        civ: BuiltinCiv::Portugal, leader: BuiltinLeader::JoaoIII,
        civ_name: "Portugal", adjective: "Portuguese", leader_name: "Joao III",
        civ_ability_name: "Casa Da India",
        civ_ability_description: "International Trade Routes can only reach cities on the coast or with a Harbor. +50% yields for all international Trade Routes.",
        leader_ability_name: "Porta Do Cerco",
        leader_ability_description: "Open Borders provides +25% to all international Trade Route yields. +1 sight for all units.",
        civ_modifiers: vec![
            Modifier::new(
                ModifierSource::CivAbility("Casa Da India"),
                TargetSelector::TradeRoutesOwned,
                EffectType::TradeRouteYieldFlat(YieldType::Gold, 4),
                StackingRule::Additive,
            ),
        ],
        leader_modifiers: vec![],
        unique_unit: Some(UniqueUnitDef {
            civ: BuiltinCiv::Portugal, name: "nau", replaces: Some("caravel"),
            production_cost: 240, domain: UnitDomain::Sea, category: UnitCategory::Combat,
            max_movement: 400, combat_strength: Some(55), ranged_strength: None,
            range: 0, vision_range: 2, resource_cost: None,
            abilities: vec![],
        }),
        unique_district: None,
        unique_building: Some(UniqueBuildingDef {
            civ: BuiltinCiv::Portugal, name: "navigation_school", replaces: Some("university"),
            cost: 250, maintenance: 2,
            yields: YieldBundle::new().with(YieldType::Science, 4),
            requires_district: Some("Campus"),
            extra_housing: 1, extra_amenities: 0,
            abilities: vec![],
        }),
        unique_improvement: Some(UniqueImprovementDef {
            civ: BuiltinCiv::Portugal, name: "Feitoria",
            base_yields: YieldBundle::new().with(YieldType::Gold, 4).with(YieldType::Production, 1),
            appeal_modifier: 0,
            adjacency_bonuses: vec![],
        }),
        on_city_founded: vec![],
        rule_overrides: vec![RuleOverride::InternationalTradeYieldBonus(50)],
    }
}
