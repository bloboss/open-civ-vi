use crate::{CityId, CivId, GovernorId, GreatPersonId, GreatPersonType, PolicyId, UnitId, YieldBundle};
use crate::civ::civ_ability::{CivAbilityBundle, RuleOverride};
use crate::civ::civ_identity::BuiltinCiv;
use crate::rules::civ_registry;
use libhexgrid::coord::HexCoord;

use super::diff::GameStateDiff;
use super::state::GameState;

pub(crate) mod city;
pub(crate) mod combat;
pub(crate) mod diplomacy;
pub(crate) mod effects;
pub(crate) mod governors;
pub(crate) mod great_people;
pub(crate) mod movement;
pub(crate) mod production;
pub(crate) mod religion;
pub(crate) mod trade;
pub(crate) mod turn_phase;

/// Look up a civ's ability bundle by identity. Returns None for custom civs.
pub(crate) fn lookup_bundle(civ_identity: Option<BuiltinCiv>) -> Option<CivAbilityBundle> {
    let civ = civ_identity?;
    Some(match civ {
        BuiltinCiv::Rome    => civ_registry::rome(),
        BuiltinCiv::Greece  => civ_registry::greece(),
        BuiltinCiv::Egypt   => civ_registry::egypt(),
        BuiltinCiv::Babylon => civ_registry::babylon(),
        BuiltinCiv::Germany => civ_registry::germany(),
        BuiltinCiv::Japan   => civ_registry::japan(),
        BuiltinCiv::India   => civ_registry::india(),
        BuiltinCiv::Arabia  => civ_registry::arabia(),
    })
}

/// Check if a civ has a specific rule override.
pub(crate) fn has_rule_override(state: &GameState, civ_id: CivId, check: &dyn Fn(&RuleOverride) -> bool) -> bool {
    state.civilizations.iter()
        .find(|c| c.id == civ_id)
        .and_then(|c| lookup_bundle(c.civ_identity))
        .is_some_and(|b| b.rule_overrides.iter().any(check))
}

/// Core rules evaluation interface.
pub trait RulesEngine: std::fmt::Debug {
    /// Validate and apply a unit move. Returns the resulting diff, or
    /// `Err(InsufficientMovement(partial_diff))` when the unit cannot reach
    /// the destination within its remaining movement budget.
    fn move_unit(
        &self,
        state: &GameState,
        unit: UnitId,
        to: HexCoord,
    ) -> Result<GameStateDiff, RulesError>;

    /// Compute all yields for a civilization this turn (tile yields + building
    /// yields + resolved modifier effects). Only tiles in each city's
    /// `worked_tiles` are counted; resource yields are suppressed when the civ
    /// lacks the required reveal tech.
    fn compute_yields(&self, state: &GameState, civ: CivId) -> YieldBundle;

    /// Advance the game state by one turn. Returns diff.
    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff;

    /// Assign a citizen to work `tile` in `city`. When `lock` is true the tile
    /// is added to `city.locked_tiles` so auto-reassignment on future growth
    /// will not displace it.
    fn assign_citizen(
        &self,
        state: &mut GameState,
        city: CityId,
        tile: HexCoord,
        lock: bool,
    ) -> Result<GameStateDiff, RulesError>;

    /// Validate and assign a policy to the civilization's active slots.
    /// Validates: policy is unlocked; current government has a free slot of the
    /// required type; maintenance cost does not exceed treasury.
    fn assign_policy(
        &self,
        state: &mut GameState,
        civ: CivId,
        policy: PolicyId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Declare war between `aggressor` and `target`. Sets status to War, records a
    /// `DeclaredWarGrievance` for the target, and emits `DiplomacyChanged`.
    /// Returns `AlreadyAtWar` if they are already at war, `SameCivilization` if both
    /// IDs are equal, or `CivNotFound` if either civ does not exist.
    fn declare_war(
        &self,
        state: &mut GameState,
        aggressor: CivId,
        target: CivId,
    ) -> Result<GameStateDiff, RulesError>;

    /// End the war between `civ_a` and `civ_b`. Resets `turns_at_war`, recomputes
    /// status from the current opinion score, and emits `DiplomacyChanged`.
    /// Returns `NotAtWar` if they are not at war, `SameCivilization` if both IDs
    /// are equal, or `RelationNotFound` if no relation exists.
    fn make_peace(
        &self,
        state: &mut GameState,
        civ_a: CivId,
        civ_b: CivId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Resolve combat between `attacker` and `defender`.
    ///
    /// Melee (`attacker.range == 0`): both units take damage using the formula
    /// `30 * exp((cs_atk - cs_def) / 25) * rng[0.75, 1.25]`. Attacker takes
    /// the symmetric version. Ranged (`range > 0`): only defender takes damage.
    /// When a unit's health reaches 0 it is destroyed and removed from state.
    /// Attacker loses all remaining movement.
    fn attack(
        &self,
        state:    &mut GameState,
        attacker: UnitId,
        defender: UnitId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Consume a settler unit and found a new city at its current position.
    ///
    /// Validation: settler must have `UnitTypeDef.can_found_city == true`; tile
    /// must be land (not ocean / mountain); no existing city within 3 tiles.
    /// On success: removes the settler, creates the city, claims ring-1 tiles
    /// for the civ (if unowned), and auto-assigns the first citizen.
    fn found_city(
        &self,
        state:   &mut GameState,
        settler: UnitId,
        name:    String,
    ) -> Result<GameStateDiff, RulesError>;

    /// Place an improvement on `coord`. Validates `valid_on()` for the tile's
    /// terrain/feature combination. Returns `InvalidImprovement` when the
    /// placement is illegal (water tile, wrong terrain, etc.).
    ///
    /// When `builder` is `Some(unit_id)`, the builder's charges are decremented
    /// after successful placement. If charges reach 0, the builder is destroyed.
    fn place_improvement(
        &self,
        state: &mut GameState,
        civ_id: CivId,
        coord: HexCoord,
        improvement: crate::world::improvement::BuiltinImprovement,
        builder: Option<UnitId>,
    ) -> Result<GameStateDiff, RulesError>;

    /// Place a road on `coord` using a builder unit.
    ///
    /// Validation: builder must be at `coord` with charges remaining; tile must
    /// be land and owned by the builder's civ; road tier cannot be a downgrade;
    /// tech requirements must be met (Ancient = none, Medieval = Engineering,
    /// Industrial = Steam Power, Railroad = Railroads).
    fn place_road(
        &self,
        state: &mut GameState,
        unit_id: UnitId,
        coord: HexCoord,
        road: crate::world::road::BuiltinRoad,
    ) -> Result<GameStateDiff, RulesError>;

    /// Place a district for `city_id` at `coord`.
    ///
    /// Validation: coord must be within 1–3 tiles of the city center; the tile
    /// must be owned by the city's civilization; no existing district on the tile;
    /// coord must not be a city center; the city must not already have this
    /// district type; terrain and water constraints from `DistrictRequirements`
    /// must be satisfied; required tech/civic must be researched/completed.
    fn place_district(
        &self,
        state: &mut GameState,
        city_id: CityId,
        district: crate::civ::district::BuiltinDistrict,
        coord: HexCoord,
    ) -> Result<GameStateDiff, RulesError>;

    /// Claim `coord` for the civilization that owns `city_id`.
    ///
    /// Validation:
    /// - City must exist.
    /// - `coord` must be within 1–3 tiles of the city center.
    /// - If the tile is already owned by the same civ, returns an empty diff (idempotent).
    /// - If the tile is owned by a different civ and `force` is `false`, returns
    ///   `TileOwnedByEnemy`. If `force` is `true` (culture flip), the tile is taken.
    fn claim_tile(
        &self,
        state: &mut GameState,
        city_id: CityId,
        coord: HexCoord,
        force: bool,
    ) -> Result<GameStateDiff, RulesError>;

    /// Reassign `coord` from one city to another within the same civilization.
    ///
    /// Validation:
    /// - Both cities must exist.
    /// - Both cities must belong to the same civilization (`CitiesNotSameCiv`).
    /// - The tile must be owned by that civilization (`TileNotOwned`).
    /// - `coord` must be within 1–3 tiles of `to_city`'s center (`TileNotInCityRange`).
    /// - If `from_city == to_city`, returns an empty diff (idempotent).
    fn reassign_tile(
        &self,
        state: &mut GameState,
        from_city: CityId,
        to_city: CityId,
        coord: HexCoord,
    ) -> Result<GameStateDiff, RulesError>;

    /// Assign a trade route destination to a trader unit.
    ///
    /// The trader must be located at a city tile owned by its civilization (origin city).
    /// Once assigned, the trader will move autonomously toward the destination city
    /// each turn during `advance_turn`. When it arrives, the trade route is
    /// automatically established and the trader is consumed.
    fn assign_trade_route(
        &self,
        state: &mut GameState,
        trader_unit: UnitId,
        destination: CityId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Consume a trader unit and establish a trade route to `destination`.
    ///
    /// The trader must be located at a city tile owned by its civilization (origin city)
    /// **or** have a pre-assigned `trade_origin` (set by `assign_trade_route`).
    /// The route lasts 30 turns; yields are delivered each turn via `compute_yields`.
    fn establish_trade_route(
        &self,
        state: &mut GameState,
        trader_unit: UnitId,
        destination: CityId,
    ) -> Result<GameStateDiff, RulesError>;

    /// City with walls fires a ranged bombardment at an enemy unit within range 2.
    ///
    /// Requires `WallLevel != None`. Each city may fire once per turn
    /// (`has_attacked_this_turn`). City bombardments deal damage using the
    /// standard exponential formula; no counter-damage is taken. City ranged
    /// strength = 15 + wall_defense_bonus.
    fn city_bombard(
        &self,
        state: &mut GameState,
        city_id: CityId,
        target: UnitId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Retire (consume) a great person, applying its one-time ability.
    ///
    /// The great person must exist, not already be retired, and be owned by a civ.
    /// On success the great person is marked retired, its corresponding unit is
    /// removed, and the retire effect is applied (combat modifier, production
    /// burst, or gold grant).
    fn retire_great_person(
        &self,
        state: &mut GameState,
        great_person_id: crate::GreatPersonId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Create a great work from a great person (Writer/Artist/Musician).
    /// The great person must be owned by a civ, not retired, and of the right type.
    /// The work is auto-slotted in the first available matching slot in an owned city.
    fn create_great_work(
        &self,
        state: &mut GameState,
        great_person_id: GreatPersonId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Patronize (sponsor) a great person by spending gold. Claims the next
    /// available candidate of `person_type` for `civ_id`. Gold cost is
    /// `(threshold - current_points) * GP_PATRONAGE_GOLD_PER_POINT`.
    /// The great person spawns at the civ's capital.
    fn recruit_great_person(
        &self,
        state: &mut GameState,
        civ_id: CivId,
        person_type: GreatPersonType,
    ) -> Result<GameStateDiff, RulesError>;

    /// Patronize (sponsor) a Great Prophet by spending faith. Only valid for
    /// `GreatPersonType::Prophet`. Faith cost is
    /// `(threshold - current_points) * GP_PATRONAGE_FAITH_PER_POINT`.
    fn recruit_great_person_with_faith(
        &self,
        state: &mut GameState,
        civ_id: CivId,
        person_type: GreatPersonType,
    ) -> Result<GameStateDiff, RulesError>;

    /// Assign (or reassign) a governor to a city.
    ///
    /// Validation: governor must exist and be owned by the city's owner.
    /// If the governor is already at that city, returns `GovernorAlreadyInCity`.
    /// On success, sets `turns_to_establish` to 5 and emits `GovernorAssigned`.
    fn assign_governor(
        &self,
        state: &mut GameState,
        governor_id: GovernorId,
        city_id: CityId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Unlock a promotion for a governor.
    ///
    /// Costs one governor title. Validates prerequisites are met.
    fn promote_governor(
        &self,
        state: &mut GameState,
        governor_id: GovernorId,
        promotion_name: &'static str,
    ) -> Result<GameStateDiff, RulesError>;

    /// Found a pantheon for `civ`. Requires 25 accumulated faith and no
    /// existing pantheon. The selected belief must be a Follower-category belief
    /// that has not been taken by any other civ's pantheon.
    fn found_pantheon(
        &self,
        state: &mut GameState,
        civ: CivId,
        belief: crate::BeliefId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Found a religion using a Great Prophet at a Holy Site.
    ///
    /// Validation: unit must be a Great Prophet at a tile with the civ's Holy Site;
    /// civ must not have already founded a religion; name must not be taken.
    /// On success: creates Religion, assigns initial beliefs (founder + follower),
    /// sets holy city followers, consumes the prophet.
    fn found_religion(
        &self,
        state: &mut GameState,
        prophet: UnitId,
        name: String,
        beliefs: Vec<crate::BeliefId>,
    ) -> Result<GameStateDiff, RulesError>;

    /// Spread religion from a Missionary or Apostle to the city at the unit's
    /// current location. Decrements spread charges; destroys unit when charges
    /// reach zero.
    fn spread_religion(
        &self,
        state: &mut GameState,
        unit: UnitId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Theological combat between two religious units (Apostle vs Apostle/Missionary).
    /// Both must be `UnitCategory::Religious`, attacker must have `religious_strength`.
    fn theological_combat(
        &self,
        state: &mut GameState,
        attacker: UnitId,
        defender: UnitId,
    ) -> Result<GameStateDiff, RulesError>;

    /// Purchase a unit or building with faith.
    fn purchase_with_faith(
        &self,
        state: &mut GameState,
        civ: CivId,
        city: CityId,
        item: FaithPurchaseItem,
    ) -> Result<GameStateDiff, RulesError>;

    // TODO(PHASE3-BORDERS): fn purchase_tile(&self, state: &mut GameState, city_id: CityId,
    //   coord: HexCoord) -> Result<GameStateDiff, RulesError>;
    //   Spends gold (or culture) from the civilization's treasury to immediately claim a tile
    //   within radius 3 of the city. Cost scales with tile distance. Distinct from automatic
    //   cultural expansion: this is a player action and debits the main treasury.
}
/// What can be purchased with faith.
#[derive(Debug, Clone)]
pub enum FaithPurchaseItem {
    /// Purchase a unit by its type name (e.g. "Missionary", "Apostle").
    Unit(&'static str),
    /// Purchase a worship building by its belief name (e.g. "Cathedral").
    WorshipBuilding(&'static str),
}

/// Errors returned by rules engine operations.
#[derive(Debug, Clone)]
pub enum RulesError {
    UnitNotFound,
    CityNotFound,
    CivNotFound,
    PolicyNotFound,
    /// Policy is not in the civilization's `unlocked_policies` list.
    PolicyNotUnlocked,
    /// The civilization's current government has no free slot for this policy type.
    InsufficientPolicySlots,
    /// No active government; cannot assign policies.
    NoGovernment,
    /// Not enough gold to cover the policy's maintenance cost.
    InsufficientGold,
    /// No path exists to the destination (impassable terrain or out of bounds).
    DestinationImpassable,
    /// A path exists but the unit's movement budget was exhausted before reaching
    /// the destination. The inner diff records the partial move that did occur
    /// (if any movement was possible).
    InsufficientMovement(GameStateDiff),
    InvalidCoord,
    NotYourTurn,
    /// Both civilization IDs refer to the same civilization.
    SameCivilization,
    /// The two civilizations are already at war.
    AlreadyAtWar,
    /// The two civilizations are not at war.
    NotAtWar,
    /// No diplomatic relation exists between the two civilizations.
    RelationNotFound,
    /// Target tile contains no enemy unit.
    NoValidTarget,
    /// The attacking unit has no combat strength (civilian unit).
    UnitCannotAttack,
    /// Units are not adjacent (melee) or not within attack range (ranged).
    NotInRange,
    /// The unit is not a settler-class unit (UnitTypeDef.can_found_city == false).
    NotASettler,
    /// The target tile already contains a city.
    TileOccupied,
    /// The founding site is within 3 tiles of an existing city.
    TooCloseToCity,
    /// The terrain type cannot host a city (ocean, mountain).
    InvalidFoundingTerrain,
    /// Destination tile is already occupied by another unit.
    /// Use `attack()` to engage an enemy; friendly unit stacking is not allowed.
    TileOccupiedByUnit,
    /// The improvement cannot be placed on the target tile (wrong terrain / feature).
    InvalidImprovement,
    /// Improvement requires a specific resource not present on the tile.
    ResourceRequired,
    /// Improvement requires an adjacent tile condition that is not satisfied.
    ProximityRequired,
    /// Improvement requires a tech not yet researched by the civilization.
    TechRequired,
    /// Improvement requires a civic not yet completed by the civilization.
    CivicRequired,
    /// The target tile is not owned by the acting civilization.
    TileNotOwned,
    /// The district cannot be placed on this terrain or tile type.
    InvalidDistrict,
    /// The city already contains a district of this type (max 1 per city).
    DistrictAlreadyPresent,
    /// The target coord is not within the valid range (1–3 tiles) of the city center.
    TileNotInCityRange,
    /// The target tile is already occupied by a different district.
    TileOccupiedByDistrict,
    /// The target tile is owned by a different civilization; cannot claim enemy territory.
    TileOwnedByEnemy,
    /// The two cities belong to different civilizations; tile reassignment requires same-civ cities.
    CitiesNotSameCiv,
    /// The civilization does not have enough of the required strategic resource to train the unit.
    InsufficientStrategicResource,
    /// The unit is not a trader (UnitCategory::Trader).
    NotATrader,
    /// The trader is not located on a tile owned by one of the civ's cities.
    NoOriginCity,
    /// The origin and destination cities are the same.
    SameCity,
    /// City has no walls and cannot perform a ranged bombardment.
    CityCannotAttack,
    /// City has already performed its bombardment this turn.
    CityAlreadyAttacked,
    /// The unit is not a builder (has no charges).
    NotABuilder,
    /// The builder has no charges remaining.
    NoChargesRemaining,
    /// Cannot downgrade to a lower-tier road.
    RoadDowngrade,
    /// The great person ID was not found in `state.great_people`.
    GreatPersonNotFound,
    /// The great person has already been retired.
    GreatPersonAlreadyRetired,
    /// No great person definition found matching this great person's name.
    GreatPersonDefNotFound,
    InvalidGreatPersonType,
    NoGreatWorkSlot,
    /// No great person candidate of the requested type is available in the current era.
    NoGreatPersonAvailable,
    /// The governor ID was not found in `state.governors`.
    GovernorNotFound,
    /// The governor is not owned by the acting civilization.
    GovernorNotOwned,
    /// The civilization has no unspent governor titles.
    InsufficientGovernorTitles,
    /// The specified promotion does not exist or does not belong to this governor.
    PromotionNotFound,
    /// A prerequisite promotion has not been unlocked yet.
    PromotionPrerequisiteNotMet,
    /// The governor already has this promotion.
    PromotionAlreadyUnlocked,
    /// The governor is already assigned to this city.
    GovernorAlreadyInCity,
    /// The civilization has already founded a pantheon.
    PantheonAlreadyFounded,
    /// The civilization has already founded a religion.
    ReligionAlreadyFounded,
    /// The unit is not a Great Prophet.
    NotAGreatProphet,
    /// The unit is not on a tile with a Holy Site district.
    NoHolySite,
    /// The religion name is already taken by another civilization.
    ReligionNameTaken,
    /// The selected belief is not valid (wrong category or already taken).
    InvalidBelief,
    /// The unit has no remaining spread charges.
    NoSpreadCharges,
    /// The unit is not a religious unit (Missionary/Apostle).
    NotAReligiousUnit,
    /// The unit has no religious combat strength.
    NoReligiousStrength,
    /// Not enough faith to make the purchase.
    InsufficientFaith,
    /// The city does not have the required building/district for this purchase.
    MissingPrerequisite,
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulesError::UnitNotFound              => write!(f, "unit not found"),
            RulesError::CityNotFound              => write!(f, "city not found"),
            RulesError::CivNotFound               => write!(f, "civilization not found"),
            RulesError::PolicyNotFound            => write!(f, "policy not found"),
            RulesError::PolicyNotUnlocked         => write!(f, "policy not unlocked"),
            RulesError::InsufficientPolicySlots   => write!(f, "no free policy slot in current government"),
            RulesError::NoGovernment              => write!(f, "no active government"),
            RulesError::InsufficientGold          => write!(f, "insufficient gold for policy maintenance"),
            RulesError::DestinationImpassable     => write!(f, "destination is impassable"),
            RulesError::InsufficientMovement(_)   => write!(f, "insufficient movement points"),
            RulesError::InvalidCoord              => write!(f, "invalid coordinate"),
            RulesError::NotYourTurn               => write!(f, "not your turn"),
            RulesError::SameCivilization          => write!(f, "both IDs refer to the same civilization"),
            RulesError::AlreadyAtWar              => write!(f, "civilizations are already at war"),
            RulesError::NotAtWar                  => write!(f, "civilizations are not at war"),
            RulesError::RelationNotFound          => write!(f, "no diplomatic relation between the civilizations"),
            RulesError::NoValidTarget             => write!(f, "no enemy unit at target tile"),
            RulesError::UnitCannotAttack          => write!(f, "unit has no combat strength"),
            RulesError::NotInRange                => write!(f, "target is not within attack range"),
            RulesError::NotASettler               => write!(f, "unit cannot found cities"),
            RulesError::TileOccupied              => write!(f, "a city already exists at that location"),
            RulesError::TooCloseToCity            => write!(f, "too close to an existing city"),
            RulesError::InvalidFoundingTerrain    => write!(f, "cannot found a city on this terrain"),
            RulesError::TileOccupiedByUnit        => write!(f, "destination tile is occupied by another unit"),
            RulesError::InvalidImprovement        => write!(f, "improvement cannot be placed on this terrain"),
            RulesError::ResourceRequired          => write!(f, "improvement requires a resource not present on the tile"),
            RulesError::ProximityRequired         => write!(f, "improvement requires an adjacent tile condition not satisfied"),
            RulesError::TechRequired              => write!(f, "requires a tech not yet researched"),
            RulesError::CivicRequired             => write!(f, "requires a civic not yet completed"),
            RulesError::TileNotOwned              => write!(f, "tile is not owned by the acting civilization"),
            RulesError::InvalidDistrict           => write!(f, "district cannot be placed on this terrain"),
            RulesError::DistrictAlreadyPresent    => write!(f, "city already has a district of this type"),
            RulesError::TileNotInCityRange        => write!(f, "tile is not within 1–3 tiles of the city center"),
            RulesError::TileOccupiedByDistrict    => write!(f, "tile is already occupied by a district"),
            RulesError::TileOwnedByEnemy          => write!(f, "tile is owned by a different civilization"),
            RulesError::CitiesNotSameCiv          => write!(f, "tile reassignment requires both cities to belong to the same civilization"),
            RulesError::InsufficientStrategicResource => write!(f, "insufficient strategic resource to train this unit"),
            RulesError::NotATrader                    => write!(f, "unit is not a trader"),
            RulesError::NoOriginCity                  => write!(f, "trader is not located at a city owned by this civilization"),
            RulesError::SameCity                      => write!(f, "origin and destination are the same city"),
            RulesError::CityCannotAttack              => write!(f, "city has no walls and cannot bombard"),
            RulesError::CityAlreadyAttacked           => write!(f, "city has already attacked this turn"),
            RulesError::NotABuilder                   => write!(f, "unit is not a builder"),
            RulesError::NoChargesRemaining            => write!(f, "builder has no charges remaining"),
            RulesError::RoadDowngrade                 => write!(f, "cannot downgrade to a lower-tier road"),
            RulesError::GreatPersonNotFound            => write!(f, "great person not found"),
            RulesError::GreatPersonAlreadyRetired      => write!(f, "great person already retired"),
            RulesError::GreatPersonDefNotFound         => write!(f, "great person definition not found"),
            RulesError::InvalidGreatPersonType         => write!(f, "great person type cannot create great works"),
            RulesError::NoGreatWorkSlot                => write!(f, "no matching great work slot available in any owned city"),
            RulesError::NoGreatPersonAvailable         => write!(f, "no great person candidate of the requested type is available"),
            RulesError::GovernorNotFound               => write!(f, "governor not found"),
            RulesError::GovernorNotOwned               => write!(f, "governor is not owned by the acting civilization"),
            RulesError::InsufficientGovernorTitles     => write!(f, "no unspent governor titles"),
            RulesError::PromotionNotFound              => write!(f, "promotion not found for this governor"),
            RulesError::PromotionPrerequisiteNotMet    => write!(f, "prerequisite promotion not yet unlocked"),
            RulesError::PromotionAlreadyUnlocked       => write!(f, "promotion already unlocked"),
            RulesError::GovernorAlreadyInCity           => write!(f, "governor is already assigned to this city"),
            RulesError::PantheonAlreadyFounded         => write!(f, "pantheon already founded"),
            RulesError::ReligionAlreadyFounded         => write!(f, "religion already founded"),
            RulesError::NotAGreatProphet               => write!(f, "unit is not a Great Prophet"),
            RulesError::NoHolySite                     => write!(f, "no Holy Site district at unit location"),
            RulesError::ReligionNameTaken              => write!(f, "religion name already taken"),
            RulesError::InvalidBelief                  => write!(f, "invalid belief selection"),
            RulesError::NoSpreadCharges                => write!(f, "no spread charges remaining"),
            RulesError::NotAReligiousUnit              => write!(f, "unit is not a religious unit"),
            RulesError::NoReligiousStrength            => write!(f, "unit has no religious combat strength"),
            RulesError::InsufficientFaith              => write!(f, "insufficient faith"),
            RulesError::MissingPrerequisite            => write!(f, "missing prerequisite building or district"),
        }
    }
}

impl std::error::Error for RulesError {}


// ── DefaultRulesEngine ────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DefaultRulesEngine;

impl RulesEngine for DefaultRulesEngine {
    fn move_unit(&self, state: &GameState, unit_id: UnitId, to: HexCoord) -> Result<GameStateDiff, RulesError> {
        movement::move_unit(state, unit_id, to)
    }

    fn compute_yields(&self, state: &GameState, civ: CivId) -> YieldBundle {
        city::compute_yields(state, civ)
    }

    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff {
        turn_phase::advance_turn(self, state)
    }

    fn assign_citizen(&self, state: &mut GameState, city_id: CityId, tile: HexCoord, lock: bool) -> Result<GameStateDiff, RulesError> {
        city::assign_citizen(state, city_id, tile, lock)
    }

    fn assign_policy(&self, state: &mut GameState, civ: CivId, policy: PolicyId) -> Result<GameStateDiff, RulesError> {
        diplomacy::assign_policy(state, civ, policy)
    }

    fn declare_war(&self, state: &mut GameState, aggressor: CivId, target: CivId) -> Result<GameStateDiff, RulesError> {
        diplomacy::declare_war(state, aggressor, target)
    }

    fn make_peace(&self, state: &mut GameState, civ_a: CivId, civ_b: CivId) -> Result<GameStateDiff, RulesError> {
        diplomacy::make_peace(state, civ_a, civ_b)
    }

    fn attack(&self, state: &mut GameState, attacker: UnitId, defender: UnitId) -> Result<GameStateDiff, RulesError> {
        combat::attack(state, attacker, defender)
    }

    fn found_city(&self, state: &mut GameState, settler: UnitId, name: String) -> Result<GameStateDiff, RulesError> {
        city::found_city(state, settler, name)
    }

    fn place_improvement(&self, state: &mut GameState, civ_id: CivId, coord: HexCoord, improvement: crate::world::improvement::BuiltinImprovement, builder: Option<UnitId>) -> Result<GameStateDiff, RulesError> {
        production::place_improvement(state, civ_id, coord, improvement, builder)
    }

    fn place_road(&self, state: &mut GameState, unit_id: UnitId, coord: HexCoord, road: crate::world::road::BuiltinRoad) -> Result<GameStateDiff, RulesError> {
        production::place_road(state, unit_id, coord, road)
    }

    fn place_district(&self, state: &mut GameState, city_id: CityId, district: crate::civ::district::BuiltinDistrict, coord: HexCoord) -> Result<GameStateDiff, RulesError> {
        production::place_district(state, city_id, district, coord)
    }

    fn claim_tile(&self, state: &mut GameState, city_id: CityId, coord: HexCoord, force: bool) -> Result<GameStateDiff, RulesError> {
        city::claim_tile(state, city_id, coord, force)
    }

    fn reassign_tile(&self, state: &mut GameState, from_city: CityId, to_city: CityId, coord: HexCoord) -> Result<GameStateDiff, RulesError> {
        city::reassign_tile(state, from_city, to_city, coord)
    }

    fn assign_trade_route(&self, state: &mut GameState, trader_unit: UnitId, destination: CityId) -> Result<GameStateDiff, RulesError> {
        trade::assign_trade_route(state, trader_unit, destination)
    }

    fn establish_trade_route(&self, state: &mut GameState, trader_unit: UnitId, destination: CityId) -> Result<GameStateDiff, RulesError> {
        trade::establish_trade_route(state, trader_unit, destination)
    }

    fn city_bombard(&self, state: &mut GameState, city_id: CityId, target: UnitId) -> Result<GameStateDiff, RulesError> {
        combat::city_bombard(state, city_id, target)
    }

    fn retire_great_person(&self, state: &mut GameState, great_person_id: GreatPersonId) -> Result<GameStateDiff, RulesError> {
        great_people::retire_great_person(state, great_person_id)
    }

    fn create_great_work(&self, state: &mut GameState, great_person_id: GreatPersonId) -> Result<GameStateDiff, RulesError> {
        great_people::create_great_work(state, great_person_id)
    }

    fn recruit_great_person(&self, state: &mut GameState, civ_id: CivId, person_type: GreatPersonType) -> Result<GameStateDiff, RulesError> {
        great_people::recruit_great_person(state, civ_id, person_type)
    }

    fn recruit_great_person_with_faith(&self, state: &mut GameState, civ_id: CivId, person_type: GreatPersonType) -> Result<GameStateDiff, RulesError> {
        great_people::recruit_great_person_with_faith(state, civ_id, person_type)
    }

    fn assign_governor(&self, state: &mut GameState, governor_id: GovernorId, city_id: CityId) -> Result<GameStateDiff, RulesError> {
        governors::assign_governor(state, governor_id, city_id)
    }

    fn promote_governor(&self, state: &mut GameState, governor_id: GovernorId, promotion_name: &'static str) -> Result<GameStateDiff, RulesError> {
        governors::promote_governor(state, governor_id, promotion_name)
    }

    fn found_pantheon(&self, state: &mut GameState, civ: CivId, belief: crate::BeliefId) -> Result<GameStateDiff, RulesError> {
        religion::found_pantheon(state, civ, belief)
    }

    fn found_religion(&self, state: &mut GameState, prophet: UnitId, name: String, beliefs: Vec<crate::BeliefId>) -> Result<GameStateDiff, RulesError> {
        religion::found_religion(state, prophet, name, beliefs)
    }

    fn spread_religion(&self, state: &mut GameState, unit: UnitId) -> Result<GameStateDiff, RulesError> {
        religion::spread_religion(state, unit)
    }

    fn theological_combat(&self, state: &mut GameState, attacker: UnitId, defender: UnitId) -> Result<GameStateDiff, RulesError> {
        combat::theological_combat(state, attacker, defender)
    }

    fn purchase_with_faith(&self, state: &mut GameState, civ: CivId, city: CityId, item: FaithPurchaseItem) -> Result<GameStateDiff, RulesError> {
        religion::purchase_with_faith(state, civ, city, item)
    }
}



// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::{BasicUnit, Civilization, City, DiplomaticRelation, DiplomaticStatus, GrievanceRecord, Leader};
    use crate::civ::civilization::Agenda;
    use crate::{CivId, PolicyType, UnitCategory, UnitDomain, UnitTypeId};
    use crate::rules::effect::OneShotEffect;
    use super::effects::apply_effect;
    use super::super::diff::StateDelta;
    use libhexgrid::board::HexBoard;
    use libhexgrid::coord::HexCoord;

    // ── Shared test helpers ───────────────────────────────────────────────────

    struct NoOpAgenda;
    impl std::fmt::Debug for NoOpAgenda {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "NoOpAgenda")
        }
    }
    impl Agenda for NoOpAgenda {
        fn name(&self) -> &'static str { "No-op" }
        fn description(&self) -> &'static str { "" }
        fn attitude(&self, _: CivId) -> i32 { 0 }
    }

    fn test_leader(civ_id: CivId) -> Leader {
        Leader {
            name: "TestLeader",
            civ_id,
            abilities: Vec::new(),
            agenda: Box::new(NoOpAgenda),
        }
    }

    fn make_state() -> (GameState, CivId) {
        let mut state = GameState::new(42, 10, 10);
        let civ_id = state.id_gen.next_civ_id();
        let civ = Civilization::new(civ_id, "TestCiv", "Test", test_leader(civ_id));
        state.civilizations.push(civ);
        (state, civ_id)
    }

    fn spawn_unit(state: &mut GameState, civ_id: CivId, coord: HexCoord, movement: u32) -> crate::UnitId {
        let unit_id   = state.id_gen.next_unit_id();
        let unit_type = state.id_gen.next_ulid();
        let unit_type_id = crate::UnitTypeId::from_ulid(unit_type);
        state.units.push(BasicUnit {
            id:             unit_id,
            unit_type:      unit_type_id,
            owner:          civ_id,
            coord,
            domain:         UnitDomain::Land,
            category:       UnitCategory::Combat,
            movement_left:  movement,
            max_movement:   movement,
            combat_strength: Some(20),
            promotions:     Vec::new(),
            health:         100,
            range:          0,
            vision_range:   2,
            charges:        None,
            trade_origin:   None,
            trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
        });
        unit_id
    }

    /// Add the 6 neighbors of `coord` to `city.worked_tiles` so the city
    /// starts with the standard 7-tile founding area (center + ring-1).
    fn add_founding_tiles(city: &mut City) {
        let center = city.coord;
        for n in center.neighbors() {
            city.worked_tiles.push(n);
        }
    }

    // ── move_unit tests ───────────────────────────────────────────────────────

    #[test]
    fn test_move_unit_full_move() {
        let (mut state, civ_id) = make_state();
        let start = HexCoord::from_qr(2, 2);
        let dest  = HexCoord::from_qr(4, 2);
        // Hex distance = 2. Each Grassland tile costs 100. Budget = 300 (ample).
        let uid = spawn_unit(&mut state, civ_id, start, 300);

        let engine = DefaultRulesEngine;
        let result = engine.move_unit(&state, uid, dest);

        assert!(result.is_ok(), "full move should succeed: {:?}", result.err());
        let diff = result.unwrap();
        assert_eq!(diff.len(), 1);
        match &diff.deltas[0] {
            StateDelta::UnitMoved { unit, from, to, .. } => {
                assert_eq!(*unit, uid);
                assert_eq!(*from, start);
                assert_eq!(*to, dest);
            }
            other => panic!("unexpected delta: {:?}", other),
        }
    }

    #[test]
    fn test_move_unit_impassable_destination() {
        use crate::world::terrain::BuiltinTerrain;

        let (mut state, civ_id) = make_state();
        let start = HexCoord::from_qr(2, 2);
        let mountain = HexCoord::from_qr(3, 2);

        // Block the only direct neighbor in the E direction.
        if let Some(t) = state.board.tile_mut(mountain) {
            t.terrain = BuiltinTerrain::Mountain;
        }
        // Also block all other neighbours so there's truly no path.
        for dir in libhexgrid::coord::HexDir::ALL {
            let nb = state.board.normalize(start + dir.unit_vec());
            if let Some(coord) = nb {
                if coord != mountain {
                    if let Some(t) = state.board.tile_mut(coord) {
                        t.terrain = BuiltinTerrain::Mountain;
                    }
                }
            }
        }

        let uid = spawn_unit(&mut state, civ_id, start, 500);
        let engine = DefaultRulesEngine;
        // Mountain itself is impassable, and all other neighbours blocked -> no path.
        let result = engine.move_unit(&state, uid, mountain);
        assert!(
            matches!(result, Err(RulesError::DestinationImpassable)),
            "move to impassable tile should fail: {:?}", result
        );
        // State must be unaffected.
        assert_eq!(state.unit(uid).unwrap().coord, start);
    }

    #[test]
    fn test_move_unit_partial_move() {
        let (mut state, civ_id) = make_state();
        let start = HexCoord::from_qr(0, 5);
        let far   = HexCoord::from_qr(4, 5);

        // Budget = 150. Each Grassland tile costs 100.
        // Direct path (4 steps): total cost = 400. Unit can only do 1 step (100 <= 150).
        let uid = spawn_unit(&mut state, civ_id, start, 150);
        let engine = DefaultRulesEngine;

        let result = engine.move_unit(&state, uid, far);

        match result {
            Err(RulesError::InsufficientMovement(diff)) => {
                assert!(!diff.is_empty(), "partial diff must record the move that occurred");
                match &diff.deltas[0] {
                    StateDelta::UnitMoved { unit, from, to, .. } => {
                        assert_eq!(*unit, uid);
                        assert_eq!(*from, start);
                        // Moved one step (100 <= 150) but not all four.
                        assert_ne!(*to, start, "unit must have moved at least one tile");
                        assert_ne!(*to, far,   "unit must not have reached the destination");
                    }
                    other => panic!("unexpected delta: {:?}", other),
                }
            }
            other => panic!("expected InsufficientMovement, got {:?}", other),
        }
    }

    // ── compute_yields tests ──────────────────────────────────────────────────

    #[test]
    fn test_compute_yields_uses_worked_tiles() {
        // Verifies that compute_yields sums only worked_tiles (4.1), not all
        // neighbors. The city starts with only the center in worked_tiles (2 food
        // from Grassland). Adding 6 neighbors raises it to 14.
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let city    = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        state.cities.push(city);

        let engine = DefaultRulesEngine;

        // City center only: 2 food.
        let yields = engine.compute_yields(&state, civ_id);
        assert_eq!(yields.food, 2, "only city center worked: 1 Grassland tile = 2 food");

        // Add the 6 neighbors manually.
        add_founding_tiles(state.cities.last_mut().unwrap());
        let yields = engine.compute_yields(&state, civ_id);
        assert_eq!(yields.food, 14, "7 Grassland tiles (center + 6 neighbors) = 14 food");
    }

    #[test]
    fn test_compute_yields_resource_tech_gating() {
        use crate::world::resource::BuiltinResource;

        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let mut city = City::new(city_id, "TestCity".to_string(), civ_id, coord);

        // Place Iron (reveal_tech = "Bronze Working") on the city center tile.
        if let Some(tile) = state.board.tile_mut(coord) {
            tile.resource = Some(BuiltinResource::Iron);
        }
        // Also work the center tile.
        city.worked_tiles = vec![coord];
        state.cities.push(city);

        let engine = DefaultRulesEngine;

        // Without "Bronze Working": resource yields suppressed.
        let yields_no_tech = engine.compute_yields(&state, civ_id);
        // Grassland base = 2 food, 0 production. Iron adds 1 production but is gated.
        assert_eq!(yields_no_tech.production, 0, "Iron production must be suppressed without Bronze Working");

        // "Grant" the civ a fake tech named "Bronze Working" by pushing a fake TechId.
        // Use a TechId whose node in the tech tree has name = "Bronze Working".
        use crate::rules::tech::{TechNode};
        let tech_id = state.id_gen.next_ulid();
        let tech_id = crate::TechId::from_ulid(tech_id);
        state.tech_tree.add_node(TechNode {
            id:   tech_id,
            name: "Bronze Working",
            cost: 100,
            prerequisites: vec![],
            effects: vec![],
            eureka_description: "",
            eureka_effects: vec![],
        });
        state.civilizations.iter_mut()
            .find(|c| c.id == civ_id)
            .unwrap()
            .researched_techs.push(tech_id);

        let yields_with_tech = engine.compute_yields(&state, civ_id);
        assert_eq!(yields_with_tech.production, 1, "Iron production visible after Bronze Working");
    }

    // ── advance_turn tests ────────────────────────────────────────────────────

    #[test]
    fn test_advance_turn_population_grows() {
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let mut city = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        // Give the city 7 worked tiles so it produces 14 food/turn.
        add_founding_tiles(&mut city);
        state.cities.push(city);

        // Grassland gives 14 food/turn (center + 6 neighbors). food_to_grow = 15.
        // Turn 1: food_stored = 14 < 15 -> no growth.
        // Turn 2: food_stored = 28 >= 15 -> growth; reset to 0, population = 2.
        let engine = DefaultRulesEngine;

        let diff1 = engine.advance_turn(&mut state);
        assert_eq!(state.cities[0].population, 1, "no growth after turn 1");
        assert!(!diff1.deltas.iter().any(|d| matches!(d, StateDelta::PopulationGrew { .. })));

        let diff2 = engine.advance_turn(&mut state);
        assert_eq!(state.cities[0].population, 2, "population should grow after turn 2");
        assert!(diff2.deltas.iter().any(|d| matches!(
            d,
            StateDelta::PopulationGrew { city, new_population: 2 } if *city == city_id
        )));
    }

    #[test]
    fn test_advance_turn_population_growth_auto_assigns_tile() {
        // When a city grows, a new worked tile should be auto-assigned.
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let mut city = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        add_founding_tiles(&mut city);
        state.cities.push(city);

        let engine = DefaultRulesEngine;
        let before = state.cities[0].worked_tiles.len();

        // Run until population grows.
        engine.advance_turn(&mut state);
        engine.advance_turn(&mut state);

        assert_eq!(state.cities[0].population, 2, "population grew");
        assert_eq!(
            state.cities[0].worked_tiles.len(),
            before + 1,
            "one new tile auto-assigned on growth"
        );
    }

    #[test]
    fn test_advance_turn_increments_turn_counter() {
        let (mut state, _) = make_state();
        let engine = DefaultRulesEngine;
        assert_eq!(state.turn, 0);
        engine.advance_turn(&mut state);
        assert_eq!(state.turn, 1);
        engine.advance_turn(&mut state);
        assert_eq!(state.turn, 2);
    }

    #[test]
    fn test_advance_turn_production_accumulates() {
        // Cities on Grassland produce 0 production by default. Verify that
        // production_stored does not change on tiles with no production yield.
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(3, 3);
        let city    = City::new(city_id, "Forge".to_string(), civ_id, coord);
        state.cities.push(city);

        let engine = DefaultRulesEngine;
        engine.advance_turn(&mut state);

        // Grassland has 0 production, so production_stored stays at 0.
        assert_eq!(state.cities[0].production_stored, 0);
    }

    // ── assign_citizen tests ──────────────────────────────────────────────────

    #[test]
    fn test_assign_citizen_adds_worked_tile() {
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let city    = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        state.cities.push(city);

        let engine = DefaultRulesEngine;
        let neighbor = HexCoord::from_qr(6, 5); // one step E

        let result = engine.assign_citizen(&mut state, city_id, neighbor, false);
        assert!(result.is_ok(), "assign should succeed: {:?}", result);

        let city = state.cities.iter().find(|c| c.id == city_id).unwrap();
        assert!(city.worked_tiles.contains(&neighbor), "neighbor added to worked_tiles");
        assert!(!city.locked_tiles.contains(&neighbor), "not locked");

        let diff = result.unwrap();
        assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::CitizenAssigned { .. })));
    }

    #[test]
    fn test_assign_citizen_lock_persists() {
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let city    = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        state.cities.push(city);

        let engine = DefaultRulesEngine;
        let neighbor = HexCoord::from_qr(5, 6); // one step SE

        engine.assign_citizen(&mut state, city_id, neighbor, true).unwrap();
        let city = state.cities.iter().find(|c| c.id == city_id).unwrap();
        assert!(city.locked_tiles.contains(&neighbor), "tile is locked");
    }

    #[test]
    fn test_assign_citizen_out_of_range_fails() {
        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let coord   = HexCoord::from_qr(5, 5);
        let city    = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        state.cities.push(city);

        let engine = DefaultRulesEngine;
        // 4 hexes away -- out of the 3-tile working radius.
        let far = HexCoord::from_qr(9, 5);
        let result = engine.assign_citizen(&mut state, city_id, far, false);
        assert!(matches!(result, Err(RulesError::InvalidCoord)), "out-of-range tile should fail");
    }

    // ── capital() method test ─────────────────────────────────────────────────

    #[test]
    fn test_civilization_capital_computed() {
        let (mut state, civ_id) = make_state();
        let civ = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();

        // No cities yet: capital returns None.
        assert!(civ.capital(&state.cities).is_none());

        // Found a capital city.
        let city_id = state.id_gen.next_city_id();
        let mut city = City::new(city_id, "Rome".to_string(), civ_id, HexCoord::from_qr(0, 0));
        city.is_capital = true;
        state.cities.push(city);

        let civ = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
        let cap = civ.capital(&state.cities);
        assert!(cap.is_some(), "capital() should find the capital city");
        assert_eq!(cap.unwrap().id, city_id);
    }

    // ── research_queue tests ──────────────────────────────────────────────────

    #[test]
    fn test_research_queue_advances_on_tech_complete() {
        use crate::civ::civilization::TechProgress;
        use crate::rules::tech::TechNode;
        use crate::world::resource::BuiltinResource;

        let (mut state, civ_id) = make_state();

        // Set up two techs in the tree.
        let tid1 = crate::TechId::from_ulid(state.id_gen.next_ulid());
        let tid2 = crate::TechId::from_ulid(state.id_gen.next_ulid());
        state.tech_tree.add_node(TechNode { id: tid1, name: "Pottery", cost: 25,
            prerequisites: vec![], effects: vec![], eureka_description: "", eureka_effects: vec![] });
        state.tech_tree.add_node(TechNode { id: tid2, name: "Animal Husbandry", cost: 25,
            prerequisites: vec![], effects: vec![], eureka_description: "", eureka_effects: vec![] });

        // Aluminum gives 1 science but requires "Refining". Add Refining to
        // researched_techs so it is ungated, then place Aluminum on the city tile.
        let tid_refining = crate::TechId::from_ulid(state.id_gen.next_ulid());
        state.tech_tree.add_node(TechNode { id: tid_refining, name: "Refining", cost: 9999,
            prerequisites: vec![], effects: vec![], eureka_description: "", eureka_effects: vec![] });
        state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap()
            .researched_techs.push(tid_refining);

        let coord = HexCoord::from_qr(1, 1);
        if let Some(tile) = state.board.tile_mut(coord) {
            tile.resource = Some(BuiltinResource::Aluminum);
        }

        // City working only the center tile (1 science/turn from Aluminum).
        let city_id = state.id_gen.next_city_id();
        let city = City::new(city_id, "TestCity".to_string(), civ_id, coord);
        state.cities.push(city);

        // Queue both techs; first one needs just 1 more science to complete.
        let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
        civ.research_queue.push_back(TechProgress { tech_id: tid1, progress: 24, boosted: false });
        civ.research_queue.push_back(TechProgress { tech_id: tid2, progress: 0, boosted: false });

        let engine = DefaultRulesEngine;
        let diff = engine.advance_turn(&mut state);

        // Aluminum gives 1 science; progress: 24 + 1 = 25 = cost -> tech1 completes.
        let civ = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
        assert!(civ.researched_techs.contains(&tid1), "first tech completed");
        assert_eq!(civ.research_queue.len(), 1, "second tech still queued");
        assert_eq!(civ.research_queue.front().unwrap().tech_id, tid2, "second tech is now front");

        assert!(diff.deltas.iter().any(|d| matches!(
            d, StateDelta::TechResearched { tech: "Pottery", .. }
        )), "TechResearched delta emitted");
    }

    // ── assign_policy tests ───────────────────────────────────────────────────

    fn make_state_with_govt() -> (GameState, CivId) {
        use crate::rules::policy::{Government, PolicySlots};
        use crate::GovernmentId;

        let (mut state, civ_id) = make_state();

        let gov_id = GovernmentId::from_ulid(state.id_gen.next_ulid());
        let gov = Government {
            id: gov_id,
            name: "Autocracy",
            slots: PolicySlots { military: 1, economic: 1, diplomatic: 0, wildcard: 0 },
            inherent_modifiers: vec![],
            legacy_bonus: None,
        };
        state.governments.push(gov);

        // Adopt the government on the civ.
        let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
        civ.current_government = Some(gov_id);
        civ.current_government_name = Some("Autocracy");

        (state, civ_id)
    }

    #[test]
    fn test_assign_policy_success() {
        use crate::rules::policy::Policy;

        let (mut state, civ_id) = make_state_with_govt();
        let engine = DefaultRulesEngine;

        let pol_id = crate::PolicyId::from_ulid(state.id_gen.next_ulid());
        state.policies.push(Policy {
            id: pol_id,
            name: "Strategos",
            policy_type: PolicyType::Military,
            modifiers: vec![],
            maintenance: 0,
        });

        // Unlock the policy for the civ.
        state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap()
            .unlocked_policies.push("Strategos");

        let result = engine.assign_policy(&mut state, civ_id, pol_id);
        assert!(result.is_ok(), "assign_policy should succeed: {:?}", result);

        let civ = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
        assert!(civ.active_policies.contains(&pol_id), "policy is now active");

        let diff = result.unwrap();
        assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::PolicyAssigned { .. })));
    }

    #[test]
    fn test_assign_policy_not_unlocked() {
        use crate::rules::policy::Policy;

        let (mut state, civ_id) = make_state_with_govt();
        let engine = DefaultRulesEngine;

        let pol_id = crate::PolicyId::from_ulid(state.id_gen.next_ulid());
        state.policies.push(Policy {
            id: pol_id,
            name: "Strategos",
            policy_type: PolicyType::Military,
            modifiers: vec![],
            maintenance: 0,
        });
        // Policy NOT added to unlocked_policies.

        let result = engine.assign_policy(&mut state, civ_id, pol_id);
        assert!(
            matches!(result, Err(RulesError::PolicyNotUnlocked)),
            "unlocked check should fail: {:?}", result
        );
    }

    #[test]
    fn test_assign_policy_no_slot() {
        use crate::rules::policy::Policy;

        let (mut state, civ_id) = make_state_with_govt();
        let engine = DefaultRulesEngine;

        // Fill the one military slot.
        let pol1_id = crate::PolicyId::from_ulid(state.id_gen.next_ulid());
        state.policies.push(Policy {
            id: pol1_id, name: "First", policy_type: PolicyType::Military,
            modifiers: vec![], maintenance: 0,
        });
        let pol2_id = crate::PolicyId::from_ulid(state.id_gen.next_ulid());
        state.policies.push(Policy {
            id: pol2_id, name: "Second", policy_type: PolicyType::Military,
            modifiers: vec![], maintenance: 0,
        });

        let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
        civ.unlocked_policies.push("First");
        civ.unlocked_policies.push("Second");
        civ.active_policies.push(pol1_id); // slot already used

        let result = engine.assign_policy(&mut state, civ_id, pol2_id);
        assert!(
            matches!(result, Err(RulesError::InsufficientPolicySlots)),
            "slot check should fail: {:?}", result
        );
    }

    #[test]
    fn test_assign_policy_no_government() {
        use crate::rules::policy::Policy;

        let (mut state, civ_id) = make_state();
        let engine = DefaultRulesEngine;

        let pol_id = crate::PolicyId::from_ulid(state.id_gen.next_ulid());
        state.policies.push(Policy {
            id: pol_id, name: "Free", policy_type: PolicyType::Economic,
            modifiers: vec![], maintenance: 0,
        });
        state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap()
            .unlocked_policies.push("Free");

        // No government adopted; current_government is None.
        let result = engine.assign_policy(&mut state, civ_id, pol_id);
        assert!(matches!(result, Err(RulesError::NoGovernment)), "{:?}", result);
    }

    // ── AdoptGovernment tests ─────────────────────────────────────────────────

    #[test]
    fn test_adopt_government_sets_id_and_evicts_policies() {
        use crate::rules::policy::{Government, Policy, PolicySlots};
        use crate::GovernmentId;

        let (mut state, civ_id) = make_state();

        // Old government: 2 military slots.
        let old_gov_id = GovernmentId::from_ulid(state.id_gen.next_ulid());
        state.governments.push(Government {
            id: old_gov_id, name: "OldGov",
            slots: PolicySlots { military: 2, economic: 0, diplomatic: 0, wildcard: 0 },
            inherent_modifiers: vec![], legacy_bonus: None,
        });

        // New government: only 1 military slot.
        let new_gov_id = GovernmentId::from_ulid(state.id_gen.next_ulid());
        state.governments.push(Government {
            id: new_gov_id, name: "NewGov",
            slots: PolicySlots { military: 1, economic: 0, diplomatic: 0, wildcard: 0 },
            inherent_modifiers: vec![], legacy_bonus: None,
        });

        let pol1_id = crate::PolicyId::from_ulid(state.id_gen.next_ulid());
        let pol2_id = crate::PolicyId::from_ulid(state.id_gen.next_ulid());
        state.policies.push(Policy { id: pol1_id, name: "Pol1", policy_type: PolicyType::Military, modifiers: vec![], maintenance: 0 });
        state.policies.push(Policy { id: pol2_id, name: "Pol2", policy_type: PolicyType::Military, modifiers: vec![], maintenance: 0 });

        let civ = state.civilizations.iter_mut().find(|c| c.id == civ_id).unwrap();
        civ.current_government = Some(old_gov_id);
        civ.current_government_name = Some("OldGov");
        civ.active_policies = vec![pol1_id, pol2_id]; // 2 policies in old govt

        // Apply AdoptGovernment effect.
        let effect = OneShotEffect::AdoptGovernment("NewGov");
        let mut diff = GameStateDiff::new();
        apply_effect(&mut state, civ_id, &effect, &mut diff);

        let civ = state.civilizations.iter().find(|c| c.id == civ_id).unwrap();
        // New government ID set.
        assert_eq!(civ.current_government, Some(new_gov_id), "current_government updated");
        // Only 1 military slot: one policy kept, one evicted.
        assert_eq!(civ.active_policies.len(), 1, "one policy evicted");
        // PolicyUnslotted delta emitted for the removed policy.
        assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::PolicyUnslotted { .. })),
            "PolicyUnslotted delta required");
        assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::GovernmentAdopted { .. })),
            "GovernmentAdopted delta required");
    }

    // ── FreeUnit registry tests ───────────────────────────────────────────────

    #[test]
    fn test_free_unit_with_registry_spawns_basic_unit() {
        use crate::game::state::UnitTypeDef;

        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let city = City::new(city_id, "Rome".to_string(), civ_id, HexCoord::from_qr(0, 0));
        state.cities.push(city);

        let type_id = UnitTypeId::from_ulid(state.id_gen.next_ulid());
        state.unit_type_defs.push(UnitTypeDef {
            id: type_id,
            name: "Warrior",
            production_cost: 40,
            domain: UnitDomain::Land,
            category: UnitCategory::Combat,
            max_movement: 200,
            combat_strength: Some(20),
            range: 0,
            vision_range: 2,
            can_found_city: false,
            resource_cost: None,
            siege_bonus: 0, max_charges: 0,
            exclusive_to: None, replaces: None,
        });

        let effect = OneShotEffect::FreeUnit { unit_type: "Warrior", city: None };
        let mut diff = GameStateDiff::new();
        apply_effect(&mut state, civ_id, &effect, &mut diff);

        assert_eq!(state.units.len(), 1, "one unit spawned");
        assert_eq!(state.units[0].owner, civ_id);
        assert_eq!(state.units[0].max_movement, 200);
        assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::UnitCreated { .. })),
            "UnitCreated delta expected");
    }

    #[test]
    fn test_free_unit_without_registry_emits_placeholder() {
        let (mut state, civ_id) = make_state();
        // No unit_type_defs registered.

        let effect = OneShotEffect::FreeUnit { unit_type: "Catapult", city: None };
        let mut diff = GameStateDiff::new();
        apply_effect(&mut state, civ_id, &effect, &mut diff);

        assert_eq!(state.units.len(), 0, "no unit created without registry");
        assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::FreeUnitGranted { .. })),
            "placeholder delta expected");
    }

    // ── war_weariness modifier test ───────────────────────────────────────────

    #[test]
    fn test_war_weariness_reduces_culture() {
        use crate::civ::diplomacy::{DiplomaticRelation, DiplomaticStatus};

        let (mut state, civ_id) = make_state();
        let city_id = state.id_gen.next_city_id();
        let city = City::new(city_id, "Rome".to_string(), civ_id, HexCoord::from_qr(5, 5));
        state.cities.push(city);

        let engine = DefaultRulesEngine;

        // Baseline culture without war.
        let yields_peace = engine.compute_yields(&state, civ_id);

        // Start a war: create a diplomatic relation with turns_at_war > 0.
        let enemy_id = state.id_gen.next_civ_id();
        let mut rel = DiplomaticRelation::new(civ_id, enemy_id);
        rel.status = DiplomaticStatus::War;
        rel.turns_at_war = 3;
        state.diplomatic_relations.push(rel);

        let yields_war = engine.compute_yields(&state, civ_id);
        assert!(
            yields_war.culture < yields_peace.culture,
            "war weariness should reduce culture (peace={}, war={})",
            yields_peace.culture, yields_war.culture
        );
        assert!(
            yields_war.amenities < yields_peace.amenities,
            "war weariness should reduce amenities"
        );
    }

    // ── Part 7: Diplomacy state machine tests ─────────────────────────────────

    fn make_two_civ_state() -> (GameState, CivId, CivId) {
        let mut state = GameState::new(77, 10, 10);
        let a = state.id_gen.next_civ_id();
        let b = state.id_gen.next_civ_id();
        state.civilizations.push(Civilization::new(a, "CivA", "A", test_leader(a)));
        state.civilizations.push(Civilization::new(b, "CivB", "B", test_leader(b)));
        (state, a, b)
    }

    // ── 7.3: declare_war ──────────────────────────────────────────────────────

    #[test]
    fn test_declare_war_creates_relation_and_emits_delta() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        let diff = engine.declare_war(&mut state, a, b).unwrap();

        // Status is War.
        let rel = state.diplomatic_relations.iter()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
            .unwrap();
        assert_eq!(rel.status, DiplomaticStatus::War);

        // Grievance recorded for the target (b against a = a declared war).
        let total_grievance: i32 = rel.grievances_b_against_a.iter().map(|g| g.amount).sum::<i32>()
            + rel.grievances_a_against_b.iter().map(|g| g.amount).sum::<i32>();
        assert_eq!(total_grievance, 30, "DeclaredWarGrievance amount should be 30");

        // DiplomacyChanged delta emitted.
        assert!(diff.deltas.iter().any(|d| matches!(
            d,
            StateDelta::DiplomacyChanged { new_status: DiplomaticStatus::War, .. }
        )));
    }

    #[test]
    fn test_declare_war_already_at_war_returns_error() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        engine.declare_war(&mut state, a, b).unwrap();
        let err = engine.declare_war(&mut state, a, b).unwrap_err();
        assert!(matches!(err, RulesError::AlreadyAtWar));
    }

    #[test]
    fn test_declare_war_same_civ_returns_error() {
        let (mut state, a, _) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        let err = engine.declare_war(&mut state, a, a).unwrap_err();
        assert!(matches!(err, RulesError::SameCivilization));
    }

    // ── 7.3: make_peace ──────────────────────────────────────────────────────

    #[test]
    fn test_make_peace_resolves_war_and_emits_delta() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        engine.declare_war(&mut state, a, b).unwrap();
        let diff = engine.make_peace(&mut state, a, b).unwrap();

        let rel = state.diplomatic_relations.iter()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
            .unwrap();
        assert_ne!(rel.status, DiplomaticStatus::War, "status should no longer be War");
        assert_eq!(rel.turns_at_war, 0, "turns_at_war should reset to 0");

        assert!(diff.deltas.iter().any(|d| matches!(d, StateDelta::DiplomacyChanged { .. })));
    }

    #[test]
    fn test_make_peace_not_at_war_returns_error() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;
        // Create a neutral relation.
        state.diplomatic_relations.push(DiplomaticRelation::new(a, b));

        let err = engine.make_peace(&mut state, a, b).unwrap_err();
        assert!(matches!(err, RulesError::NotAtWar));
    }

    #[test]
    fn test_make_peace_no_relation_returns_error() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        let err = engine.make_peace(&mut state, a, b).unwrap_err();
        assert!(matches!(err, RulesError::RelationNotFound));
    }

    // ── 7.1: Grievance decay ─────────────────────────────────────────────────

    #[test]
    fn test_grievance_decay_removes_expired_records() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        // Manually add a small grievance that should decay to zero quickly.
        let mut rel = DiplomaticRelation::new(a, b);
        rel.grievances_a_against_b.push(GrievanceRecord {
            grievance_id: state.id_gen.next_grievance_id(),
            description: "test",
            amount: 2,
            visibility: crate::civ::GrievanceVisibility::Public,
            recorded_turn: 0,
        });
        state.diplomatic_relations.push(rel);

        // After 2 advance_turns, amount should reach 0 and be pruned.
        engine.advance_turn(&mut state);
        engine.advance_turn(&mut state);

        let rel = state.diplomatic_relations.iter()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
            .unwrap();
        assert!(rel.grievances_a_against_b.is_empty(), "decayed grievance should be removed");
    }

    #[test]
    fn test_turns_at_war_increments_each_turn() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        engine.declare_war(&mut state, a, b).unwrap();

        // Add a large grievance so War status persists.
        let gid = state.id_gen.next_grievance_id();
        if let Some(rel) = state.diplomatic_relations.iter_mut()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
        {
            rel.grievances_b_against_a.push(GrievanceRecord {
                grievance_id: gid,
                description: "hold war",
                amount: 999,
                visibility: crate::civ::GrievanceVisibility::Public,
                recorded_turn: 0,
            });
        }

        engine.advance_turn(&mut state);
        engine.advance_turn(&mut state);

        let rel = state.diplomatic_relations.iter()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
            .unwrap();
        assert_eq!(rel.turns_at_war, 2, "turns_at_war should increment each turn while at war");
        assert_eq!(rel.status, DiplomaticStatus::War, "war should persist with large grievances");
    }

    // ── 7.2: Opinion-based auto-transition ──────────────────────────────────

    #[test]
    fn test_status_transitions_to_denounced_on_grievance() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        // Two-sided grievances: combined score = (-25 + -25) / 2 = -25 < -20 => Denounced.
        let mut rel = DiplomaticRelation::new(a, b);
        let (gid1, gid2) = (state.id_gen.next_grievance_id(), state.id_gen.next_grievance_id());
        rel.grievances_a_against_b.push(GrievanceRecord {
            grievance_id: gid1,
            description: "large grievance A",
            amount: 25,
            visibility: crate::civ::GrievanceVisibility::Public,
            recorded_turn: 0,
        });
        rel.grievances_b_against_a.push(GrievanceRecord {
            grievance_id: gid2,
            description: "large grievance B",
            amount: 25,
            visibility: crate::civ::GrievanceVisibility::Public,
            recorded_turn: 0,
        });
        state.diplomatic_relations.push(rel);

        // One advance_turn triggers Phase 5 recomputation.
        let diff = engine.advance_turn(&mut state);

        let rel = state.diplomatic_relations.iter()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
            .unwrap();
        assert_eq!(rel.status, DiplomaticStatus::Denounced);

        // DiplomacyChanged delta emitted.
        assert!(diff.deltas.iter().any(|d| matches!(
            d,
            StateDelta::DiplomacyChanged { new_status: DiplomaticStatus::Denounced, .. }
        )));
    }

    #[test]
    fn test_war_persists_while_opinion_below_minus_50() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        engine.declare_war(&mut state, a, b).unwrap();

        // Pump opinion far below -50 so War sticks.
        let gid = state.id_gen.next_grievance_id();
        if let Some(rel) = state.diplomatic_relations.iter_mut()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
        {
            rel.grievances_b_against_a.push(GrievanceRecord {
                grievance_id: gid,
                description: "heavy grievance",
                amount: 999,
                visibility: crate::civ::GrievanceVisibility::Public,
                recorded_turn: 0,
            });
        }
        engine.advance_turn(&mut state);

        let rel = state.diplomatic_relations.iter()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
            .unwrap();
        assert_eq!(rel.status, DiplomaticStatus::War, "war must persist when score < -50");
    }

    #[test]
    fn test_war_auto_resolves_when_grievances_decay() {
        let (mut state, a, b) = make_two_civ_state();
        let engine = DefaultRulesEngine;

        engine.declare_war(&mut state, a, b).unwrap();

        // Leave the initial 30-point DeclaredWar grievance in place.
        // Score = -30/2 = -15, above -50 so War doesn't persist.
        // But score is -15 which is > -20, so status becomes Neutral.
        engine.advance_turn(&mut state);

        let rel = state.diplomatic_relations.iter()
            .find(|r| (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a))
            .unwrap();
        // Score is average of opinion_score_a_toward_b() and opinion_score_b_toward_a().
        // Only one side has the 30-pt grievance (target's grievance against aggressor = -30).
        // Average = (-30 + 0) / 2 = -15, which is > -20 -> Neutral.
        assert_eq!(rel.status, DiplomaticStatus::Neutral,
            "war should auto-resolve to Neutral when grievance score > -50 (got {:?})", rel.status);
    }

    // ── 7.4: Grievance triggers re-exported from civ::grievance ─────────────

    #[test]
    fn test_grievance_triggers_re_exported() {
        use crate::civ::{DeclaredWarGrievance, PillageGrievance, CapturedCityGrievance};
        use crate::civ::diplomacy::GrievanceTrigger;

        assert_eq!(DeclaredWarGrievance.grievance_amount(), 30);
        assert_eq!(PillageGrievance.grievance_amount(), 5);
        assert_eq!(CapturedCityGrievance.grievance_amount(), 20);
    }
}
