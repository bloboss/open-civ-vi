use crate::{
  AgeType, BarbarianCampId, BeliefId, CivId, CityId, GovernorId, GreatPersonId, GreatPersonType,
  PolicyId, PromotionId, ReligionId, TradeRouteId, UnitId,
};
use crate::world::disaster::DisasterKind;
use crate::civ::DiplomaticStatus;
use crate::civ::diplomacy::AllianceType;
use crate::civ::city::WallLevel;
use crate::civ::district::BuiltinDistrict;
use crate::civ::era::EraAge;
use crate::world::improvement::BuiltinImprovement;
use crate::world::resource::BuiltinResource;
use crate::world::road::BuiltinRoad;
use libhexgrid::coord::HexCoord;

/// Distinguishes how a combat event was initiated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttackType {
    Melee,
    Ranged,
    /// A city with walls fires a ranged attack at a nearby enemy unit.
    CityBombard,
}

/// A single atomic change to the game state.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
pub enum StateDelta {
    TurnAdvanced    { from: u32, to: u32 },
    UnitMoved       { unit: UnitId, from: HexCoord, to: HexCoord, cost: u32 },
    UnitCreated     { unit: UnitId, coord: HexCoord, owner: CivId },
    UnitDestroyed   { unit: UnitId },
    CityFounded     { city: CityId, coord: HexCoord, owner: CivId },
    CityCaptured    { city: CityId, new_owner: CivId, old_owner: CivId },
    PopulationGrew  { city: CityId, new_population: u32 },
    GoldChanged     { civ: CivId, delta: i32 },
    TechResearched  {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        tech: &'static str,
    },
    CivicCompleted  {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        civic: &'static str,
    },
    DiplomacyChanged { civ_a: CivId, civ_b: CivId, new_status: DiplomaticStatus },
    // ── OneShotEffect outcomes ──────────────────────────────────────────────
    ResourceRevealed     { civ: CivId, resource: BuiltinResource },
    EurekaTriggered      {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        tech: &'static str,
    },
    InspirationTriggered {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        civic: &'static str,
    },
    UnitUnlocked         {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        unit_type: &'static str,
    },
    BuildingUnlocked     {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        building: &'static str,
    },
    ImprovementUnlocked  {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        improvement: &'static str,
    },
    GovernmentUnlocked   {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        government: &'static str,
    },
    GovernmentAdopted    {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        government: &'static str,
    },
    PolicyUnlocked       {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        policy: &'static str,
    },
    /// A policy was removed from the slot because the new government has fewer slots.
    PolicyUnslotted      { civ: CivId, policy: PolicyId },
    PolicyAssigned       { civ: CivId, policy: PolicyId },
    /// Emitted when a free unit grant is processed. Full unit creation
    /// requires a unit-type registry (Phase 4).
    FreeUnitGranted      {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        unit_type: &'static str,
        coord: HexCoord,
    },
    /// Emitted when a free building grant is processed. Full building creation
    /// requires a building registry (Phase 4).
    FreeBuildingGranted  {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        building: &'static str,
        city: CityId,
    },
    // ── Production queue outcomes (PHASE3-4.3) ──────────────────────────────
    /// A building has been completed and added to the city.
    BuildingCompleted    {
        city: CityId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        building: &'static str,
    },
    /// A district has been placed on the map.
    DistrictBuilt        { city: CityId, district: BuiltinDistrict, coord: HexCoord },
    /// A wonder has been completed globally.
    WonderBuilt          {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        wonder: &'static str,
        city: CityId,
    },
    /// A new production item has moved to the front of the queue.
    ProductionStarted    {
        city: CityId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        item: &'static str,
    },
    // ── Citizen assignment (PHASE3-4.1) ──────────────────────────────────────
    /// A citizen has been assigned (or auto-assigned) to work a tile.
    CitizenAssigned      { city: CityId, tile: HexCoord },

    // ── Combat outcome (PHASE3-8.1) ───────────────────────────────────────────
    UnitAttacked {
        attacker:        UnitId,
        defender:        UnitId,
        attack_type:     AttackType,
        attacker_damage: u32,
        defender_damage: u32,
    },

    // ── Combat XP and promotions ──────────────────────────────────────────────
    /// A unit gained combat experience.
    ExperienceGained { unit: UnitId, amount: u32, new_total: u32 },
    /// A unit was promoted (gained a promotion ability and healed).
    UnitPromoted {
        unit: UnitId,
        promotion: PromotionId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        promotion_name: &'static str,
    },

    // ── Fog of war (PHASE3-10.2) ─────────────────────────────────────────────
    /// Tiles newly added to `explored_tiles` this move (not previously explored).
    TilesRevealed { civ: CivId, coords: Vec<HexCoord> },

    /// A civilization explored a tile containing a natural wonder for the first time.
    NaturalWonderDiscovered {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        wonder_name: &'static str,
        coord: HexCoord,
    },

    /// An improvement was placed on a tile.
    ImprovementPlaced { coord: HexCoord, improvement: BuiltinImprovement },

    /// A road was placed on a tile.
    RoadPlaced { coord: HexCoord, road: BuiltinRoad },

    /// A builder unit's remaining charges changed (after placing an improvement or road).
    ChargesChanged { unit: UnitId, remaining: u8 },

    /// A civilization's strategic resource stockpile changed (positive = gained, negative = consumed).
    StrategicResourceChanged { civ: CivId, resource: BuiltinResource, delta: i32 },

    /// A tile has been claimed by a civilization (city founding or cultural expansion).
    TileClaimed { civ: CivId, city: CityId, coord: HexCoord },

    /// A tile has been moved from one city to another within the same civilization.
    TileReassigned { civ: CivId, from_city: CityId, to_city: CityId, coord: HexCoord },

    // ── Trade routes (PHASE3-8.4) ─────────────────────────────────────────────
    /// A trader unit was assigned a trade route destination and will move
    /// autonomously toward the destination city each turn.
    TradeRouteAssigned {
        unit:        UnitId,
        origin:      CityId,
        destination: CityId,
    },
    /// A trade route was established by a trader unit (which is consumed).
    TradeRouteEstablished {
        route:       TradeRouteId,
        origin:      CityId,
        destination: CityId,
        owner:       CivId,
    },
    /// A trade route has expired (turns_remaining reached 0) or was cancelled.
    TradeRouteExpired { route: TradeRouteId },

    // ── City defense (PHASE3-8.3) ──────────────────────────────────────────────
    /// City walls took damage from a melee attack.
    WallDamaged { city: CityId, damage: u32, hp_remaining: u32 },
    /// City walls were destroyed (HP reached 0); walls breached.
    WallDestroyed { city: CityId, previous_level: WallLevel },

    // ── Tourism (PHASE3-8.6) ──────────────────────────────────────────────────
    /// Emitted each turn when a civ generates tourism. Records total tourism
    /// output and how much lifetime culture was accumulated this turn.
    TourismGenerated { civ: CivId, tourism: u32, lifetime_culture: u32 },
    // ── Loyalty system (PHASE3-8.6) ──────────────────────────────────────────
    /// A city's loyalty score changed during the turn. `delta` is the net
    /// change (positive = towards owner, negative = away); `new_value` is
    /// the clamped result in 0–100.
    LoyaltyChanged { city: CityId, delta: i32, new_value: i32 },
    /// A city revolted due to loyalty reaching 0. If `new_owner` is `Some`,
    /// the city flipped to the civilization exerting the highest loyalty
    /// pressure. If `None`, the city became a Free City (independent).
    CityRevolted { city: CityId, new_owner: Option<CivId>, old_owner: CivId },

    // ── Era score (PHASE3-8.8) ─────────────────────────────────────────────
    /// A civilization earned a historic moment, gaining era score.
    HistoricMomentEarned {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        moment: &'static str,
        era_score: u32,
    },
    /// A civilization transitioned to a new era with a determined age.
    EraAdvanced { civ: CivId, new_era: AgeType, era_age: EraAge },
    // ── Great persons (PHASE3-8.6) ─────────────────────────────────────────
    /// A great person was retired (consumed) by its owner.
    GreatPersonRetired { great_person: GreatPersonId, owner: CivId },
    /// A one-time production burst was applied to a city (e.g. Great Engineer).
    ProductionBurst { city: CityId, amount: u32 },
    /// Great person points were accumulated for a civ during advance_turn.
    GreatPersonPointsAccumulated { civ: CivId, person_type: GreatPersonType, points: u32, total: u32 },
    /// A great person was automatically recruited when points reached the threshold.
    GreatPersonRecruited { great_person: GreatPersonId, civ: CivId, person_type: GreatPersonType },
    /// A great person was patronized (sponsored) by spending gold.
    GreatPersonPatronized { great_person: GreatPersonId, civ: CivId, gold_spent: u32 },
    /// A great person was patronized (sponsored) by spending faith.
    GreatPersonPatronizedWithFaith { great_person: GreatPersonId, civ: CivId, faith_spent: u32 },

    // ── Diff consolidation (passive healing / trade route cleanup) ─────────
    /// A unit passively healed (unique unit ability, fortification, etc.).
    UnitHealed { unit: UnitId, old_health: u32, new_health: u32 },
    /// A trader's route assignment was cleared after establishment failed.
    TradeRouteCleared { unit: UnitId },

    // ── Great works / tourism (cultural victory) ──────────────────────────────
    /// A great person created a great work and it was slotted into a city.
    GreatWorkCreated {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        work_name: &'static str,
        city: CityId,
    },

    // ── Governors (PHASE3-8.7) ──────────────────────────────────────────────
    /// A governor was assigned (or reassigned) to a city.
    GovernorAssigned  { governor: GovernorId, city: CityId, owner: CivId },
    /// A governor finished its establishment countdown and is now active.
    GovernorEstablished { governor: GovernorId, city: CityId },
    /// A governor promotion was unlocked.
    GovernorPromoted  {
        governor: GovernorId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        promotion: &'static str,
    },
    /// A civilization earned a governor title (from completing a civic, etc.).
    GovernorTitleEarned { civ: CivId },

    // ── Victory condition (PHASE3-8.9) ────────────────────────────────────────
    /// Emitted when a civ wins the game. After this delta `GameState::game_over` is set.
    VictoryAchieved {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        condition: &'static str,
    },

    // ── Religion ─────────────────────────────────────────────────────────────
    /// A civilization founded a new religion.
    ReligionFounded { civ: CivId, religion: ReligionId, name: String },
    /// A civilization selected a pantheon belief.
    PantheonFounded { civ: CivId, belief: BeliefId },
    /// A belief was added to a religion (founder, follower, worship, or enhancer).
    BeliefSelected { civ: CivId, religion: ReligionId, belief: BeliefId },
    /// A religious unit spread religion to a city.
    ReligionSpread { city: CityId, religion: ReligionId, followers_added: u32 },
    /// Passive religious pressure changed followers in a city.
    ReligiousPressureApplied { city: CityId, religion: ReligionId, delta: i32 },
    /// A city's majority religion changed.
    CityConvertedReligion { city: CityId, old_religion: Option<ReligionId>, new_religion: ReligionId },
    /// Theological combat between two religious units.
    TheologicalCombat { attacker: UnitId, defender: UnitId, attacker_damage: u32, defender_damage: u32 },
    /// A civilization's faith stockpile changed.
    FaithChanged { civ: CivId, delta: i32 },
    /// An Apostle evangelized a new belief onto a religion.
    BeliefEvangelized { civ: CivId, religion: ReligionId, belief: BeliefId },
    /// An Apostle launched an inquisition for a civilization.
    InquisitionLaunched { civ: CivId },
    /// An Inquisitor removed foreign religion followers from a city.
    HeresyRemoved { city: CityId, followers_removed: u32 },
    /// A Guru healed nearby religious units.
    ReligiousUnitsHealed { healer: UnitId, healed_count: u32 },

    // ── Barbarian system ────────────────────────────────────────────────────
    // ── Science victory ─────────────────────────────────────────────────────
    /// A civilization completed a science milestone toward Science Victory.
    ScienceMilestoneCompleted {
        civ: CivId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        milestone: &'static str,
    },

    // ── Diplomatic victory ──────────────────────────────────────────────────
    /// A civilization's diplomatic favor changed.
    DiplomaticFavorChanged { civ: CivId, delta: i32 },

    // ── Power & CO2 (GS-1) ──────────────────────────────────────────────
    /// Global CO2 accumulated this turn from fossil fuel power plants.
    CO2Accumulated { total: u32 },

    /// A city project was completed.
    ProjectCompleted {
        city: CityId,
        #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
        project: &'static str,
    },

    /// A barbarian camp was spawned on the map.
    BarbarianCampSpawned { camp: BarbarianCampId, coord: HexCoord },
    /// A barbarian camp was destroyed (cleared by a player unit).
    BarbarianCampDestroyed { camp: BarbarianCampId, coord: HexCoord, cleared_by: CivId },
    /// A barbarian scout was spawned from a camp.
    BarbarianScoutSpawned { camp: BarbarianCampId, scout: UnitId, coord: HexCoord },
    /// A barbarian scout discovered a player civilization.
    BarbarianScoutDiscovered { camp: BarbarianCampId, scout: UnitId, discovered_civ: CivId },
    /// A barbarian scout returned to its camp, activating unit generation.
    BarbarianScoutReturned { camp: BarbarianCampId },
    /// A barbarian combat unit was generated by a camp.
    BarbarianUnitGenerated { camp: BarbarianCampId, unit: UnitId, coord: HexCoord },
    /// A player hired a unit from a barbarian clan (Clans mode).
    BarbarianClanHired { camp: BarbarianCampId, civ: CivId, unit: UnitId, gold_spent: u32 },
    /// A player bribed a barbarian clan (Clans mode).
    BarbarianClanBribed { camp: BarbarianCampId, civ: CivId, gold_spent: u32 },
    /// A player incited a barbarian clan against another player (Clans mode).
    BarbarianClanIncited { camp: BarbarianCampId, civ: CivId, target: CivId, gold_spent: u32 },
    /// A barbarian camp converted into a city-state (Clans mode).
    BarbarianCampConverted { camp: BarbarianCampId, city: CityId, coord: HexCoord },

    // ── World Congress (GS-3) ────────────────────────────────────────────
    /// A World Congress session was held; the civ with the most diplomatic
    /// favor won the session.
    CongressSessionHeld { winner: CivId },
    /// A civilization earned diplomatic victory points from a Congress session.
    DiplomaticVPEarned { civ: CivId, points: u32 },

    // ── Rock Band / Cultural Combat (GS-16) ────────────────────────────
    /// A Rock Band performed at a foreign city, generating tourism.
    RockBandPerformed { unit: UnitId, city: CityId, tourism_gained: u32 },

    // ── Climate & Disasters (GS-2) ─────────────────────────────────────
    /// Global sea level rose to a new stage.
    SeaLevelRose { new_level: u8 },
    /// A coastal lowland tile was submerged by rising sea levels.
    TileSubmerged { coord: HexCoord },
    /// An environmental disaster occurred on a tile.
    DisasterOccurred { kind: DisasterKind, coord: HexCoord, severity: u8 },

    // ── Alliances (Rise & Fall) ────────────────────────────────────────────
    /// Two civilizations formed an alliance of a specific type.
    AllianceFormed { civ_a: CivId, civ_b: CivId, alliance_type: AllianceType },
    /// An alliance between two civilizations leveled up.
    AllianceLevelUp { civ_a: CivId, civ_b: CivId, new_level: u8 },

    // ── Embarkation ──────────────────────────────────────────────────────────
    /// A civilization unlocked coast embarkation (via Shipbuilding tech).
    EmbarkCoastUnlocked { civ: CivId },
    /// A civilization unlocked ocean embarkation (via Cartography tech).
    EmbarkOceanUnlocked { civ: CivId },
    /// A land unit embarked onto a water tile.
    UnitEmbarked { unit: UnitId, coord: HexCoord },
    /// An embarked unit disembarked onto a land tile.
    UnitDisembarked { unit: UnitId, coord: HexCoord },
}

/// A batch of deltas representing a complete state transition.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "'de: 'static")))]
pub struct GameStateDiff {
    pub deltas: Vec<StateDelta>,
}

impl GameStateDiff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, delta: StateDelta) {
        self.deltas.push(delta);
    }

    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    pub fn len(&self) -> usize {
        self.deltas.len()
    }
}
