//! Civilization ability bundle — all unique mechanics for a civ as data.

use crate::civ::civ_identity::{BuiltinCiv, BuiltinLeader};
use crate::rules::modifier::Modifier;
use crate::rules::unique::{
    UniqueBuildingDef, UniqueDistrictDef, UniqueImprovementDef, UniqueUnitDef,
};
use crate::PolicyType;

/// Hook that fires when a city is founded.
#[derive(Debug, Clone)]
pub enum CityFoundedHook {
    /// Grant a free building by name (e.g., Trajan: "monument").
    FreeBuilding(&'static str),
    /// City starts with a trading post (Rome).
    FreeTradingPost,
    /// Build a road from the new city to the capital (Rome).
    RoadToCapital,
}

/// Special rule overrides that modify core engine behaviour.
#[derive(Debug, Clone)]
pub enum RuleOverride {
    /// Eurekas grant 100% of tech cost instead of 50% (Babylon).
    EurekaGivesFullTech,
    /// Multiply science per turn by this percentage (-50 = halved) (Babylon).
    SciencePerTurnMultiplier(i32),
    /// Extra district slots beyond population limit (Germany: +1).
    ExtraDistrictSlot(i32),
    /// Extra policy slot of the given type (Greece: Wildcard, Barbarossa: Military).
    ExtraPolicySlot(PolicyType),
    /// Immunity to flood damage (Egypt).
    ImmunityToFloodDamage,
    /// Automatically receive the last available Great Prophet (Arabia).
    AutoLastGreatProphet,
    /// Receive follower beliefs from all religions with followers in cities (India).
    FollowerBeliefsFromAllReligions,
    /// Enemies receive double war weariness (Gandhi).
    DoubleWarWearinessToEnemies,
    /// First specialty district of each type gives a free lowest-cost building (Hammurabi).
    FirstDistrictGivesFreeBuildingAndEnvoy,
}

/// All unique components and abilities for a civilization, encoded as data.
#[derive(Debug)]
pub struct CivAbilityBundle {
    pub civ: BuiltinCiv,
    pub leader: BuiltinLeader,
    pub civ_name: &'static str,
    pub adjective: &'static str,
    pub leader_name: &'static str,

    pub civ_ability_name: &'static str,
    pub civ_ability_description: &'static str,
    pub leader_ability_name: &'static str,
    pub leader_ability_description: &'static str,

    /// Modifiers always active for this civilization (civ ability).
    pub civ_modifiers: Vec<Modifier>,
    /// Modifiers from the leader ability.
    pub leader_modifiers: Vec<Modifier>,

    pub unique_unit: Option<UniqueUnitDef>,
    pub unique_district: Option<UniqueDistrictDef>,
    pub unique_building: Option<UniqueBuildingDef>,
    pub unique_improvement: Option<UniqueImprovementDef>,

    /// Hooks that fire when a city is founded.
    pub on_city_founded: Vec<CityFoundedHook>,
    /// Special rule overrides.
    pub rule_overrides: Vec<RuleOverride>,
}
