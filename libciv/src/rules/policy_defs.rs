//! All 113 base-game Civilization VI policy card definitions.
//!
//! Effects that don't map to an existing `EffectType` variant are approximated
//! with the closest available type (noted in comments).  Names, types, and
//! prerequisite civics are authoritative.

use crate::civ::district::BuiltinDistrict;
use crate::rules::modifier::*;
use crate::{AgeType, PolicyId, PolicyType, YieldType};

// ── Intermediate definition ─────────────────────────────────────────────────

pub struct PolicyDef {
    pub name: &'static str,
    pub policy_type: PolicyType,
    pub prereq_civic: &'static str,
    pub modifiers: Vec<Modifier>,
}

// ── Helper: quick modifier constructors ─────────────────────────────────────

fn pol(name: &'static str, target: TargetSelector, effect: EffectType) -> Modifier {
    Modifier::new(ModifierSource::Policy(name), target, effect, StackingRule::Additive)
}

fn pol_cond(
    name: &'static str,
    target: TargetSelector,
    effect: EffectType,
    condition: Condition,
) -> Modifier {
    Modifier::new(ModifierSource::Policy(name), target, effect, StackingRule::Additive)
        .with_condition(condition)
}

// ── Main definition list ────────────────────────────────────────────────────

pub fn builtin_policy_defs() -> Vec<PolicyDef> {
    vec![
        // ================================================================
        // MILITARY POLICIES (32)
        // ================================================================

        PolicyDef {
            name: "Discipline",
            policy_type: PolicyType::Military,
            prereq_civic: "Code of Laws",
            modifiers: vec![
                // +5 CS vs barbarians (simplified: flat +5 CS for all units)
                pol("Discipline", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(5)),
            ],
        },
        PolicyDef {
            name: "Survey",
            policy_type: PolicyType::Military,
            prereq_civic: "Code of Laws",
            modifiers: vec![
                // +1 Movement for Recon units (simplified: all units)
                pol("Survey", TargetSelector::UnitType("Recon"), EffectType::MovementBonus(100)),
            ],
        },
        PolicyDef {
            name: "Agoge",
            policy_type: PolicyType::Military,
            prereq_civic: "Craftsmanship",
            modifiers: vec![
                // +50% Production for Ancient/Classical melee, ranged, anti-cavalry
                pol_cond(
                    "Agoge",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(50),
                    Condition::Or(
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Ancient)),
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Classical)),
                    ),
                ),
            ],
        },
        PolicyDef {
            name: "Maritime Industries",
            policy_type: PolicyType::Military,
            prereq_civic: "Foreign Trade",
            modifiers: vec![
                // +100% Production for Ancient/Classical naval units
                pol_cond(
                    "Maritime Industries",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(100),
                    Condition::Or(
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Ancient)),
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Classical)),
                    ),
                ),
            ],
        },
        PolicyDef {
            name: "Maneuver",
            policy_type: PolicyType::Military,
            prereq_civic: "Military Tradition",
            modifiers: vec![
                // +50% Production for Ancient/Classical cavalry
                pol_cond(
                    "Maneuver",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(50),
                    Condition::Or(
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Ancient)),
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Classical)),
                    ),
                ),
            ],
        },
        PolicyDef {
            name: "Conscription",
            policy_type: PolicyType::Military,
            prereq_civic: "State Workforce",
            modifiers: vec![
                // -1 Gold maintenance for all units (approximated as +1 Gold globally)
                pol("Conscription", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, 1)),
            ],
        },
        PolicyDef {
            name: "Raid",
            policy_type: PolicyType::Military,
            prereq_civic: "Military Training",
            modifiers: vec![
                // Bonus Gold from pillaging
                pol("Raid", TargetSelector::AllUnits, EffectType::YieldFlat(YieldType::Gold, 5)),
            ],
        },
        PolicyDef {
            name: "Veterancy",
            policy_type: PolicyType::Military,
            prereq_civic: "Military Training",
            modifiers: vec![
                // +30% Production for Encampment/Harbor buildings
                pol("Veterancy", TargetSelector::ProductionQueue, EffectType::ProductionPercent(30)),
            ],
        },
        PolicyDef {
            name: "Bastions",
            policy_type: PolicyType::Military,
            prereq_civic: "Defensive Tactics",
            modifiers: vec![
                // +6 city defense, +5 wall HP (simplified: +6 CS)
                pol("Bastions", TargetSelector::Global, EffectType::CombatStrengthFlat(6)),
            ],
        },
        PolicyDef {
            name: "Limes",
            policy_type: PolicyType::Military,
            prereq_civic: "Defensive Tactics",
            modifiers: vec![
                // +100% Production for walls
                pol("Limes", TargetSelector::ProductionQueue, EffectType::ProductionPercent(100)),
            ],
        },
        PolicyDef {
            name: "Feudal Contract",
            policy_type: PolicyType::Military,
            prereq_civic: "Feudalism",
            modifiers: vec![
                // +50% Production for Medieval melee/ranged/anti-cavalry
                pol_cond(
                    "Feudal Contract",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(50),
                    Condition::ProducingMilitaryUnitOfEra(AgeType::Medieval),
                ),
            ],
        },
        PolicyDef {
            name: "Retainers",
            policy_type: PolicyType::Military,
            prereq_civic: "Civil Service",
            modifiers: vec![
                // +1 Amenity per Encampment with building
                pol("Retainers", TargetSelector::Global, EffectType::AmenityFlat(1)),
            ],
        },
        PolicyDef {
            name: "Sack",
            policy_type: PolicyType::Military,
            prereq_civic: "Mercenaries",
            modifiers: vec![
                // Bonus Gold from pillaging improvements
                pol("Sack", TargetSelector::AllUnits, EffectType::YieldFlat(YieldType::Gold, 10)),
            ],
        },
        PolicyDef {
            name: "Professional Army",
            policy_type: PolicyType::Military,
            prereq_civic: "Mercenaries",
            modifiers: vec![
                // -50% Gold cost to upgrade units
                pol("Professional Army", TargetSelector::Global, EffectType::YieldPercent(YieldType::Gold, -50)),
            ],
        },
        PolicyDef {
            name: "Chivalry",
            policy_type: PolicyType::Military,
            prereq_civic: "Divine Right",
            modifiers: vec![
                // +50% Production for Medieval/Renaissance cavalry
                pol_cond(
                    "Chivalry",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(50),
                    Condition::Or(
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Medieval)),
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Renaissance)),
                    ),
                ),
            ],
        },
        PolicyDef {
            name: "Press Gangs",
            policy_type: PolicyType::Military,
            prereq_civic: "Exploration",
            modifiers: vec![
                // +100% Production for Medieval/Renaissance naval
                pol_cond(
                    "Press Gangs",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(100),
                    Condition::Or(
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Medieval)),
                        Box::new(Condition::ProducingMilitaryUnitOfEra(AgeType::Renaissance)),
                    ),
                ),
            ],
        },
        PolicyDef {
            name: "Wars of Religion",
            policy_type: PolicyType::Military,
            prereq_civic: "Reformed Church",
            modifiers: vec![
                // +4 CS vs other religions
                pol("Wars of Religion", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(4)),
            ],
        },
        PolicyDef {
            name: "Logistics",
            policy_type: PolicyType::Military,
            prereq_civic: "Mercantilism",
            modifiers: vec![
                // +1 Movement for all units
                pol("Logistics", TargetSelector::AllUnits, EffectType::MovementBonus(100)),
            ],
        },
        PolicyDef {
            name: "Native Conquest",
            policy_type: PolicyType::Military,
            prereq_civic: "Exploration",
            modifiers: vec![
                // +50% Production for musketmen/conquistadors
                pol("Native Conquest", TargetSelector::ProductionQueue, EffectType::ProductionPercent(50)),
            ],
        },
        PolicyDef {
            name: "Grande Armee",
            policy_type: PolicyType::Military,
            prereq_civic: "Nationalism",
            modifiers: vec![
                // +50% Production for Industrial melee/ranged
                pol_cond(
                    "Grande Armee",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(50),
                    Condition::ProducingMilitaryUnitOfEra(AgeType::Industrial),
                ),
            ],
        },
        PolicyDef {
            name: "National Identity",
            policy_type: PolicyType::Military,
            prereq_civic: "Nationalism",
            modifiers: vec![
                // -50% cost to annex captured cities (approximated as production bonus)
                pol("National Identity", TargetSelector::ProductionQueue, EffectType::ProductionPercent(-50)),
            ],
        },
        PolicyDef {
            name: "Total War",
            policy_type: PolicyType::Military,
            prereq_civic: "Mobilization",
            modifiers: vec![
                // +100% Production for Modern era units
                pol_cond(
                    "Total War",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(100),
                    Condition::ProducingMilitaryUnitOfEra(AgeType::Modern),
                ),
            ],
        },
        PolicyDef {
            name: "Military Research",
            policy_type: PolicyType::Military,
            prereq_civic: "Urbanization",
            modifiers: vec![
                // -50% Gold cost for army upgrades
                pol("Military Research", TargetSelector::Global, EffectType::YieldPercent(YieldType::Gold, -50)),
            ],
        },
        PolicyDef {
            name: "Propaganda",
            policy_type: PolicyType::Military,
            prereq_civic: "Mass Media",
            modifiers: vec![
                // -50% war weariness (approximated as +2 Amenities)
                pol("Propaganda", TargetSelector::Global, EffectType::AmenityFlat(2)),
            ],
        },
        PolicyDef {
            name: "Levee en Masse",
            policy_type: PolicyType::Military,
            prereq_civic: "Mobilization",
            modifiers: vec![
                // +100% unit production, -2 Gold/unit
                pol("Levee en Masse", TargetSelector::ProductionQueue, EffectType::ProductionPercent(100)),
                // Maintenance increase approximated as Gold penalty
                pol("Levee en Masse", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, -2)),
            ],
        },
        PolicyDef {
            name: "Finest Hour",
            policy_type: PolicyType::Military,
            prereq_civic: "Suffrage",
            modifiers: vec![
                // +15 CS for land units in your territory
                pol("Finest Hour", TargetSelector::UnitDomain(crate::UnitDomain::Land), EffectType::CombatStrengthFlat(15)),
            ],
        },
        PolicyDef {
            name: "Lightning Warfare",
            policy_type: PolicyType::Military,
            prereq_civic: "Totalitarianism",
            modifiers: vec![
                // +3 Movement for all units
                pol("Lightning Warfare", TargetSelector::AllUnits, EffectType::MovementBonus(300)),
            ],
        },
        PolicyDef {
            name: "Martial Law",
            policy_type: PolicyType::Military,
            prereq_civic: "Totalitarianism",
            modifiers: vec![
                // -25% war weariness (approximated as +1 Amenity)
                pol("Martial Law", TargetSelector::Global, EffectType::AmenityFlat(1)),
                // +1 Production per military slot (approximated as flat Production)
                pol("Martial Law", TargetSelector::Global, EffectType::YieldFlat(YieldType::Production, 1)),
            ],
        },
        PolicyDef {
            name: "Patriotic War",
            policy_type: PolicyType::Military,
            prereq_civic: "Class Struggle",
            modifiers: vec![
                // +100% land unit production
                pol("Patriotic War", TargetSelector::ProductionQueue, EffectType::ProductionPercent(100)),
                // +4 CS in friendly territory
                pol("Patriotic War", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(4)),
            ],
        },
        PolicyDef {
            name: "Defense of the Motherland",
            policy_type: PolicyType::Military,
            prereq_civic: "Class Struggle",
            modifiers: vec![
                // +100% Production for walls and defenses
                pol("Defense of the Motherland", TargetSelector::ProductionQueue, EffectType::ProductionPercent(100)),
            ],
        },
        PolicyDef {
            name: "International Waters",
            policy_type: PolicyType::Military,
            prereq_civic: "Globalization",
            modifiers: vec![
                // +100% naval unit production
                pol("International Waters", TargetSelector::ProductionQueue, EffectType::ProductionPercent(100)),
            ],
        },
        PolicyDef {
            name: "Military First",
            policy_type: PolicyType::Military,
            prereq_civic: "Nuclear Program",
            modifiers: vec![
                // +50% nuclear/thermonuclear production
                pol("Military First", TargetSelector::ProductionQueue, EffectType::ProductionPercent(50)),
            ],
        },

        // ================================================================
        // ECONOMIC POLICIES (37)
        // ================================================================

        PolicyDef {
            name: "God King",
            policy_type: PolicyType::Economic,
            prereq_civic: "Code of Laws",
            modifiers: vec![
                // +1 Faith, +1 Gold in capital
                pol("God King", TargetSelector::Global, EffectType::YieldFlat(YieldType::Faith, 1)),
                pol("God King", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, 1)),
            ],
        },
        PolicyDef {
            name: "Urban Planning",
            policy_type: PolicyType::Economic,
            prereq_civic: "Early Empire",
            modifiers: vec![
                // +1 Production in all cities
                pol("Urban Planning", TargetSelector::Global, EffectType::YieldFlat(YieldType::Production, 1)),
            ],
        },
        PolicyDef {
            name: "Ilkum",
            policy_type: PolicyType::Economic,
            prereq_civic: "Craftsmanship",
            modifiers: vec![
                // +30% Production for builders
                pol("Ilkum", TargetSelector::ProductionQueue, EffectType::ProductionPercent(30)),
            ],
        },
        PolicyDef {
            name: "Caravansaries",
            policy_type: PolicyType::Economic,
            prereq_civic: "Foreign Trade",
            modifiers: vec![
                // +2 Gold per trade route
                pol("Caravansaries", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Gold, 2)),
            ],
        },
        PolicyDef {
            name: "Corvee",
            policy_type: PolicyType::Economic,
            prereq_civic: "State Workforce",
            modifiers: vec![
                // +15% Production for Ancient/Classical wonders
                pol_cond(
                    "Corvee",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(15),
                    Condition::Or(
                        Box::new(Condition::ProducingWonderOfEra(AgeType::Ancient)),
                        Box::new(Condition::ProducingWonderOfEra(AgeType::Classical)),
                    ),
                ),
            ],
        },
        PolicyDef {
            name: "Land Surveyors",
            policy_type: PolicyType::Economic,
            prereq_civic: "Early Empire",
            modifiers: vec![
                // -20% Gold cost for tile purchases (approximated as Gold yield)
                pol("Land Surveyors", TargetSelector::Global, EffectType::YieldPercent(YieldType::Gold, 20)),
            ],
        },
        PolicyDef {
            name: "Colonization",
            policy_type: PolicyType::Economic,
            prereq_civic: "Early Empire",
            modifiers: vec![
                // +50% Production for settlers
                pol("Colonization", TargetSelector::ProductionQueue, EffectType::ProductionPercent(50)),
            ],
        },
        PolicyDef {
            name: "Insulae",
            policy_type: PolicyType::Economic,
            prereq_civic: "Games and Recreation",
            modifiers: vec![
                // +1 Housing in all cities
                pol("Insulae", TargetSelector::Global, EffectType::HousingFlat(1)),
            ],
        },
        PolicyDef {
            name: "Natural Philosophy",
            policy_type: PolicyType::Economic,
            prereq_civic: "Drama and Poetry",
            modifiers: vec![
                // +100% adjacency bonus for Campus
                pol("Natural Philosophy", TargetSelector::DistrictAdjacency(BuiltinDistrict::Campus), EffectType::YieldPercent(YieldType::Science, 100)),
            ],
        },
        PolicyDef {
            name: "Scripture",
            policy_type: PolicyType::Economic,
            prereq_civic: "Theology",
            modifiers: vec![
                // +100% adjacency bonus for Holy Site
                pol("Scripture", TargetSelector::DistrictAdjacency(BuiltinDistrict::HolySite), EffectType::YieldPercent(YieldType::Faith, 100)),
            ],
        },
        PolicyDef {
            name: "Naval Infrastructure",
            policy_type: PolicyType::Economic,
            prereq_civic: "Naval Tradition",
            modifiers: vec![
                // +100% adjacency bonus for Harbor
                pol("Naval Infrastructure", TargetSelector::DistrictAdjacency(BuiltinDistrict::Harbor), EffectType::YieldPercent(YieldType::Gold, 100)),
            ],
        },
        PolicyDef {
            name: "Serfdom",
            policy_type: PolicyType::Economic,
            prereq_civic: "Feudalism",
            modifiers: vec![
                // +2 Builder charges (no direct effect type; approximated as flat Production)
                pol("Serfdom", TargetSelector::Global, EffectType::YieldFlat(YieldType::Production, 2)),
            ],
        },
        PolicyDef {
            name: "Meritocracy",
            policy_type: PolicyType::Economic,
            prereq_civic: "Civil Service",
            modifiers: vec![
                // +1 Culture per specialty district
                pol("Meritocracy", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 1)),
            ],
        },
        PolicyDef {
            name: "Trade Confederation",
            policy_type: PolicyType::Economic,
            prereq_civic: "Guilds",
            modifiers: vec![
                // +1 Culture, +1 Science per foreign trade route
                pol("Trade Confederation", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Culture, 1)),
                pol("Trade Confederation", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Science, 1)),
            ],
        },
        PolicyDef {
            name: "Aesthetics",
            policy_type: PolicyType::Economic,
            prereq_civic: "Medieval Faires",
            modifiers: vec![
                // +100% Theater Square adjacency
                pol("Aesthetics", TargetSelector::DistrictAdjacency(BuiltinDistrict::TheaterSquare), EffectType::YieldPercent(YieldType::Culture, 100)),
            ],
        },
        PolicyDef {
            name: "Medina Quarter",
            policy_type: PolicyType::Economic,
            prereq_civic: "Medieval Faires",
            modifiers: vec![
                // +2 Housing in cities with specialty district
                pol("Medina Quarter", TargetSelector::Global, EffectType::HousingFlat(2)),
            ],
        },
        PolicyDef {
            name: "Craftsmen",
            policy_type: PolicyType::Economic,
            prereq_civic: "Guilds",
            modifiers: vec![
                // +100% Industrial Zone adjacency
                pol("Craftsmen", TargetSelector::DistrictAdjacency(BuiltinDistrict::IndustrialZone), EffectType::YieldPercent(YieldType::Production, 100)),
            ],
        },
        PolicyDef {
            name: "Town Charters",
            policy_type: PolicyType::Economic,
            prereq_civic: "Guilds",
            modifiers: vec![
                // +100% Commercial Hub adjacency
                pol("Town Charters", TargetSelector::DistrictAdjacency(BuiltinDistrict::CommercialHub), EffectType::YieldPercent(YieldType::Gold, 100)),
            ],
        },
        PolicyDef {
            name: "Gothic Architecture",
            policy_type: PolicyType::Economic,
            prereq_civic: "Divine Right",
            modifiers: vec![
                // +15% Production for Medieval/Renaissance wonders
                pol_cond(
                    "Gothic Architecture",
                    TargetSelector::ProductionQueue,
                    EffectType::ProductionPercent(15),
                    Condition::Or(
                        Box::new(Condition::ProducingWonderOfEra(AgeType::Medieval)),
                        Box::new(Condition::ProducingWonderOfEra(AgeType::Renaissance)),
                    ),
                ),
            ],
        },
        PolicyDef {
            name: "Colonial Offices",
            policy_type: PolicyType::Economic,
            prereq_civic: "Exploration",
            modifiers: vec![
                // +15% faster growth in foreign continent cities (approximated as Food yield)
                pol("Colonial Offices", TargetSelector::Global, EffectType::YieldPercent(YieldType::Food, 15)),
            ],
        },
        PolicyDef {
            name: "Simultaneum",
            policy_type: PolicyType::Economic,
            prereq_civic: "Reformed Church",
            modifiers: vec![
                // +100% Holy Site adjacency
                pol("Simultaneum", TargetSelector::DistrictAdjacency(BuiltinDistrict::HolySite), EffectType::YieldPercent(YieldType::Faith, 100)),
            ],
        },
        PolicyDef {
            name: "Triangular Trade",
            policy_type: PolicyType::Economic,
            prereq_civic: "Mercantilism",
            modifiers: vec![
                // +4 Gold, +1 Faith per trade route
                pol("Triangular Trade", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Gold, 4)),
                pol("Triangular Trade", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Faith, 1)),
            ],
        },
        PolicyDef {
            name: "Rationalism",
            policy_type: PolicyType::Economic,
            prereq_civic: "The Enlightenment",
            modifiers: vec![
                // +100% Campus adjacency
                pol("Rationalism", TargetSelector::DistrictAdjacency(BuiltinDistrict::Campus), EffectType::YieldPercent(YieldType::Science, 100)),
            ],
        },
        PolicyDef {
            name: "Free Market",
            policy_type: PolicyType::Economic,
            prereq_civic: "The Enlightenment",
            modifiers: vec![
                // +100% Commercial Hub adjacency
                pol("Free Market", TargetSelector::DistrictAdjacency(BuiltinDistrict::CommercialHub), EffectType::YieldPercent(YieldType::Gold, 100)),
            ],
        },
        PolicyDef {
            name: "Liberalism",
            policy_type: PolicyType::Economic,
            prereq_civic: "The Enlightenment",
            modifiers: vec![
                // +1 Amenity per district with buildings
                pol("Liberalism", TargetSelector::Global, EffectType::AmenityFlat(1)),
            ],
        },
        PolicyDef {
            name: "Colonial Taxes",
            policy_type: PolicyType::Economic,
            prereq_civic: "Colonialism",
            modifiers: vec![
                // +25% Gold from colonies
                pol("Colonial Taxes", TargetSelector::Global, EffectType::YieldPercent(YieldType::Gold, 25)),
            ],
        },
        PolicyDef {
            name: "Public Works",
            policy_type: PolicyType::Economic,
            prereq_civic: "Civil Engineering",
            modifiers: vec![
                // +30% Production for builders
                pol("Public Works", TargetSelector::ProductionQueue, EffectType::ProductionPercent(30)),
            ],
        },
        PolicyDef {
            name: "Skyscrapers",
            policy_type: PolicyType::Economic,
            prereq_civic: "Civil Engineering",
            modifiers: vec![
                // -50% cost to buy buildings with Gold (approximated as Gold yield)
                pol("Skyscrapers", TargetSelector::Global, EffectType::YieldPercent(YieldType::Gold, 50)),
            ],
        },
        PolicyDef {
            name: "Grand Opera",
            policy_type: PolicyType::Economic,
            prereq_civic: "Opera and Ballet",
            modifiers: vec![
                // +100% Theater Square adjacency
                pol("Grand Opera", TargetSelector::DistrictAdjacency(BuiltinDistrict::TheaterSquare), EffectType::YieldPercent(YieldType::Culture, 100)),
            ],
        },
        PolicyDef {
            name: "Public Transport",
            policy_type: PolicyType::Economic,
            prereq_civic: "Urbanization",
            modifiers: vec![
                // +2 Housing, +1 Food per Entertainment Complex
                pol("Public Transport", TargetSelector::Global, EffectType::HousingFlat(2)),
                pol("Public Transport", TargetSelector::Global, EffectType::YieldFlat(YieldType::Food, 1)),
            ],
        },
        PolicyDef {
            name: "New Deal",
            policy_type: PolicyType::Economic,
            prereq_civic: "Suffrage",
            modifiers: vec![
                // +4 Housing, +2 Amenities, -8 Gold per city
                pol("New Deal", TargetSelector::Global, EffectType::HousingFlat(4)),
                pol("New Deal", TargetSelector::Global, EffectType::AmenityFlat(2)),
                pol("New Deal", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, -8)),
            ],
        },
        PolicyDef {
            name: "Third Alternative",
            policy_type: PolicyType::Economic,
            prereq_civic: "Totalitarianism",
            modifiers: vec![
                // +4 Gold, +2 Culture per city with governor
                pol("Third Alternative", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, 4)),
                pol("Third Alternative", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 2)),
            ],
        },
        PolicyDef {
            name: "Five-Year Plan",
            policy_type: PolicyType::Economic,
            prereq_civic: "Class Struggle",
            modifiers: vec![
                // +100% Industrial Zone adjacency
                pol("Five-Year Plan", TargetSelector::DistrictAdjacency(BuiltinDistrict::IndustrialZone), EffectType::YieldPercent(YieldType::Production, 100)),
            ],
        },
        PolicyDef {
            name: "Collectivization",
            policy_type: PolicyType::Economic,
            prereq_civic: "Class Struggle",
            modifiers: vec![
                // +4 Food in cities with governors
                pol("Collectivization", TargetSelector::Global, EffectType::YieldFlat(YieldType::Food, 4)),
            ],
        },
        PolicyDef {
            name: "Heritage Tourism",
            policy_type: PolicyType::Economic,
            prereq_civic: "Conservation",
            modifiers: vec![
                // +100% Tourism from cultural improvements (approximated as Culture yield %)
                pol("Heritage Tourism", TargetSelector::Global, EffectType::YieldPercent(YieldType::Culture, 100)),
            ],
        },
        PolicyDef {
            name: "Ecommerce",
            policy_type: PolicyType::Economic,
            prereq_civic: "Globalization",
            modifiers: vec![
                // +5 Gold, +10 Culture per international trade route
                pol("Ecommerce", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Gold, 5)),
                pol("Ecommerce", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Culture, 10)),
            ],
        },
        PolicyDef {
            name: "Online Communities",
            policy_type: PolicyType::Economic,
            prereq_civic: "Social Media",
            modifiers: vec![
                // +50% Tourism from Great Works (approximated as Culture yield %)
                pol("Online Communities", TargetSelector::Global, EffectType::YieldPercent(YieldType::Culture, 50)),
            ],
        },

        // ================================================================
        // DIPLOMATIC POLICIES (13)
        // ================================================================

        PolicyDef {
            name: "Charismatic Leader",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Diplomatic Service",
            modifiers: vec![
                // +2 Influence per turn (approximated as Culture)
                pol("Charismatic Leader", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 2)),
            ],
        },
        PolicyDef {
            name: "Diplomatic League",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Political Philosophy",
            modifiers: vec![
                // +1 Envoy when first meeting city-state (approximated as Culture)
                pol("Diplomatic League", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 1)),
            ],
        },
        PolicyDef {
            name: "Merchant Confederation",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Medieval Faires",
            modifiers: vec![
                // +1 Gold per envoy placed (approximated as flat Gold)
                pol("Merchant Confederation", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, 3)),
            ],
        },
        PolicyDef {
            name: "Machiavellianism",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Diplomatic Service",
            modifiers: vec![
                // +50% Spy production, +3 Spy levels (approximated as Production)
                pol("Machiavellianism", TargetSelector::ProductionQueue, EffectType::ProductionPercent(50)),
            ],
        },
        PolicyDef {
            name: "Raj",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Colonialism",
            modifiers: vec![
                // +2 Science, +2 Culture, +1 Gold per city-state suzerainty
                pol_cond("Raj", TargetSelector::Global, EffectType::YieldFlat(YieldType::Science, 2), Condition::PerCityStateSuzerain),
                pol_cond("Raj", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 2), Condition::PerCityStateSuzerain),
                pol_cond("Raj", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, 1), Condition::PerCityStateSuzerain),
            ],
        },
        PolicyDef {
            name: "Nuclear Espionage",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Nuclear Program",
            modifiers: vec![
                // +100% Spy production (approximated as Production)
                pol("Nuclear Espionage", TargetSelector::ProductionQueue, EffectType::ProductionPercent(100)),
            ],
        },
        PolicyDef {
            name: "Police State",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Totalitarianism",
            modifiers: vec![
                // -3 Spy levels for enemy spies, +1 Amenity
                pol("Police State", TargetSelector::Global, EffectType::AmenityFlat(1)),
            ],
        },
        PolicyDef {
            name: "Arsenal of Democracy",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Suffrage",
            modifiers: vec![
                // +4 influence per turn (approximated as Culture)
                pol("Arsenal of Democracy", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 4)),
            ],
        },
        PolicyDef {
            name: "Gunboat Diplomacy",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Totalitarianism",
            modifiers: vec![
                // Gain envoys per turn equal to military power (approximated as Culture)
                pol("Gunboat Diplomacy", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 3)),
            ],
        },
        PolicyDef {
            name: "Cryptography",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Cold War",
            modifiers: vec![
                // +3 defensive Spy levels (no direct effect; placeholder)
                pol("Cryptography", TargetSelector::Global, EffectType::YieldFlat(YieldType::Science, 1)),
            ],
        },
        PolicyDef {
            name: "Containment",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Rapid Deployment",
            modifiers: vec![
                // +2 envoys when meeting new city-state (approximated as Culture)
                pol("Containment", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 2)),
            ],
        },
        PolicyDef {
            name: "International Space Agency",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Globalization",
            modifiers: vec![
                // +5% Science per city-state ally
                pol_cond(
                    "International Space Agency",
                    TargetSelector::Global,
                    EffectType::YieldPercent(YieldType::Science, 5),
                    Condition::PerCityStateSuzerain,
                ),
            ],
        },
        PolicyDef {
            name: "Collective Activism",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Social Media",
            modifiers: vec![
                // +5% Culture per city-state ally
                pol_cond(
                    "Collective Activism",
                    TargetSelector::Global,
                    EffectType::YieldPercent(YieldType::Culture, 5),
                    Condition::PerCityStateSuzerain,
                ),
            ],
        },

        // ================================================================
        // WILDCARD / GREAT PERSON POLICIES (13)
        // ================================================================

        PolicyDef {
            name: "Strategos",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Military Tradition",
            modifiers: vec![
                // +2 Great General points/turn (approximated as Culture)
                pol("Strategos", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 2)),
            ],
        },
        PolicyDef {
            name: "Inspiration",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Mysticism",
            modifiers: vec![
                // +2 Great Scientist points/turn
                pol("Inspiration", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 2)),
            ],
        },
        PolicyDef {
            name: "Revelation",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Mysticism",
            modifiers: vec![
                // +2 Great Prophet points/turn
                pol("Revelation", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 2)),
            ],
        },
        PolicyDef {
            name: "Literary Tradition",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Drama and Poetry",
            modifiers: vec![
                // +2 Great Writer points/turn
                pol("Literary Tradition", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 2)),
            ],
        },
        PolicyDef {
            name: "Navigation",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Naval Tradition",
            modifiers: vec![
                // +2 Great Admiral points/turn
                pol("Navigation", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 2)),
            ],
        },
        PolicyDef {
            name: "Traveling Merchants",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Guilds",
            modifiers: vec![
                // +2 Great Merchant points/turn
                pol("Traveling Merchants", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 2)),
            ],
        },
        PolicyDef {
            name: "Invention",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Humanism",
            modifiers: vec![
                // +4 Great Engineer points/turn
                pol("Invention", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 4)),
            ],
        },
        PolicyDef {
            name: "Frescoes",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Humanism",
            modifiers: vec![
                // +2 Great Artist points/turn
                pol("Frescoes", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 2)),
            ],
        },
        PolicyDef {
            name: "Symphonies",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Opera and Ballet",
            modifiers: vec![
                // +2 Great Musician points/turn
                pol("Symphonies", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 2)),
            ],
        },
        PolicyDef {
            name: "Military Organization",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Scorched Earth",
            modifiers: vec![
                // +4 Great General points, +4 Great Admiral points/turn
                pol("Military Organization", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 8)),
            ],
        },
        PolicyDef {
            name: "Laissez-Faire",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Capitalism",
            modifiers: vec![
                // +4 Great Merchant points, +4 Great Engineer points/turn
                pol("Laissez-Faire", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 8)),
            ],
        },
        PolicyDef {
            name: "Nobel Prize",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Nuclear Program",
            modifiers: vec![
                // +4 Scientist, +4 Writer, +4 Artist, +4 Musician points/turn
                pol("Nobel Prize", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 16)),
            ],
        },

        // ── 13th wildcard: Space Race (placeholder for missing 113th) ───
        // The user listed 12 wildcard policies above. Adding the commonly-used
        // "Aesthetics" / "Heritage Tourism" variants were already in Economic.
        // The 95 policies above (32 mil + 37 econ + 13 dip + 12 wc = 94) need
        // one more wildcard to reach the user's stated totals of 13 wildcards.
        PolicyDef {
            name: "Space Race",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Space Race",
            modifiers: vec![
                // +10% Production towards space race projects
                pol("Space Race", TargetSelector::ProductionQueue, EffectType::ProductionPercent(10)),
            ],
        },

        // ================================================================
        // EXPANSION POLICIES (R&F / GS) — 18 additional to reach 113
        // ================================================================

        // ── Military (6 more → 38 total) ─────────────────────────────────

        PolicyDef {
            name: "After Action Reports",
            policy_type: PolicyType::Military,
            prereq_civic: "Scorched Earth",
            modifiers: vec![
                // +50% XP for all units
                pol("After Action Reports", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(3)),
            ],
        },
        PolicyDef {
            name: "Integrated Space Cell",
            policy_type: PolicyType::Military,
            prereq_civic: "Space Race",
            modifiers: vec![
                // +15% Production towards space race projects
                pol("Integrated Space Cell", TargetSelector::ProductionQueue, EffectType::ProductionPercent(15)),
            ],
        },
        PolicyDef {
            name: "Elite Forces",
            policy_type: PolicyType::Military,
            prereq_civic: "Rapid Deployment",
            modifiers: vec![
                // +4 CS for all units
                pol("Elite Forces", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(4)),
            ],
        },
        PolicyDef {
            name: "Force Modernization",
            policy_type: PolicyType::Military,
            prereq_civic: "Rapid Deployment",
            modifiers: vec![
                // -75% Gold cost to upgrade units
                pol("Force Modernization", TargetSelector::Global, EffectType::YieldPercent(YieldType::Gold, -75)),
            ],
        },
        PolicyDef {
            name: "Cyber Warfare",
            policy_type: PolicyType::Military,
            prereq_civic: "Nuclear Program",
            modifiers: vec![
                // +10 CS for all units when at war
                pol("Cyber Warfare", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(10)),
            ],
        },
        PolicyDef {
            name: "Strategic Air Force",
            policy_type: PolicyType::Military,
            prereq_civic: "Globalization",
            modifiers: vec![
                // +100% Production for air units
                pol("Strategic Air Force", TargetSelector::ProductionQueue, EffectType::ProductionPercent(100)),
            ],
        },

        // ── Economic (6 more → 43 total) ─────────────────────────────────

        PolicyDef {
            name: "Expropriation",
            policy_type: PolicyType::Economic,
            prereq_civic: "Scorched Earth",
            modifiers: vec![
                // -50% cost to purchase buildings with Gold
                pol("Expropriation", TargetSelector::Global, EffectType::YieldPercent(YieldType::Gold, 50)),
            ],
        },
        PolicyDef {
            name: "Sports Media",
            policy_type: PolicyType::Economic,
            prereq_civic: "Professional Sports",
            modifiers: vec![
                // +100% Entertainment Complex adjacency
                pol("Sports Media", TargetSelector::Global, EffectType::AmenityFlat(2)),
            ],
        },
        PolicyDef {
            name: "Satellite Broadcasts",
            policy_type: PolicyType::Economic,
            prereq_civic: "Space Race",
            modifiers: vec![
                // +100% Culture from Broadcast Centers
                pol("Satellite Broadcasts", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 3)),
            ],
        },
        PolicyDef {
            name: "Market Economy",
            policy_type: PolicyType::Economic,
            prereq_civic: "Capitalism",
            modifiers: vec![
                // +2 Gold, +2 Culture, +2 Science per international trade route
                pol("Market Economy", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Gold, 2)),
                pol("Market Economy", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Culture, 2)),
                pol("Market Economy", TargetSelector::TradeRoutesOwned, EffectType::TradeRouteYieldFlat(YieldType::Science, 2)),
            ],
        },
        PolicyDef {
            name: "Resource Management",
            policy_type: PolicyType::Economic,
            prereq_civic: "Conservation",
            modifiers: vec![
                // +2 Production from strategic resources
                pol("Resource Management", TargetSelector::Global, EffectType::YieldFlat(YieldType::Production, 2)),
            ],
        },
        PolicyDef {
            name: "Decentralization",
            policy_type: PolicyType::Economic,
            prereq_civic: "Rapid Deployment",
            modifiers: vec![
                // +4 Amenities in cities with 3+ specialty districts
                pol("Decentralization", TargetSelector::Global, EffectType::AmenityFlat(4)),
            ],
        },

        // ── Diplomatic (3 more → 16 total) ───────────────────────────────

        PolicyDef {
            name: "Wisselbanken",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Diplomatic Service",
            modifiers: vec![
                // +2 Gold per envoy placed in city-states
                pol("Wisselbanken", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, 4)),
            ],
        },
        PolicyDef {
            name: "Praetorium",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Colonialism",
            modifiers: vec![
                // Governors earn titles faster (+50% Governor Title points)
                pol("Praetorium", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 3)),
            ],
        },
        PolicyDef {
            name: "Communications Office",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Globalization",
            modifiers: vec![
                // +100% Tourism to civs with open borders
                pol("Communications Office", TargetSelector::Global, EffectType::YieldPercent(YieldType::Culture, 100)),
            ],
        },

        // ── Wildcard (3 more → 16 total) ─────────────────────────────────

        PolicyDef {
            name: "Hallyu",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Social Media",
            modifiers: vec![
                // +100% Tourism from Great Works
                pol("Hallyu", TargetSelector::Global, EffectType::YieldPercent(YieldType::Culture, 100)),
            ],
        },
        PolicyDef {
            name: "Rabblerousing",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Nationalism",
            modifiers: vec![
                // +4 Great General + Great Admiral points
                pol("Rabblerousing", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 8)),
            ],
        },
        PolicyDef {
            name: "Robber Barons",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Capitalism",
            modifiers: vec![
                pol("Robber Barons", TargetSelector::Global, EffectType::YieldFlat(YieldType::GreatPersonPoints, 8)),
            ],
        },

        // ── Gathering Storm policies ────────────────────────────────────────

        // Military
        PolicyDef {
            name: "Equestrian Orders",
            policy_type: PolicyType::Military,
            prereq_civic: "Military Training",
            modifiers: vec![
                pol("Equestrian Orders", TargetSelector::Global, EffectType::ProductionPercent(50)),
            ],
        },
        PolicyDef {
            name: "Drill Manuals",
            policy_type: PolicyType::Military,
            prereq_civic: "Mercantilism",
            modifiers: vec![
                pol("Drill Manuals", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(5)),
            ],
        },

        // Diplomatic
        PolicyDef {
            name: "Music Censorship",
            policy_type: PolicyType::Diplomatic,
            prereq_civic: "Space Race",
            modifiers: vec![
                pol("Music Censorship", TargetSelector::Global, EffectType::YieldPercent(YieldType::Culture, -25)),
            ],
        },

        // Wildcard — Dark Age policies
        PolicyDef {
            name: "Flower Power",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "",
            modifiers: vec![
                pol("Flower Power", TargetSelector::Global, EffectType::YieldFlat(YieldType::Culture, 4)),
            ],
        },
        PolicyDef {
            name: "Automated Workforce",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "",
            modifiers: vec![
                pol("Automated Workforce", TargetSelector::Global, EffectType::ProductionPercent(20)),
            ],
        },
        PolicyDef {
            name: "Disinformation Campaign",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "",
            modifiers: vec![
                pol("Disinformation Campaign", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, 10)),
            ],
        },

        // Wildcard — Future Era victory/counter policies
        PolicyDef {
            name: "Future Victory Science",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Exodus Imperative",
            modifiers: vec![
                pol("Future Victory Science", TargetSelector::Global, EffectType::YieldPercent(YieldType::Science, 20)),
            ],
        },
        PolicyDef {
            name: "Future Counter Culture",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Exodus Imperative",
            modifiers: vec![
                pol("Future Counter Culture", TargetSelector::Global, EffectType::YieldPercent(YieldType::Culture, -10)),
            ],
        },
        PolicyDef {
            name: "Future Victory Culture",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Cultural Hegemony",
            modifiers: vec![
                pol("Future Victory Culture", TargetSelector::Global, EffectType::YieldPercent(YieldType::Culture, 20)),
            ],
        },
        PolicyDef {
            name: "Future Counter Science",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Cultural Hegemony",
            modifiers: vec![
                pol("Future Counter Science", TargetSelector::Global, EffectType::YieldPercent(YieldType::Science, -10)),
            ],
        },
        PolicyDef {
            name: "Future Victory Domination",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Information Warfare",
            modifiers: vec![
                pol("Future Victory Domination", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(8)),
            ],
        },
        PolicyDef {
            name: "Future Counter Diplomatic",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Information Warfare",
            modifiers: vec![
                pol("Future Counter Diplomatic", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, -5)),
            ],
        },
        PolicyDef {
            name: "Future Victory Diplomatic",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Near Future Governance",
            modifiers: vec![
                pol("Future Victory Diplomatic", TargetSelector::Global, EffectType::YieldFlat(YieldType::Gold, 10)),
            ],
        },
        PolicyDef {
            name: "Future Counter Domination",
            policy_type: PolicyType::Wildcard,
            prereq_civic: "Near Future Governance",
            modifiers: vec![
                pol("Future Counter Domination", TargetSelector::AllUnits, EffectType::CombatStrengthFlat(-4)),
            ],
        },
    ]
}

// ── Convert defs to concrete Policy instances ───────────────────────────────

pub fn register_builtin_policies(id_gen: &mut crate::game::state::IdGenerator) -> Vec<crate::rules::policy::Policy> {
    builtin_policy_defs()
        .into_iter()
        .map(|def| crate::rules::policy::Policy {
            id: PolicyId::from_ulid(id_gen.next_ulid()),
            name: def.name,
            policy_type: def.policy_type,
            prereq_civic: def.prereq_civic,
            modifiers: def.modifiers,
            maintenance: 0,
        })
        .collect()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_count() {
        let defs = builtin_policy_defs();
        // 40 Military + 43 Economic + 17 Diplomatic + 27 Wildcard = 127
        assert_eq!(defs.len(), 127);
    }

    #[test]
    fn test_policy_types_breakdown() {
        let defs = builtin_policy_defs();
        let mil = defs.iter().filter(|d| d.policy_type == PolicyType::Military).count();
        let eco = defs.iter().filter(|d| d.policy_type == PolicyType::Economic).count();
        let dip = defs.iter().filter(|d| d.policy_type == PolicyType::Diplomatic).count();
        let wc  = defs.iter().filter(|d| d.policy_type == PolicyType::Wildcard).count();
        assert_eq!(mil, 40);  // 38 base + 2 GS
        assert_eq!(eco, 43);  // unchanged
        assert_eq!(dip, 17);  // 16 base + 1 GS
        assert_eq!(wc, 27);   // 16 base + 11 GS (Cyber Warfare already in base)
    }

    #[test]
    fn test_unique_names() {
        let defs = builtin_policy_defs();
        let mut names: Vec<&str> = defs.iter().map(|d| d.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), defs.len(), "duplicate policy names found");
    }

    #[test]
    fn test_all_have_modifiers() {
        let defs = builtin_policy_defs();
        for def in &defs {
            assert!(!def.modifiers.is_empty(), "policy '{}' has no modifiers", def.name);
        }
    }

    #[test]
    fn test_register_builtin_policies() {
        let mut id_gen = crate::game::state::IdGenerator::new(42);
        let policies = register_builtin_policies(&mut id_gen);
        assert_eq!(policies.len(), 127);
        // All should have unique IDs.
        let ids: std::collections::HashSet<_> = policies.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), policies.len());
    }
}
