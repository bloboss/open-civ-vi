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
    /// Government legacy bonuses are earned at double speed (America).
    LegacyBonusesFaster,
    /// Eurekas and inspirations grant +10% bonus (China).
    EurekaInspirationBonus(i32),
    /// Extra territory on city founding (Russia).
    ExtraTerritoryOnFounding(u32),
    /// Build two light cavalry or unique unit at once (Scythia).
    DoubleUnitProduction(&'static str),
    /// +yield to intercontinental trade routes (Spain).
    InterContinentalTradeBonus,
    /// Bonus reward from tribal villages on barbarian camp capture (Sumeria).
    BonusOnBarbarianCampCapture,
    /// Cannot earn Great Prophets but gets bonus great works slots (Kongo).
    NoGreatProphets,
    /// Ocean travel unlocked early; +50% XP for naval melee (Norway).
    EarlyOceanTravel,
    // ── Gathering Storm ─────────────────────────────────────────────────────
    /// +100% diplomatic favor from suzerainties (Canada).
    DiplomaticFavorFromSuzerainties(i32),
    /// +50% production for buildings across river from city center (Hungary).
    ProductionBonusAcrossRiver(i32),
    /// Citizens can work mountain tiles (Inca).
    CanWorkMountains,
    /// Mines +4 Gold, -30% unit/building production (Mali).
    MineGoldBonusProductionMalus { mine_gold: i32, production_percent: i32 },
    /// Unimproved features +2 Production (Maori).
    UnimprovedFeatureProductionBonus(i32),
    /// +50% siege production, conquered cities no loyalty loss (Ottoman).
    SiegeProductionAndLoyalty { siege_percent: i32 },
    /// 100% loyalty in cities on same continent as capital (Phoenicia).
    SameContinentLoyalty,
    /// +50 diplomatic favor on great person recruitment (Sweden).
    DiplomaticFavorOnGreatPerson(i32),
    // ── Rise & Fall ────────────────────────────────────────────────────────
    /// Free trader when Pottery researched (Cree).
    FreeTraderOnPottery,
    /// +faith from walls; protectorate wars gain no grievances (Georgia).
    FaithFromWallsNoProtectorateGrievances,
    /// Farms adjacent to Seowon +1 food (Korea).
    SeowonAdjacentFarmBonus(i32),
    /// Defeating units in enemy territory reduces city loyalty (Mapuche).
    DefeatReducesLoyalty,
    /// +50% production for campus/harbor/industrial/theater on river (Netherlands).
    RiverDistrictProductionBonus(i32),
    /// +5% science and +5% production in happy cities (Scotland).
    HappyCityBonus { science_percent: i32, production_percent: i32 },
    /// Units can form corps/armies earlier (Zulu).
    EarlyCorpsAndArmies,
    // ── DLC Civilization Packs ──────────────────────────────────────────────
    /// +100% production when liberating cities or at war after being declared on (Australia).
    ProductionBonusOnLiberation(i32),
    /// Luxury resources provide +1 amenity to extra cities (Aztec).
    LuxuryExtraAmenity,
    /// +3 Combat Strength when attacking units following a different religion (Byzantium).
    CombatBonusVsDifferentReligion(i32),
    /// Adjacent units gain bonus CS; mines +1 Culture (Gaul).
    AdjacentUnitBonusAndMineCulture,
    /// +15% science/culture on hills (Ethiopia).
    HillsYieldBonus { science_percent: i32, culture_percent: i32 },
    /// +1 movement for all units (Gran Colombia).
    ExtraMovementAllUnits(i32),
    /// +5% yield per city within 6 tiles of capital (Maya).
    YieldBonusPerNearbyCity { percent_per_city: i32, max_range: u32 },
    /// Coast tiles provide faith; can purchase naval units with faith (Indonesia).
    CoastFaithAndNavalFaithPurchase,
    /// Holy Sites provide food and housing (Khmer).
    HolySiteFoodAndHousing,
    /// Land units gain bonuses in forest/jungle/marsh (Vietnam).
    TerrainDefenseBonus,
    /// No war weariness; cities don't lose loyalty when conquering (Macedon).
    NoWarWearinessNoConquestLoyaltyLoss,
    /// +1 movement and +5 CS for 10 turns after declaring surprise war (Persia).
    SurpriseWarBonus { movement: i32, combat_strength: i32, turns: u32 },
    /// +20% production for ranged units; mines over strategic +1 production (Nubia).
    RangedProductionBonusAndMineStrategic { ranged_percent: i32 },
    /// Culture bomb adjacent tiles when completing Encampment/Fort (Poland).
    CultureBombOnEncampmentOrFort,
    /// International trade routes get +50% yields (Portugal).
    InternationalTradeYieldBonus(i32),
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
