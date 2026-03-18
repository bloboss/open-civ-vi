use std::collections::HashSet;
use crate::{AgreementId, CityId, CivId, PolicyId, PolicyType, TechId, UnitId, UnitTypeId, YieldBundle};
use crate::civ::unit::Unit;
use crate::civ::{BasicUnit, DiplomaticRelation, DiplomaticStatus, GrievanceRecord};
use crate::civ::grievance::DeclaredWarGrievance;
use crate::civ::diplomacy::GrievanceTrigger;
use crate::rules::effect::OneShotEffect;
use crate::rules::modifier::{EffectType, resolve_modifiers};
use crate::world::tile::WorldTile;
use libhexgrid::board::HexBoard;
use libhexgrid::coord::{HexCoord, HexDir};
use libhexgrid::types::MovementCost;
use libhexgrid::{HexEdge, HexTile};

use super::board::WorldBoard;
use super::diff::{AttackType, GameStateDiff, StateDelta};
use super::state::GameState;

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
    fn place_improvement(
        &self,
        state: &mut GameState,
        civ_id: CivId,
        coord: HexCoord,
        improvement: crate::world::improvement::BuiltinImprovement,
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

    /// Consume a trader unit and establish a trade route to `destination`.
    ///
    /// The trader must be located at a city tile owned by its civilization (origin city).
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

    // TODO(PHASE3-BORDERS): fn purchase_tile(&self, state: &mut GameState, city_id: CityId,
    //   coord: HexCoord) -> Result<GameStateDiff, RulesError>;
    //   Spends gold (or culture) from the civilization's treasury to immediately claim a tile
    //   within radius 3 of the city. Cost scales with tile distance. Distinct from automatic
    //   cultural expansion: this is a player action and debits the main treasury.
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
    /// The great person ID was not found in `state.great_people`.
    GreatPersonNotFound,
    /// The great person has already been retired.
    GreatPersonAlreadyRetired,
    /// No great person definition found matching this great person's name.
    GreatPersonDefNotFound,
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
            RulesError::GreatPersonNotFound            => write!(f, "great person not found"),
            RulesError::GreatPersonAlreadyRetired      => write!(f, "great person already retired"),
            RulesError::GreatPersonDefNotFound         => write!(f, "great person definition not found"),
        }
    }
}

impl std::error::Error for RulesError {}

// ── DefaultRulesEngine ────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DefaultRulesEngine;

/// Claim a single tile for a city, emitting `TileClaimed` if the tile was unclaimed.
/// Skips enemy-owned tiles silently (happens at map edge during founding near a rival).
fn try_claim_tile(
    state: &mut GameState,
    civ_id: CivId,
    city_id: CityId,
    coord: HexCoord,
    diff: &mut GameStateDiff,
) {
    let newly_claimed = if let Some(t) = state.board.tile_mut(coord) {
        match t.owner {
            Some(owner) if owner == civ_id => false, // already ours, no delta
            Some(_) => false,                         // enemy tile, skip
            None => {
                t.owner = Some(civ_id);
                true
            }
        }
    } else {
        false
    };

    if newly_claimed {
        if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
            city.territory.insert(coord);
        }
        diff.push(StateDelta::TileClaimed { civ: civ_id, city: city_id, coord });
    }
}

/// Cost in shadow-culture to claim a tile at `distance` hexes from the city center.
/// Ring 1 tiles are free and claimed automatically at founding; this is only
/// called for distances 2–5.
fn tile_border_cost(distance: u32) -> u32 {
    (10.0 + (6.0 * distance as f64).powf(1.3)) as u32
}

/// Per-city culture output for one turn: 1 base culture plus culture from
/// worked tiles. The base matches the per-city culture added in `compute_yields`.
/// Used exclusively for the border expansion shadow accumulator; does not
/// affect the civilization's culture pool.
fn city_culture_output(board: &WorldBoard, city: &crate::civ::City) -> u32 {
    let tile_culture: u32 = city.worked_tiles.iter()
        .filter_map(|&c| board.tile(c))
        .map(|t| t.total_yields().culture.max(0) as u32)
        .sum();
    1 + tile_culture
}

// ── Loyalty pressure helpers ─────────────────────────────────────────────────

/// Maximum distance (hex tiles) at which a foreign city exerts loyalty pressure.
const LOYALTY_PRESSURE_RADIUS: u32 = 9;

/// Base loyalty pressure exerted by a single city at distance `d`.
/// Falls off as `10 / d` so closer cities exert dramatically more pressure.
/// Returns 0 when `d == 0` (the city itself) or `d > LOYALTY_PRESSURE_RADIUS`.
fn city_loyalty_pressure_at_distance(population: u32, distance: u32) -> i32 {
    if distance == 0 || distance > LOYALTY_PRESSURE_RADIUS {
        return 0;
    }
    // Base pressure scales with population: each pop adds +1 base pressure.
    let base = population as i32;
    // Fall-off: 10 / d, so adjacent city (d=1) gives full 10×base,
    // d=2 gives 5×base, d=9 gives ~1×base.
    (base * 10) / distance as i32
}

/// Compute the net loyalty delta for a single city this turn.
///
/// Positive delta means loyalty trends toward max (owner has strong presence);
/// negative means foreign pressure is eroding loyalty.
///
/// Sources of loyalty pressure:
/// - **Domestic cities**: Each city owned by the same civ within
///   `LOYALTY_PRESSURE_RADIUS` adds positive pressure proportional to population
///   and inversely proportional to distance.
/// - **Foreign cities**: Each city owned by a different civ within
///   `LOYALTY_PRESSURE_RADIUS` adds negative pressure (same formula).
/// - **Capital bonus**: The city's own civ's capital within range adds +5 flat.
/// - **Occupied penalty**: Occupied cities suffer -5 per turn base penalty.
/// - **Governor bonus**: An established governor in this city adds +8 loyalty/turn.
/// - **Population bonus**: City's own population adds +1 per 2 pop (loyalty
///   from its own citizens).
fn compute_city_loyalty_delta(
    city_idx: usize,
    cities: &[crate::civ::City],
    governors: &[crate::civ::Governor],
) -> i32 {
    let city = &cities[city_idx];
    let owner = city.owner;
    let coord = city.coord;

    // Skip city-states — they don't participate in the loyalty system.
    if matches!(city.kind, crate::civ::city::CityKind::CityState(_)) {
        return 0;
    }

    let mut domestic_pressure: i32 = 0;
    let mut foreign_pressure: i32 = 0;
    let mut has_capital_nearby = false;

    for (i, other) in cities.iter().enumerate() {
        if i == city_idx {
            continue;
        }
        // Skip city-states as pressure sources.
        if matches!(other.kind, crate::civ::city::CityKind::CityState(_)) {
            continue;
        }
        let dist = coord.distance(&other.coord);
        if dist > LOYALTY_PRESSURE_RADIUS {
            continue;
        }
        let pressure = city_loyalty_pressure_at_distance(other.population, dist);
        if other.owner == owner {
            domestic_pressure += pressure;
            if other.is_capital {
                has_capital_nearby = true;
            }
        } else {
            foreign_pressure += pressure;
        }
    }

    let mut delta: i32 = 0;

    // Net city pressure: domestic pushes loyalty up, foreign pushes it down.
    delta += domestic_pressure - foreign_pressure;

    // Capital proximity bonus.
    if has_capital_nearby {
        delta += 5;
    }

    // Occupied city penalty: loyalty erodes faster.
    if city.ownership == crate::civ::city::CityOwnership::Occupied {
        delta -= 5;
    }

    // Governor bonus: an established governor stabilizes loyalty.
    let has_governor = governors.iter().any(|g| {
        g.owner == owner
            && g.assigned_city == Some(city.id)
            && g.is_established()
    });
    if has_governor {
        delta += 8;
    }

    // Self-population bonus: the city's own citizens contribute loyalty.
    delta += city.population as i32 / 2;

    // Is-capital bonus: capitals are naturally more loyal.
    if city.is_capital {
        delta += 10;
    }

    // Clamp the delta so loyalty changes are gradual (max ±20 per turn).
    delta.clamp(-20, 20)
}

/// Find the civilization exerting the highest foreign loyalty pressure on a city.
/// Returns `None` if no foreign civ exerts any pressure (city becomes Free City).
fn highest_pressure_civ(
    city_idx: usize,
    cities: &[crate::civ::City],
) -> Option<CivId> {
    let city = &cities[city_idx];
    let owner = city.owner;
    let coord = city.coord;

    let mut pressure_by_civ: Vec<(CivId, i32)> = Vec::new();

    for (i, other) in cities.iter().enumerate() {
        if i == city_idx {
            continue;
        }
        if other.owner == owner {
            continue;
        }
        if matches!(other.kind, crate::civ::city::CityKind::CityState(_)) {
            continue;
        }
        let dist = coord.distance(&other.coord);
        let pressure = city_loyalty_pressure_at_distance(other.population, dist);
        if pressure > 0 {
            if let Some(entry) = pressure_by_civ.iter_mut().find(|(c, _)| *c == other.owner) {
                entry.1 += pressure;
            } else {
                pressure_by_civ.push((other.owner, pressure));
            }
        }
    }

    pressure_by_civ.sort_by(|a, b| b.1.cmp(&a.1));
    pressure_by_civ.first().map(|(civ, _)| *civ)
}

impl RulesEngine for DefaultRulesEngine {
    fn move_unit(
        &self,
        state: &GameState,
        unit_id: UnitId,
        to: HexCoord,
    ) -> Result<GameStateDiff, RulesError> {
        let unit = state.unit(unit_id).ok_or(RulesError::UnitNotFound)?;
        let from   = unit.coord();
        let budget = unit.movement_left();

        let to_norm = state.board.normalize(to).ok_or(RulesError::InvalidCoord)?;

        // Determine whether a path to the destination exists at all.
        let full_path = state.board
            .find_path(from, to_norm, u32::MAX)
            .ok_or(RulesError::DestinationImpassable)?;

        // Walk the path, consuming movement budget step by step.
        let mut spent  = 0u32;
        let mut reached = from;

        for i in 1..full_path.len() {
            let prev = full_path[i - 1];
            let next = full_path[i];

            let tile_cost = match state.board.tile(next) {
                Some(t) => {
                    let base = match t.road.as_ref() {
                        Some(r) => r.as_def().movement_cost(),
                        None    => t.movement_cost(),
                    };
                    match base {
                        MovementCost::Impassable => break,
                        MovementCost::Cost(c)    => c,
                    }
                }
                None => break,
            };

            // Edge crossing cost: free (0) when no edge feature exists.
            let edge_cost: u32 = {
                let crossing = neighbor_dir(&state.board, prev, next)
                    .and_then(|dir| state.board.edge(prev, dir))
                    .map(|e| e.crossing_cost());
                match crossing {
                    Some(MovementCost::Impassable) => break,
                    Some(MovementCost::Cost(c))    => c,
                    None                           => 0,
                }
            };

            let step = tile_cost + edge_cost;
            if spent + step > budget {
                break;
            }
            spent   += step;
            reached  = next;
        }

        let mut diff = GameStateDiff::new();

        if reached == from {
            // Zero movement occurred (budget was 0 or first step too costly).
            return Err(RulesError::InsufficientMovement(diff));
        }

        // Occupancy check: reject if the destination is held by any other unit.
        if let Some(occupant) = state.units.iter().find(|u| u.id != unit_id && u.coord == reached) {
            let mover_owner      = state.unit(unit_id).map(|u| u.owner);
            let mover_can_attack = state.unit(unit_id).and_then(|u| u.combat_strength).is_some();
            if occupant.owner == mover_owner.unwrap_or(occupant.owner) {
                // Friendly unit on destination — stacking not allowed.
                return Err(RulesError::TileOccupiedByUnit);
            } else if !mover_can_attack {
                // Civilian trying to move onto an enemy — it cannot fight back.
                return Err(RulesError::UnitCannotAttack);
            } else {
                // Combat unit vs enemy: player must call attack() explicitly.
                return Err(RulesError::TileOccupiedByUnit);
            }
        }

        diff.push(StateDelta::UnitMoved {
            unit: unit_id,
            from,
            to: reached,
            cost: spent });

        if reached == to_norm {
            Ok(diff)
        } else {
            // Partial move: unit moved but did not reach the destination.
            Err(RulesError::InsufficientMovement(diff))
        }
    }

    fn compute_yields(&self, state: &GameState, civ_id: CivId) -> YieldBundle {
        let mut total = YieldBundle::default();

        // Build the set of researched tech names for resource tech-gating (4.2).
        let known_techs: HashSet<&str> = state.civ(civ_id)
            .map(|civ| {
                state.tech_tree.nodes.values()
                    .filter(|n| civ.researched_techs.contains(&n.id))
                    .map(|n| n.name)
                    .collect()
            })
            .unwrap_or_default();

        // Sum yields only from worked tiles (4.1: replaces 7-tile approximation).
        // Resource yields are suppressed when the civ lacks the reveal tech (4.2).
        for city in state.cities.iter().filter(|c| c.owner == civ_id) {
            for &coord in &city.worked_tiles {
                if let Some(tile) = state.board.tile(coord) {
                    total += tile_yields_gated(tile, &known_techs);
                }
            }
        }

        // Base yields: every city contributes 1 science and 1 culture per turn.
        let city_count = state.cities.iter().filter(|c| c.owner == civ_id).count();
        total.science += city_count as i32;
        total.culture += city_count as i32;

        // ── Trade route yields ────────────────────────────────────────────────
        // Origin side: routes owned by this civ deliver origin_yields.
        for route in &state.trade_routes {
            if route.owner == civ_id {
                total += route.origin_yields.clone();
            }
        }
        // Destination side: destination city's owner receives destination_yields
        // (only for routes owned by a *different* civ — avoids double-counting domestic).
        for route in &state.trade_routes {
            let dest_owner = state.cities.iter()
                .find(|c| c.id == route.destination)
                .map(|c| c.owner);
            if dest_owner == Some(civ_id) && route.owner != civ_id {
                total += route.destination_yields.clone();
            }
        }

        // Collect modifiers: base sources (leader/policies/govt/war) + tech/civic tree grants.
        let modifiers = state.civ(civ_id)
            .map(|civ| {
                let mut mods = civ.get_modifiers(
                    &state.policies,
                    &state.governments,
                    &state.diplomatic_relations,
                );
                mods.extend(civ.get_tree_modifiers(&state.tech_tree, &state.civic_tree));
                mods
            })
            .unwrap_or_default();

        let effects = resolve_modifiers(&modifiers);
        apply_effects(&effects, total)
    }

    fn advance_turn(&self, state: &mut GameState) -> GameStateDiff {
        let mut diff = GameStateDiff::new();

        // ── Reset per-turn city bombardment flag ─────────────────────────────
        for city in &mut state.cities {
            city.has_attacked_this_turn = false;
        }

        // ── Per-city food accumulation and population growth ──────────────────
        // Collect food from worked_tiles (immutable board borrow), then mutate cities.
        let city_food: Vec<(usize, i32)> = {
            let board = &state.board;
            state.cities.iter().enumerate().map(|(i, city)| {
                let food: i32 = city.worked_tiles.iter()
                    .filter_map(|&coord| board.tile(coord))
                    .map(|t| t.total_yields().food)
                    .sum();
                (i, food)
            }).collect()
        };

        // Track cities that grew so we can auto-assign a new citizen after the loop.
        let mut grew_cities: Vec<usize> = Vec::new();

        for (i, food) in city_food {
            let city = &mut state.cities[i];
            if food > 0 {
                city.food_stored += food as u32;
            }
            if city.food_stored >= city.food_to_grow {
                city.food_stored  = 0;
                city.population  += 1;
                city.food_to_grow = 15 + 6 * (city.population - 1);
                diff.push(StateDelta::PopulationGrew {
                    city: city.id,
                    new_population: city.population,
                });
                grew_cities.push(i);
            }
        }

        // Auto-assign a new worked tile for each city that just grew.
        for i in grew_cities {
            auto_assign_citizen(&state.board, &mut state.cities[i]);
        }

        // ── Per-city production accumulation + strategic resource yield ────────
        // Collect production yield and strategic resource tiles from worked tiles
        // (immutable board borrow), then apply mutations in separate passes.
        use crate::world::resource::BuiltinResource;
        use crate::enums::ResourceCategory;
        use std::collections::HashMap as StdHashMap;

        struct CityTurnData {
            city_idx: usize,
            civ_id:   CivId,
            coord:    HexCoord,
            prod:     u32,
            /// Strategic resources yielded this turn by worked tiles with an improvement.
            resource_yields: Vec<BuiltinResource>,
        }

        let city_turn_data: Vec<CityTurnData> = {
            let board = &state.board;
            state.cities.iter().enumerate().map(|(i, city)| {
                let prod: i32 = city.worked_tiles.iter()
                    .filter_map(|&coord| board.tile(coord))
                    .map(|t| t.total_yields().production)
                    .sum();
                let resource_yields: Vec<BuiltinResource> = city.worked_tiles.iter()
                    .filter_map(|&coord| board.tile(coord))
                    .filter_map(|t| {
                        let res = t.resource?;
                        if res.category() == ResourceCategory::Strategic && t.improvement.is_some() {
                            Some(res)
                        } else {
                            None
                        }
                    })
                    .collect();
                CityTurnData {
                    city_idx: i,
                    civ_id:   city.owner,
                    coord:    city.coord,
                    prod:     prod.max(0) as u32,
                    resource_yields,
                }
            }).collect()
        };

        // Apply production accumulation.
        for d in &city_turn_data {
            state.cities[d.city_idx].production_stored += d.prod;
        }

        // Aggregate strategic resource gains per (civ, resource) and apply.
        let mut resource_gains: StdHashMap<(CivId, BuiltinResource), u32> = StdHashMap::new();
        for d in &city_turn_data {
            for &res in &d.resource_yields {
                *resource_gains.entry((d.civ_id, res)).or_insert(0) += 1;
            }
        }
        for ((civ_id, resource), amount) in resource_gains {
            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
                *civ.strategic_resources.entry(resource).or_insert(0) += amount;
                diff.push(StateDelta::StrategicResourceChanged { civ: civ_id, resource, delta: amount as i32 });
            }
        }

        // Complete unit production for cities whose stored production meets the cost.
        // Units with a strategic resource cost are blocked if the civ lacks the resource.
        struct UnitCompletion {
            city_idx:        usize,
            civ_id:          CivId,
            coord:           HexCoord,
            type_id:         UnitTypeId,
            production_cost: u32,
            resource_cost:   Option<(BuiltinResource, u32)>,
            domain:          crate::UnitDomain,
            category:        crate::UnitCategory,
            max_movement:    u32,
            combat_strength: Option<u32>,
            range:           u8,
            vision_range:    u8,
        }

        let unit_completions: Vec<UnitCompletion> = city_turn_data.iter().filter_map(|d| {
            use crate::civ::city::ProductionItem;
            let city = &state.cities[d.city_idx];
            if let Some(ProductionItem::Unit(tid)) = city.production_queue.front() {
                let def = state.unit_type_defs.iter().find(|def| def.id == *tid)?;
                if city.production_stored >= def.production_cost {
                    Some(UnitCompletion {
                        city_idx:        d.city_idx,
                        civ_id:          d.civ_id,
                        coord:           d.coord,
                        type_id:         def.id,
                        production_cost: def.production_cost,
                        resource_cost:   def.resource_cost,
                        domain:          def.domain,
                        category:        def.category,
                        max_movement:    def.max_movement,
                        combat_strength: def.combat_strength,
                        range:           def.range,
                        vision_range:    def.vision_range,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }).collect();

        for uc in unit_completions {
            // Check and deduct strategic resource cost.
            if let Some((resource, required)) = uc.resource_cost {
                let available = state.civilizations.iter()
                    .find(|c| c.id == uc.civ_id)
                    .map(|c| *c.strategic_resources.get(&resource).unwrap_or(&0))
                    .unwrap_or(0);
                if available < required {
                    continue; // Insufficient — defer until resources are available.
                }
                if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == uc.civ_id) {
                    *civ.strategic_resources.entry(resource).or_insert(0) -= required;
                    diff.push(StateDelta::StrategicResourceChanged {
                        civ: uc.civ_id, resource, delta: -(required as i32),
                    });
                }
            }

            // Deduct production cost and complete the item.
            state.cities[uc.city_idx].production_stored -= uc.production_cost;
            state.cities[uc.city_idx].production_queue.pop_front();

            let unit_id = state.id_gen.next_unit_id();
            state.units.push(crate::civ::BasicUnit {
                id:              unit_id,
                unit_type:       uc.type_id,
                owner:           uc.civ_id,
                coord:           uc.coord,
                domain:          uc.domain,
                category:        uc.category,
                movement_left:   uc.max_movement,
                max_movement:    uc.max_movement,
                combat_strength: uc.combat_strength,
                promotions:      Vec::new(),
                health:          100,
                range:           uc.range,
                vision_range:    uc.vision_range,
            });
            diff.push(StateDelta::UnitCreated { unit: unit_id, coord: uc.coord, owner: uc.civ_id });
            // TODO(PHASE3-4.3): Building, District, Wonder completion.
        }

        // ── Phase 2b: Trade route countdown ──────────────────────────────────
        // Expire routes that reached turns_remaining == 0 last turn (they already
        // delivered on all their turns), then decrement remaining routes.
        {
            use crate::TradeRouteId;
            let expired: Vec<TradeRouteId> = state.trade_routes.iter()
                .filter(|r| r.turns_remaining == Some(0))
                .map(|r| r.id)
                .collect();
            for id in &expired {
                state.trade_routes.retain(|r| r.id != *id);
                diff.push(StateDelta::TradeRouteExpired { route: *id });
            }
            for route in state.trade_routes.iter_mut() {
                if let Some(ref mut t) = route.turns_remaining {
                    *t = t.saturating_sub(1);
                }
            }
        }

        // ── Per-civ yields: gold, science, culture ────────────────────────────
        let civ_ids: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();

        // Collect yields while state is immutably borrowed.
        let civ_yields: Vec<(CivId, YieldBundle)> = civ_ids.iter()
            .map(|&id| (id, self.compute_yields(state, id)))
            .collect();

        // Apply gold.
        for (civ_id, yields) in &civ_yields {
            if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id)
                && yields.gold != 0
            {
                civ.gold += yields.gold;
                diff.push(StateDelta::GoldChanged { civ: *civ_id, delta: yields.gold });
            }
        }

        // Apply science -> tech progress and check completion.
        // Two-pass: first update progress (mutates civilizations), then check
        // tech_tree (different field, disjoint borrow).
        struct TechCheck { civ_idx: usize, civ_id: CivId, tech_id: TechId, progress: u32 }
        let mut tech_checks: Vec<TechCheck> = Vec::new();

        for (civ_id, yields) in &civ_yields {
            if yields.science <= 0 { continue; }
            if let Some((idx, civ)) = state.civilizations.iter_mut()
                .enumerate().find(|(_, c)| c.id == *civ_id)
                && let Some(tp) = civ.research_queue.front_mut()
            {
                tp.progress += yields.science as u32;
                tech_checks.push(TechCheck {
                    civ_idx: idx,
                    civ_id:  *civ_id,
                    tech_id: tp.tech_id,
                    progress: tp.progress,
                });
            }
        }

        for tc in tech_checks {
            let node_info = state.tech_tree.get(tc.tech_id)
                .map(|n| (n.cost, n.name, n.effects.clone()));
            if let Some((cost, name, effects)) = node_info {
                let boosted = state.civilizations[tc.civ_idx]
                    .research_queue.front()
                    .map(|tp| tp.boosted)
                    .unwrap_or(false);
                let effective_cost = if boosted { cost / 2 } else { cost };
                if tc.progress >= effective_cost {
                    state.civilizations[tc.civ_idx].researched_techs.push(tc.tech_id);
                    state.civilizations[tc.civ_idx].research_queue.pop_front();
                    diff.push(StateDelta::TechResearched { civ: tc.civ_id, tech: name });
                    for effect in effects {
                        state.effect_queue.push_back((tc.civ_id, effect));
                    }
                }
            }
        }

        // Apply culture -> civic progress (same pattern as science).
        struct CivicCheck { civ_idx: usize, civ_id: CivId, civic_id: crate::CivicId, progress: u32 }
        let mut civic_checks: Vec<CivicCheck> = Vec::new();

        for (civ_id, yields) in &civ_yields {
            if yields.culture <= 0 { continue; }
            if let Some((idx, civ)) = state.civilizations.iter_mut()
                .enumerate().find(|(_, c)| c.id == *civ_id)
                && let Some(cp) = civ.civic_in_progress.as_mut()
            {
                cp.progress += yields.culture as u32;
                civic_checks.push(CivicCheck {
                    civ_idx: idx,
                    civ_id:  *civ_id,
                    civic_id: cp.civic_id,
                    progress: cp.progress,
                });
            }
        }

        for cc in civic_checks {
            let node_info = state.civic_tree.get(cc.civic_id)
                .map(|n| (n.cost, n.name, n.effects.clone()));
            if let Some((cost, name, effects)) = node_info {
                let inspired = state.civilizations[cc.civ_idx]
                    .civic_in_progress.as_ref()
                    .map(|cp| cp.inspired)
                    .unwrap_or(false);
                let effective_cost = if inspired { cost / 2 } else { cost };
                if cc.progress >= effective_cost {
                    state.civilizations[cc.civ_idx].completed_civics.push(cc.civic_id);
                    state.civilizations[cc.civ_idx].civic_in_progress = None;
                    diff.push(StateDelta::CivicCompleted { civ: cc.civ_id, civic: name });
                    for effect in effects {
                        state.effect_queue.push_back((cc.civ_id, effect));
                    }
                }
            }
        }

        // ── Phase 3a: Tourism accumulation ────────────────────────────────────
        // Accumulate lifetime culture from this turn's culture output, compute
        // tourism, and distribute tourism pressure to other civilizations.
        {
            use crate::civ::tourism::compute_tourism;

            let civ_ids_for_tourism: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();

            // Accumulate lifetime culture from this turn's culture yield.
            for (civ_id, yields) in &civ_yields {
                if yields.culture > 0
                    && let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id)
                {
                    civ.lifetime_culture += yields.culture as u32;
                }
            }

            // Compute tourism and distribute to all other civs.
            // Must collect tourism values first (immutable borrow) then mutate.
            let tourism_outputs: Vec<(CivId, u32)> = civ_ids_for_tourism.iter()
                .map(|&id| (id, compute_tourism(state, id)))
                .collect();

            for (civ_id, tourism) in &tourism_outputs {
                if *tourism == 0 { continue; }

                // Find the civ's lifetime culture for the delta.
                let lifetime_culture = state.civ(*civ_id)
                    .map(|c| c.lifetime_culture)
                    .unwrap_or(0);

                diff.push(StateDelta::TourismGenerated {
                    civ: *civ_id,
                    tourism: *tourism,
                    lifetime_culture,
                });

                // Distribute tourism equally to all other civs.
                let other_count = civ_ids_for_tourism.len().saturating_sub(1);
                if other_count == 0 { continue; }

                let per_civ = *tourism; // Each other civ gets full tourism pressure.
                if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == *civ_id) {
                    for &target in &civ_ids_for_tourism {
                        if target != *civ_id {
                            *civ.tourism_accumulated.entry(target).or_insert(0) += per_civ;
                        }
                    }
                }
            }
        }

        // ── Phase 3b: Cultural border expansion ───────────────────────────────
        // For each regular city, accumulate shadow culture and claim the cheapest
        // unclaimed tile within radius 2–5 while the budget allows.
        // City-states use different expansion rules (TBD); they are skipped here.
        // TODO(PHASE3-BORDERS-CITYSTATE): implement city-state territory expansion.
        let city_indices: Vec<usize> = (0..state.cities.len()).collect();
        for city_idx in city_indices {
            {
                let city = &state.cities[city_idx];
                // Skip city-states — different expansion rules TBD.
                if matches!(city.kind, crate::civ::city::CityKind::CityState(_)) {
                    continue;
                }
            }
            let (city_coord, civ_id, culture) = {
                let city = &state.cities[city_idx];
                (city.coord, city.owner, city_culture_output(&state.board, city))
            };
            state.cities[city_idx].culture_border += culture;

            loop {
                // Collect unclaimed candidates at radius 2–5 (re-evaluated each
                // iteration so that tiles claimed in this same turn are not re-selected).
                let candidates: Vec<(u32, HexCoord)> = state.board.all_coords()
                    .into_iter()
                    .filter(|&coord| {
                        let dist = city_coord.distance(&coord);
                        (2..=5).contains(&dist)
                            && state.board.tile(coord)
                                .map(|t| t.owner != Some(civ_id))
                                .unwrap_or(false)
                    })
                    .map(|coord| (tile_border_cost(city_coord.distance(&coord)), coord))
                    .collect();

                let Some(&(min_cost, _)) = candidates.iter().min_by_key(|(c, _)| c) else {
                    break;
                };
                if state.cities[city_idx].culture_border < min_cost {
                    break;
                }

                let cheapest: Vec<HexCoord> = candidates.iter()
                    .filter(|(c, _)| *c == min_cost)
                    .map(|(_, coord)| *coord)
                    .collect();

                let chosen = if cheapest.len() == 1 {
                    cheapest[0]
                } else {
                    let idx = (state.id_gen.next_f32() * cheapest.len() as f32) as usize;
                    cheapest[idx.min(cheapest.len() - 1)]
                };

                state.cities[city_idx].culture_border -= min_cost;
                let city_id = state.cities[city_idx].id;
                try_claim_tile(state, civ_id, city_id, chosen, &mut diff);
            }
        }

        // ── Phase 3c: Loyalty pressure ──────────────────────────────────────────
        // For each non-city-state city, compute the loyalty delta from nearby
        // friendly/foreign cities, governor bonuses, occupation penalties, etc.
        // When loyalty reaches 0 the city revolts: it flips to the civ exerting
        // the highest foreign pressure, or becomes independent if none.
        {
            let num_cities = state.cities.len();
            // Compute deltas first (immutable borrow of cities slice).
            let loyalty_deltas: Vec<(usize, i32)> = (0..num_cities)
                .map(|i| (i, compute_city_loyalty_delta(i, &state.cities, &state.governors)))
                .filter(|(_, d)| *d != 0)
                .collect();

            // Apply loyalty changes.
            for &(city_idx, delta) in &loyalty_deltas {
                let city = &mut state.cities[city_idx];
                let old = city.loyalty;
                city.loyalty = (old + delta).clamp(0, 100);
                if city.loyalty != old {
                    diff.push(StateDelta::LoyaltyChanged {
                        city: city.id,
                        delta: city.loyalty - old,
                        new_value: city.loyalty,
                    });
                }
            }

            // Handle revolts (loyalty == 0). Collect revolt info first, then mutate.
            let revolts: Vec<(usize, CityId, CivId)> = (0..state.cities.len())
                .filter_map(|i| {
                    let c = &state.cities[i];
                    if c.loyalty == 0
                        && !matches!(c.kind, crate::civ::city::CityKind::CityState(_))
                    {
                        Some((i, c.id, c.owner))
                    } else {
                        None
                    }
                })
                .collect();

            // Determine new owner for each revolting city.
            let revolt_targets: Vec<(usize, CityId, CivId, Option<CivId>)> = revolts.iter()
                .map(|&(idx, cid, old_owner)| {
                    let new_owner = highest_pressure_civ(idx, &state.cities);
                    (idx, cid, old_owner, new_owner)
                })
                .collect();

            for (city_idx, city_id, old_owner, new_owner) in revolt_targets {
                let city = &mut state.cities[city_idx];
                if let Some(new_civ) = new_owner {
                    // Flip to the pressuring civilization.
                    city.owner = new_civ;
                    city.ownership = crate::civ::city::CityOwnership::Occupied;
                    city.loyalty = 50; // starts at reduced loyalty under new owner

                    // Update civ city lists.
                    if let Some(old_civ) = state.civilizations.iter_mut().find(|c| c.id == old_owner) {
                        old_civ.cities.retain(|&id| id != city_id);
                    }
                    if let Some(new_civ_obj) = state.civilizations.iter_mut().find(|c| c.id == new_civ) {
                        new_civ_obj.cities.push(city_id);
                    }

                    // Update tile ownership for city's territory.
                    let territory: Vec<HexCoord> = city.territory.iter().copied().collect();
                    for coord in territory {
                        if let Some(tile) = state.board.tile_mut(coord) {
                            tile.owner = Some(new_civ);
                        }
                    }
                } else {
                    // No foreign pressure — city becomes independent (Free City).
                    // Remove from old owner's city list; owner stays but ownership
                    // is set to Occupied to signal the city is in revolt.
                    // In a full implementation this would create a new "Free City"
                    // civilization; for now we leave the owner and mark Occupied.
                    city.ownership = crate::civ::city::CityOwnership::Occupied;
                    city.loyalty = 25; // low loyalty as an independent
                }

                diff.push(StateDelta::CityRevolted {
                    city: city_id,
                    new_owner,
                    old_owner,
                });
            }
        }

        // ── Phase 4: Drain effect queue ───────────────────────────────────────
        // Take the queue out of state so apply_effect can borrow state mutably.
        // apply_effect returns () and never re-enqueues, so the loop terminates.
        let pending = std::mem::take(&mut state.effect_queue);
        for (civ_id, effect) in &pending {
            let should_apply = state.civilizations.iter()
                .find(|c| c.id == *civ_id)
                .map(|civ| effect.guard(civ))
                .unwrap_or(false);
            if should_apply {
                apply_effect(state, *civ_id, effect, &mut diff);
            }
        }
        // Any effects pushed during apply_effect (none expected) would stay in
        // state.effect_queue for the next turn. pending is dropped here.

        // ── Phase 5: Diplomacy — grievance decay and status recomputation ─────
        // Decay each grievance by 1 per turn; drop records that reach zero.
        // Increment turns_at_war for warring pairs.
        // Recompute status from the combined opinion score and emit a delta when
        // it changes.
        const GRIEVANCE_DECAY: i32 = 1;
        let mut status_changes: Vec<(CivId, CivId, DiplomaticStatus)> = Vec::new();
        for rel in state.diplomatic_relations.iter_mut() {
            for rec in rel.grievances_a_against_b.iter_mut() {
                rec.amount -= GRIEVANCE_DECAY;
            }
            rel.grievances_a_against_b.retain(|r| r.amount > 0);
            for rec in rel.grievances_b_against_a.iter_mut() {
                rec.amount -= GRIEVANCE_DECAY;
            }
            rel.grievances_b_against_a.retain(|r| r.amount > 0);

            if rel.status == DiplomaticStatus::War {
                rel.turns_at_war += 1;
            }

            let new_status = compute_diplomatic_status(rel);
            if new_status != rel.status {
                status_changes.push((rel.civ_a, rel.civ_b, new_status));
            }
        }
        for (civ_a, civ_b, new_status) in status_changes {
            if let Some(rel) = state.diplomatic_relations.iter_mut()
                .find(|r| r.civ_a == civ_a && r.civ_b == civ_b)
            {
                rel.status = new_status;
            }
            diff.push(StateDelta::DiplomacyChanged { civ_a, civ_b, new_status });
        }

        // TODO(PHASE3-8.3): Deliver trade route yields; decrement turns_remaining; expire routes.
        // TODO(PHASE3-8.5): Compute religion spread per city pair; update Religion.followers.
        // TODO(PHASE3-8.6): Accumulate great_person_points yield into per-type counters.
        // TODO(PHASE3-8.7): Decrement turns_to_establish for assigned governors.
        // TODO(PHASE3-8.8): Evaluate EraTrigger conditions; emit EraAdvanced.

        // ── Phase 5b: Victory condition evaluation ────────────────────────────
        if state.game_over.is_none() {
            use super::victory::{GameOver, VictoryKind};
            use super::score::all_scores;

            let civ_ids: Vec<CivId> = state.civilizations.iter().map(|c| c.id).collect();

            // Check ImmediateWin conditions every turn; first match wins.
            'immediate: for vc_idx in 0..state.victory_conditions.len() {
                if matches!(state.victory_conditions[vc_idx].kind(), VictoryKind::ImmediateWin) {
                    for &civ_id in &civ_ids {
                        let progress = state.victory_conditions[vc_idx].check_progress(civ_id, state);
                        if progress.is_won() {
                            let name = state.victory_conditions[vc_idx].name();
                            state.game_over = Some(GameOver { winner: civ_id, condition: name, turn: state.turn });
                            diff.push(StateDelta::VictoryAchieved { civ: civ_id, condition: name });
                            break 'immediate;
                        }
                    }
                }
            }

            // Check TurnLimit conditions when the turn limit is reached.
            // Compare `state.turn + 1` because Phase 5b runs before the turn counter
            // is incremented; `turn + 1` is the turn that will be completed.
            if state.game_over.is_none() {
                for vc_idx in 0..state.victory_conditions.len() {
                    if let VictoryKind::TurnLimit { turn_limit } = state.victory_conditions[vc_idx].kind()
                        && state.turn + 1 >= turn_limit
                    {
                        let name = state.victory_conditions[vc_idx].name();
                        let completed_turn = state.turn + 1;
                        if let Some((winner, _)) = all_scores(state).into_iter().next() {
                            state.game_over = Some(GameOver { winner, condition: name, turn: completed_turn });
                            diff.push(StateDelta::VictoryAchieved { civ: winner, condition: name });
                        }
                        break;
                    }
                }
            }
        }

        // ── Advance turn counter ──────────────────────────────────────────────
        let prev = state.turn;
        state.turn += 1;
        diff.push(StateDelta::TurnAdvanced { from: prev, to: state.turn });

        diff
    }

    fn assign_citizen(
        &self,
        state: &mut GameState,
        city_id: CityId,
        tile: HexCoord,
        lock: bool,
    ) -> Result<GameStateDiff, RulesError> {
        let city_idx = state.cities.iter().position(|c| c.id == city_id)
            .ok_or(RulesError::CityNotFound)?;

        // Normalize the coord through the board; reject if off-map.
        let tile = state.board.normalize(tile).ok_or(RulesError::InvalidCoord)?;

        // Verify the tile exists on the board.
        if state.board.tile(tile).is_none() {
            return Err(RulesError::InvalidCoord);
        }

        // Reject tiles more than 3 hexes from the city center.
        if state.cities[city_idx].coord.distance(&tile) > 3 {
            return Err(RulesError::InvalidCoord);
        }

        let mut diff = GameStateDiff::new();
        let city = &mut state.cities[city_idx];

        if !city.worked_tiles.contains(&tile) {
            city.worked_tiles.push(tile);
            diff.push(StateDelta::CitizenAssigned { city: city_id, tile });
        }
        if lock {
            city.locked_tiles.insert(tile);
        }

        Ok(diff)
    }

    fn assign_policy(
        &self,
        state: &mut GameState,
        civ_id: CivId,
        policy_id: PolicyId,
    ) -> Result<GameStateDiff, RulesError> {
        // Collect needed data before borrowing state mutably.
        let civ_idx = state.civilizations.iter().position(|c| c.id == civ_id)
            .ok_or(RulesError::CivNotFound)?;

        let policy = state.policies.iter().find(|p| p.id == policy_id)
            .cloned()
            .ok_or(RulesError::PolicyNotFound)?;

        let civ = &state.civilizations[civ_idx];

        // Policy must be in the civ's unlocked list.
        if !civ.unlocked_policies.contains(&policy.name) {
            return Err(RulesError::PolicyNotUnlocked);
        }

        // A government must be active.
        let gov_id = civ.current_government.ok_or(RulesError::NoGovernment)?;
        let gov = state.governments.iter().find(|g| g.id == gov_id)
            .cloned()
            .ok_or(RulesError::NoGovernment)?;

        // Count used slots by type among currently active policies.
        let active = civ.active_policies.clone();
        let (used_mil, used_eco, used_dip, used_wc) = active.iter().fold(
            (0u8, 0u8, 0u8, 0u8),
            |(m, e, d, w), pid| {
                match state.policies.iter().find(|p| p.id == *pid).map(|p| p.policy_type) {
                    Some(PolicyType::Military)   => (m + 1, e, d, w),
                    Some(PolicyType::Economic)   => (m, e + 1, d, w),
                    Some(PolicyType::Diplomatic) => (m, e, d + 1, w),
                    Some(PolicyType::Wildcard)   => (m, e, d, w + 1),
                    None => (m, e, d, w),
                }
            },
        );

        let has_slot = match policy.policy_type {
            PolicyType::Military   => used_mil  < gov.slots.military,
            PolicyType::Economic   => used_eco  < gov.slots.economic,
            PolicyType::Diplomatic => used_dip  < gov.slots.diplomatic,
            PolicyType::Wildcard   => used_wc   < gov.slots.wildcard,
        };
        if !has_slot {
            return Err(RulesError::InsufficientPolicySlots);
        }

        // Maintenance check: gold must cover the cost (we check current gold, not per-turn).
        let civ = &state.civilizations[civ_idx];
        if civ.gold < policy.maintenance as i32 {
            return Err(RulesError::InsufficientGold);
        }

        // Apply: add policy to active list and deduct maintenance.
        state.civilizations[civ_idx].active_policies.push(policy_id);
        state.civilizations[civ_idx].gold -= policy.maintenance as i32;

        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::PolicyAssigned { civ: civ_id, policy: policy_id });
        Ok(diff)
    }

    fn declare_war(
        &self,
        state: &mut GameState,
        aggressor: CivId,
        target: CivId,
    ) -> Result<GameStateDiff, RulesError> {
        if aggressor == target { return Err(RulesError::SameCivilization); }
        if state.civ(aggressor).is_none() { return Err(RulesError::CivNotFound); }
        if state.civ(target).is_none()    { return Err(RulesError::CivNotFound); }

        let rel_idx = find_or_create_relation(state, aggressor, target);

        if state.diplomatic_relations[rel_idx].status == DiplomaticStatus::War {
            return Err(RulesError::AlreadyAtWar);
        }

        let grievance_id = state.id_gen.next_grievance_id();
        let trigger = DeclaredWarGrievance;
        let record = GrievanceRecord {
            grievance_id,
            description: trigger.description(),
            amount: trigger.grievance_amount(),
            visibility: trigger.visibility(),
            recorded_turn: state.turn,
        };
        // The target records a grievance against the aggressor.
        state.diplomatic_relations[rel_idx].add_grievance(target, record);
        state.diplomatic_relations[rel_idx].status = DiplomaticStatus::War;

        let (civ_a, civ_b) = (
            state.diplomatic_relations[rel_idx].civ_a,
            state.diplomatic_relations[rel_idx].civ_b,
        );
        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::DiplomacyChanged { civ_a, civ_b, new_status: DiplomaticStatus::War });
        Ok(diff)
    }

    fn make_peace(
        &self,
        state: &mut GameState,
        civ_a: CivId,
        civ_b: CivId,
    ) -> Result<GameStateDiff, RulesError> {
        if civ_a == civ_b { return Err(RulesError::SameCivilization); }

        let rel_idx = state.diplomatic_relations.iter().position(|r| {
            (r.civ_a == civ_a && r.civ_b == civ_b) ||
            (r.civ_a == civ_b && r.civ_b == civ_a)
        }).ok_or(RulesError::RelationNotFound)?;

        if state.diplomatic_relations[rel_idx].status != DiplomaticStatus::War {
            return Err(RulesError::NotAtWar);
        }

        let rel = &mut state.diplomatic_relations[rel_idx];
        rel.turns_at_war = 0;
        // Compute post-peace status from opinion score (War threshold skipped).
        let score = (rel.opinion_score_a_toward_b() + rel.opinion_score_b_toward_a()) / 2;
        let new_status = status_from_score(score, &rel.active_agreements);
        rel.status = new_status;

        let (a, b) = (rel.civ_a, rel.civ_b);
        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::DiplomacyChanged { civ_a: a, civ_b: b, new_status });
        Ok(diff)
    }

    fn attack(
        &self,
        state:       &mut GameState,
        attacker_id: UnitId,
        defender_id: UnitId,
    ) -> Result<GameStateDiff, RulesError> {
        // --- validation -------------------------------------------------------
        let (atk_coord, atk_range, atk_cs, atk_owner, atk_unit_type) = {
            let u = state.unit(attacker_id).ok_or(RulesError::UnitNotFound)?;
            (u.coord, u.range, u.combat_strength, u.owner, u.unit_type)
        };
        let atk_cs = atk_cs.ok_or(RulesError::UnitCannotAttack)?;

        let (def_coord, def_cs, _def_owner) = {
            let u = state.unit(defender_id).ok_or(RulesError::UnitNotFound)?;
            (u.coord, u.combat_strength.unwrap_or(0), u.owner)
        };

        if atk_owner == state.unit(defender_id).unwrap().owner {
            return Err(RulesError::SameCivilization);
        }

        if state.unit(attacker_id).unwrap().movement_left == 0 {
            return Err(RulesError::InsufficientMovement(GameStateDiff::new()));
        }

        let dist = atk_coord.distance(&def_coord);
        if atk_range == 0 {
            if dist != 1 { return Err(RulesError::NotInRange); }
        } else if dist > atk_range as u32 {
            return Err(RulesError::NotInRange);
        }

        // --- damage calculation -----------------------------------------------
        // Terrain defense bonus applies to the defender's tile.
        let terrain_def_bonus = state.board
            .tile(def_coord)
            .map(|t| t.terrain_defense_bonus())
            .unwrap_or(0);
        // Wall defense bonus: if the defender is on a city tile with walls,
        // the wall's defense_bonus is added to effective combat strength.
        let wall_def_bonus = state.cities.iter()
            .find(|c| c.coord == def_coord)
            .map(|c| c.walls.defense_bonus())
            .unwrap_or(0);
        let effective_def_cs = (def_cs as i32 + terrain_def_bonus + wall_def_bonus).max(1) as u32;

        // Siege bonus: extra attack strength when attacking a unit on a city tile.
        let is_city_tile = state.cities.iter().any(|c| c.coord == def_coord);
        let siege_bonus = if is_city_tile {
            state.unit_type_defs.iter()
                .find(|d| d.id == atk_unit_type)
                .map(|d| d.siege_bonus)
                .unwrap_or(0)
        } else {
            0
        };
        let effective_atk_cs = atk_cs + siege_bonus;

        // Formula: 30 * exp((cs_atk - cs_def_effective) / 25) * rng[0.75, 1.25]
        let rng_a = 0.75 + state.id_gen.next_f32() * 0.5;
        let def_damage = (30.0_f32
            * f32::exp((effective_atk_cs as f32 - effective_def_cs as f32) / 25.0)
            * rng_a) as u32;

        let (attack_type, atk_damage) = if atk_range == 0 {
            let rng_b = 0.75 + state.id_gen.next_f32() * 0.5;
            let d = (30.0_f32
                * f32::exp((def_cs as f32 - atk_cs as f32) / 25.0)
                * rng_b) as u32;
            (AttackType::Melee, d)
        } else {
            (AttackType::Ranged, 0u32)
        };

        // --- mutate state and build diff --------------------------------------
        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::UnitAttacked {
            attacker:        attacker_id,
            defender:        defender_id,
            attack_type,
            attacker_damage: atk_damage,
            defender_damage: def_damage,
        });

        if let Some(u) = state.unit_mut(defender_id) {
            u.health = u.health.saturating_sub(def_damage);
            if u.health == 0 {
                diff.push(StateDelta::UnitDestroyed { unit: defender_id });
            }
        }
        if atk_damage > 0
            && let Some(u) = state.unit_mut(attacker_id)
        {
            u.health = u.health.saturating_sub(atk_damage);
            if u.health == 0 {
                diff.push(StateDelta::UnitDestroyed { unit: attacker_id });
            }
        }
        state.units.retain(|u| u.health > 0);

        // --- wall damage on melee attacks against city tiles ─────────────────
        if attack_type == AttackType::Melee
            && let Some(city_idx) = state.cities.iter().position(|c| c.coord == def_coord)
        {
            let city = &state.cities[city_idx];
            if city.walls != crate::civ::city::WallLevel::None && city.wall_hp > 0 {
                let wall_damage = def_damage / 2;
                if wall_damage > 0 {
                    let city = &mut state.cities[city_idx];
                    city.wall_hp = city.wall_hp.saturating_sub(wall_damage);
                    let hp_remaining = city.wall_hp;
                    diff.push(StateDelta::WallDamaged {
                        city: city.id,
                        damage: wall_damage,
                        hp_remaining,
                    });
                    if hp_remaining == 0 {
                        let previous_level = city.walls;
                        city.walls = crate::civ::city::WallLevel::None;
                        diff.push(StateDelta::WallDestroyed {
                            city: city.id,
                            previous_level,
                        });
                    }
                }
            }
        }

        // --- city capture on melee kill ───────────────────────────────────────
        // When a melee attacker kills a unit on an enemy city tile the city is
        // captured immediately.  Any other old-owner units still on the tile
        // (garrisoned units that cannot escape) are also destroyed.
        // If the attacked unit survived (no kill) no capture occurs.
        let defender_was_killed = !state.units.iter().any(|u| u.id() == defender_id);
        if attack_type == AttackType::Melee
            && defender_was_killed
            && let Some(city_idx) = state.cities.iter().position(|c| c.coord == def_coord)
        {
            let old_owner = state.cities[city_idx].owner;
            if old_owner != atk_owner {
                    // Destroy any old-owner units still garrisoned on the tile.
                    let garrison: Vec<UnitId> = state.units.iter()
                        .filter(|u| u.owner == old_owner && u.coord == def_coord)
                        .map(|u| u.id())
                        .collect();
                    for uid in &garrison {
                        diff.push(StateDelta::UnitDestroyed { unit: *uid });
                    }
                    state.units.retain(|u| !(u.owner == old_owner && u.coord == def_coord));

                    // Transfer city ownership.
                    let city = &mut state.cities[city_idx];
                    city.owner     = atk_owner;
                    city.ownership = crate::civ::city::CityOwnership::Occupied;
                    let city_id    = city.id;
                    diff.push(StateDelta::CityCaptured {
                        city:      city_id,
                        new_owner: atk_owner,
                        old_owner,
                    });

                    // Update civilization city lists.
                    if let Some(old_civ) = state.civilizations.iter_mut()
                        .find(|c| c.id == old_owner)
                    {
                        old_civ.cities.retain(|&id| id != city_id);
                    }
                    if let Some(new_civ) = state.civilizations.iter_mut()
                        .find(|c| c.id == atk_owner)
                    {
                        new_civ.cities.push(city_id);
                    }
                }
        }

        if let Some(u) = state.unit_mut(attacker_id) {
            u.movement_left = 0;
        }

        Ok(diff)
    }

    fn found_city(
        &self,
        state:   &mut GameState,
        settler: UnitId,
        name:    String,
    ) -> Result<GameStateDiff, RulesError> {
        let (coord, civ_id, unit_type_id) = {
            let u = state.unit(settler).ok_or(RulesError::UnitNotFound)?;
            (u.coord, u.owner, u.unit_type)
        };

        let is_settler = state.unit_type_defs.iter()
            .any(|d| d.id == unit_type_id && d.can_found_city);
        if !is_settler { return Err(RulesError::NotASettler); }

        let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;
        if !tile.terrain.is_land() {
            return Err(RulesError::InvalidFoundingTerrain);
        }
        if tile.terrain.movement_cost() == MovementCost::Impassable {
            return Err(RulesError::InvalidFoundingTerrain);
        }

        if state.cities.iter().any(|c| c.coord == coord) {
            return Err(RulesError::TileOccupied);
        }
        // Note: raw cube-coord distance; may undercount on cylindrical maps.
        if state.cities.iter().any(|c| c.coord.distance(&coord) <= 3) {
            return Err(RulesError::TooCloseToCity);
        }

        let city_id = state.id_gen.next_city_id();
        let is_capital = state.civilizations.iter()
            .find(|c| c.id == civ_id)
            .is_none_or(|c| c.cities.is_empty());
        let mut city = crate::civ::City::new(city_id, name, civ_id, coord);
        city.is_capital = is_capital;

        if let Some(civ) = state.civilizations.iter_mut().find(|c| c.id == civ_id) {
            civ.cities.push(city_id);
        }
        state.cities.push(city);
        state.units.retain(|u| u.id != settler);

        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::UnitDestroyed { unit: settler });
        diff.push(StateDelta::CityFounded { city: city_id, coord, owner: civ_id });

        try_claim_tile(state, civ_id, city_id, coord, &mut diff);
        for nb in state.board.neighbors(coord) {
            try_claim_tile(state, civ_id, city_id, nb, &mut diff);
        }

        Ok(diff)
    }

    fn claim_tile(
        &self,
        state: &mut GameState,
        city_id: CityId,
        coord: HexCoord,
        force: bool,
    ) -> Result<GameStateDiff, RulesError> {
        let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

        let (city_coord, civ_id) = state.cities.iter()
            .find(|c| c.id == city_id)
            .map(|c| (c.coord, c.owner))
            .ok_or(RulesError::CityNotFound)?;

        let dist = city_coord.distance(&coord);
        if !(1..=3).contains(&dist) {
            return Err(RulesError::TileNotInCityRange);
        }

        let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;

        match tile.owner {
            Some(owner) if owner == civ_id => {
                // Already owned by this civ — idempotent.
                return Ok(GameStateDiff::new());
            }
            Some(_) if !force => return Err(RulesError::TileOwnedByEnemy),
            Some(_) | None => {}  // unclaimed, or enemy tile with force=true (culture flip)
        }

        if let Some(t) = state.board.tile_mut(coord) {
            t.owner = Some(civ_id);
        }
        if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
            city.territory.insert(coord);
        }
        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::TileClaimed { civ: civ_id, city: city_id, coord });
        Ok(diff)
    }

    fn reassign_tile(
        &self,
        state: &mut GameState,
        from_city: CityId,
        to_city: CityId,
        coord: HexCoord,
    ) -> Result<GameStateDiff, RulesError> {
        let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

        let from_civ = state.cities.iter()
            .find(|c| c.id == from_city)
            .map(|c| c.owner)
            .ok_or(RulesError::CityNotFound)?;

        let (to_coord, to_civ) = state.cities.iter()
            .find(|c| c.id == to_city)
            .map(|c| (c.coord, c.owner))
            .ok_or(RulesError::CityNotFound)?;

        if from_civ != to_civ {
            return Err(RulesError::CitiesNotSameCiv);
        }
        let civ_id = from_civ;

        if from_city == to_city {
            return Ok(GameStateDiff::new());
        }

        let owner = state.board.tile(coord)
            .ok_or(RulesError::InvalidCoord)?
            .owner;
        if owner != Some(civ_id) {
            return Err(RulesError::TileNotOwned);
        }

        let to_dist = to_coord.distance(&coord);
        if !(1..=3).contains(&to_dist) {
            return Err(RulesError::TileNotInCityRange);
        }

        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::TileReassigned { civ: civ_id, from_city, to_city, coord });
        Ok(diff)
    }

    fn place_improvement(
        &self,
        state: &mut GameState,
        civ_id: CivId,
        coord: HexCoord,
        improvement: crate::world::improvement::BuiltinImprovement,
    ) -> Result<GameStateDiff, RulesError> {
        use libhexgrid::HexTile;
        use crate::world::improvement::{ElevationReq, ProximityReq};
        use libhexgrid::types::Elevation;

        let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

        let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;
        let req  = improvement.requirements(&state.tech_refs, &state.civic_refs);

        // 3. tile must be owned by this civilization
        if tile.owner != Some(civ_id) {
            return Err(RulesError::TileNotOwned);
        }

        // 4. requires_land / requires_water
        if req.requires_land && !tile.terrain.is_land() {
            return Err(RulesError::InvalidImprovement);
        }
        if req.requires_water && tile.terrain.is_land() {
            return Err(RulesError::InvalidImprovement);
        }

        // 5. elevation
        let elev = tile.elevation();
        let elev_ok = match req.elevation {
            ElevationReq::Any         => true,
            ElevationReq::Flat        => elev < Elevation::HILLS,
            ElevationReq::HillsOrMore => elev >= Elevation::HILLS && elev != Elevation::High,
            ElevationReq::NotMountain => elev != Elevation::High,
        };
        if !elev_ok {
            return Err(RulesError::InvalidImprovement);
        }

        // 6. blocked_terrains
        if req.blocked_terrains.contains(&tile.terrain) {
            return Err(RulesError::InvalidImprovement);
        }

        // 7. required_feature
        if let Some(req_feat) = req.required_feature
            && tile.feature != Some(req_feat)
        {
            return Err(RulesError::InvalidImprovement);
        }

        // 8. conditional_features: on matching terrain, one of listed features must be present
        let terrain = tile.terrain;
        let feature = tile.feature;
        for &(cond_terrain, allowed_features) in req.conditional_features {
            if terrain == cond_terrain {
                let ok = feature.is_some_and(|f| allowed_features.contains(&f));
                if !ok {
                    return Err(RulesError::InvalidImprovement);
                }
            }
        }

        // 9. required_resource
        if let Some(req_res) = req.required_resource
            && tile.resource != Some(req_res)
        {
            return Err(RulesError::ResourceRequired);
        }

        // 10. proximity
        if let Some(prox) = req.proximity {
            let ok = state.board.neighbors(coord).iter().any(|&nb| {
                state.board.tile(nb).is_some_and(|t| match prox {
                    ProximityReq::AdjacentTerrain(tt) => t.terrain == tt,
                    ProximityReq::AdjacentFeature(f)  => t.feature == Some(f),
                    ProximityReq::AdjacentResource(r) => t.resource == Some(r),
                })
            });
            if !ok {
                return Err(RulesError::ProximityRequired);
            }
        }

        // 11. get civ
        let civ = state.civilizations.iter()
            .find(|c| c.id == civ_id)
            .ok_or(RulesError::CivNotFound)?;

        // 12. required_tech
        if let Some(tech_id) = req.required_tech
            && !civ.researched_techs.contains(&tech_id)
        {
            return Err(RulesError::TechRequired);
        }

        // 13. required_civic
        if let Some(civic_id) = req.required_civic
            && !civ.completed_civics.contains(&civic_id)
        {
            return Err(RulesError::CivicRequired);
        }

        // 14. apply
        if let Some(tile) = state.board.tile_mut(coord) {
            tile.improvement = Some(improvement);
            tile.improvement_pillaged = false;
        }

        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::ImprovementPlaced { coord, improvement });
        Ok(diff)
    }

    fn place_district(
        &self,
        state: &mut GameState,
        city_id: CityId,
        district: crate::civ::district::BuiltinDistrict,
        coord: HexCoord,
    ) -> Result<GameStateDiff, RulesError> {
        let coord = state.board.normalize(coord).ok_or(RulesError::InvalidCoord)?;

        // 1. City must exist and we grab its coord + owner.
        let (city_coord, civ_id) = state.cities.iter()
            .find(|c| c.id == city_id)
            .map(|c| (c.coord, c.owner))
            .ok_or(RulesError::CityNotFound)?;

        // 2. Coord must be within 1–3 tiles from city center (not the city center itself).
        let dist = city_coord.distance(&coord);
        if !(1..=3).contains(&dist) {
            return Err(RulesError::TileNotInCityRange);
        }

        // 3. Tile must exist and be owned by this civ.
        let tile = state.board.tile(coord).ok_or(RulesError::InvalidCoord)?;
        if tile.owner != Some(civ_id) {
            return Err(RulesError::TileNotOwned);
        }

        // 4. Tile must not already host a district.
        if state.placed_districts.iter().any(|d| d.coord == coord) {
            return Err(RulesError::TileOccupiedByDistrict);
        }

        // 5. Tile must not be a city center.
        if state.cities.iter().any(|c| c.coord == coord) {
            return Err(RulesError::TileOccupied);
        }

        // 6. City must not already have this district type.
        let already_has = state.cities.iter()
            .find(|c| c.id == city_id)
            .is_some_and(|c| c.districts.contains(&district));
        if already_has {
            return Err(RulesError::DistrictAlreadyPresent);
        }

        let req = district.requirements(&state.tech_refs, &state.civic_refs);

        // 7. requires_land / requires_water
        if req.requires_land && !tile.terrain.is_land() {
            return Err(RulesError::InvalidDistrict);
        }
        if req.requires_water && tile.terrain.is_land() {
            return Err(RulesError::InvalidDistrict);
        }

        // 8. forbidden_terrains
        if req.forbidden_terrains.contains(&tile.terrain) {
            return Err(RulesError::InvalidDistrict);
        }

        // 9. required_tech
        if let Some(tech_id) = req.required_tech {
            let civ = state.civilizations.iter()
                .find(|c| c.id == civ_id)
                .ok_or(RulesError::CivNotFound)?;
            if !civ.researched_techs.contains(&tech_id) {
                return Err(RulesError::TechRequired);
            }
        }

        // 10. required_civic
        if let Some(civic_id) = req.required_civic {
            let civ = state.civilizations.iter()
                .find(|c| c.id == civ_id)
                .ok_or(RulesError::CivNotFound)?;
            if !civ.completed_civics.contains(&civic_id) {
                return Err(RulesError::CivicRequired);
            }
        }

        // 11. Apply: record the placed district and update the city's district list.
        state.placed_districts.push(crate::civ::district::PlacedDistrict::new(
            district, city_id, coord,
        ));
        if let Some(city) = state.cities.iter_mut().find(|c| c.id == city_id) {
            city.districts.push(district);
        }

        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::DistrictBuilt { city: city_id, district, coord });
        Ok(diff)
    }

    fn establish_trade_route(
        &self,
        state: &mut GameState,
        trader_unit: UnitId,
        destination: CityId,
    ) -> Result<GameStateDiff, RulesError> {
        use crate::UnitCategory;
        use crate::civ::trade::compute_route_yields;

        // 1. Unit must exist.
        let (unit_owner, unit_coord, unit_category) = state.units.iter()
            .find(|u| u.id() == trader_unit)
            .map(|u| (u.owner(), u.coord(), u.category()))
            .ok_or(RulesError::UnitNotFound)?;

        // 2. Must be a trader.
        if unit_category != UnitCategory::Trader {
            return Err(RulesError::NotATrader);
        }

        // 3. Trader must be at a city owned by its civ (origin city).
        let origin_id = state.cities.iter()
            .find(|c| c.owner == unit_owner && c.coord == unit_coord)
            .map(|c| c.id)
            .ok_or(RulesError::NoOriginCity)?;

        // 4. Destination city must exist.
        if !state.cities.iter().any(|c| c.id == destination) {
            return Err(RulesError::CityNotFound);
        }

        // 5. Origin and destination must differ.
        if origin_id == destination {
            return Err(RulesError::SameCity);
        }

        // TODO(PHASE3-8.4-CAPACITY): check max trade routes per civ.

        // Determine international status and compute yields.
        let international = {
            let origin_owner = state.cities.iter().find(|c| c.id == origin_id).map(|c| c.owner);
            let dest_owner   = state.cities.iter().find(|c| c.id == destination).map(|c| c.owner);
            matches!((origin_owner, dest_owner), (Some(a), Some(b)) if a != b)
        };
        let (origin_yields, dest_yields) = compute_route_yields(international);

        let route_id = state.id_gen.next_trade_route_id();
        let mut route = crate::civ::TradeRoute::new(route_id, origin_id, destination, unit_owner);
        route.origin_yields      = origin_yields;
        route.destination_yields = dest_yields;
        route.turns_remaining    = Some(30);

        // Consume the trader unit.
        state.units.retain(|u| u.id() != trader_unit);
        state.trade_routes.push(route);

        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::UnitDestroyed { unit: trader_unit });
        diff.push(StateDelta::TradeRouteEstablished {
            route: route_id,
            origin: origin_id,
            destination,
            owner: unit_owner,
        });
        Ok(diff)
    }

    fn city_bombard(
        &self,
        state: &mut GameState,
        city_id: CityId,
        target:  UnitId,
    ) -> Result<GameStateDiff, RulesError> {
        // 1. Validate city exists and has walls.
        let city_idx = state.cities.iter().position(|c| c.id == city_id)
            .ok_or(RulesError::CityNotFound)?;
        let city = &state.cities[city_idx];
        if city.walls == crate::civ::city::WallLevel::None {
            return Err(RulesError::CityCannotAttack);
        }
        if city.has_attacked_this_turn {
            return Err(RulesError::CityAlreadyAttacked);
        }
        let city_coord = city.coord;
        let city_owner = city.owner;

        // City ranged strength = 15 + wall defense bonus.
        let city_cs = 15_u32 + city.walls.defense_bonus() as u32;

        // 2. Validate target unit exists, is an enemy, and is within range 2.
        let (def_coord, def_cs) = {
            let u = state.unit(target).ok_or(RulesError::UnitNotFound)?;
            if u.owner == city_owner {
                return Err(RulesError::SameCivilization);
            }
            (u.coord, u.combat_strength.unwrap_or(0))
        };
        let dist = city_coord.distance(&def_coord);
        if dist > 2 || dist == 0 {
            return Err(RulesError::NotInRange);
        }

        // 3. Damage formula (same exponential; no terrain bonus for city offense).
        let rng = 0.75 + state.id_gen.next_f32() * 0.5;
        let damage = (30.0_f32
            * f32::exp((city_cs as f32 - def_cs as f32) / 25.0)
            * rng) as u32;

        // 4. Apply damage to target; no counter-damage to city.
        let mut diff = GameStateDiff::new();
        diff.push(StateDelta::UnitAttacked {
            attacker:        UnitId::nil(),
            defender:        target,
            attack_type:     AttackType::CityBombard,
            attacker_damage: 0,
            defender_damage: damage,
        });
        if let Some(u) = state.unit_mut(target) {
            u.health = u.health.saturating_sub(damage);
            if u.health == 0 {
                diff.push(StateDelta::UnitDestroyed { unit: target });
            }
        }
        state.units.retain(|u| u.health > 0);

        // 5. Mark city as having attacked this turn.
        state.cities[city_idx].has_attacked_this_turn = true;

        Ok(diff)
    }

    fn retire_great_person(
        &self,
        state: &mut GameState,
        great_person_id: crate::GreatPersonId,
    ) -> Result<GameStateDiff, RulesError> {
        todo!("retire_great_person: implement great person retirement logic")
    }
}

// ── apply_effect ──────────────────────────────────────────────────────────────

/// Apply a single `OneShotEffect` to state, recording the resulting `StateDelta`.
///
/// Returns `()` — structurally cannot enqueue more effects, preventing cascades.
/// The caller is responsible for checking `effect.guard(civ)` before calling this.
fn apply_effect(
    state: &mut GameState,
    civ_id: CivId,
    effect: &OneShotEffect,
    diff: &mut GameStateDiff,
) {
    let civ_idx = match state.civilizations.iter().position(|c| c.id == civ_id) {
        Some(i) => i,
        None => return,
    };

    match effect {
        OneShotEffect::RevealResource(r) => {
            state.civilizations[civ_idx].revealed_resources.insert(*r);
            diff.push(StateDelta::ResourceRevealed { civ: civ_id, resource: *r });
        }

        OneShotEffect::UnlockUnit(u) => {
            state.civilizations[civ_idx].unlocked_units.push(u);
            diff.push(StateDelta::UnitUnlocked { civ: civ_id, unit_type: u });
        }

        OneShotEffect::UnlockBuilding(b) => {
            state.civilizations[civ_idx].unlocked_buildings.push(b);
            diff.push(StateDelta::BuildingUnlocked { civ: civ_id, building: b });
        }

        OneShotEffect::UnlockImprovement(i) => {
            state.civilizations[civ_idx].unlocked_improvements.push(i);
            diff.push(StateDelta::ImprovementUnlocked { civ: civ_id, improvement: i });
        }

        OneShotEffect::TriggerEureka { tech } => {
            // Check if the front of the research queue matches this tech.
            let in_progress_id = state.civilizations[civ_idx]
                .research_queue.front().map(|tp| tp.tech_id);
            let matches_current = in_progress_id.map(|id| id == *tech).unwrap_or(false);

            state.civilizations[civ_idx].eureka_triggered.insert(*tech);
            if matches_current
                && let Some(tp) = state.civilizations[civ_idx].research_queue.front_mut()
            {
                tp.boosted = true;
            }
            let tech_name = state.tech_tree.get(*tech).map(|n| n.name).unwrap_or("?");
            diff.push(StateDelta::EurekaTriggered { civ: civ_id, tech: tech_name });
        }

        OneShotEffect::TriggerInspiration { civic } => {
            // Check if the current civic in progress matches this civic.
            let in_progress_id = state.civilizations[civ_idx]
                .civic_in_progress.as_ref().map(|cp| cp.civic_id);
            let matches_current = in_progress_id.map(|id| id == *civic).unwrap_or(false);

            state.civilizations[civ_idx].inspiration_triggered.insert(*civic);
            if matches_current
                && let Some(cp) = state.civilizations[civ_idx].civic_in_progress.as_mut()
            {
                cp.inspired = true;
            }
            let civic_name = state.civic_tree.get(*civic).map(|n| n.name).unwrap_or("?");
            diff.push(StateDelta::InspirationTriggered { civ: civ_id, civic: civic_name });
        }

        OneShotEffect::FreeUnit { unit_type, city: hint_city } => {
            // Resolve spawn coord: hint city > capital > first owned city > origin.
            let coord = hint_city
                .and_then(|cid| state.cities.iter().find(|c| c.id == cid))
                .or_else(|| state.cities.iter().find(|c| c.owner == civ_id && c.is_capital))
                .or_else(|| state.cities.iter().find(|c| c.owner == civ_id))
                .map(|c| c.coord)
                .unwrap_or(HexCoord::from_qr(0, 0));

            if let Some(def) = state.unit_type_defs.iter().find(|d| d.name == *unit_type).cloned() {
                // Registry present: spawn a real unit.
                let unit_id   = state.id_gen.next_unit_id();
                let type_id   = UnitTypeId::from_ulid(state.id_gen.next_ulid());
                state.units.push(BasicUnit {
                    id:              unit_id,
                    unit_type:       type_id,
                    owner:           civ_id,
                    coord,
                    domain:          def.domain,
                    category:        def.category,
                    movement_left:   def.max_movement,
                    max_movement:    def.max_movement,
                    combat_strength: def.combat_strength,
                    promotions:      Vec::new(),
                    health:          100,
                    range:           0,
                    vision_range:    2,
                });
                diff.push(StateDelta::UnitCreated { unit: unit_id, coord, owner: civ_id });
            } else {
                // No registry entry: emit placeholder delta for external handling.
                diff.push(StateDelta::FreeUnitGranted { civ: civ_id, unit_type, coord });
            }
        }

        OneShotEffect::FreeBuilding { building, city } => {
            // Resolve target city: hint > capital > first owned city.
            let target_city_id = city
                .and_then(|cid| state.cities.iter().find(|c| c.id == cid))
                .or_else(|| state.cities.iter().find(|c| c.owner == civ_id && c.is_capital))
                .or_else(|| state.cities.iter().find(|c| c.owner == civ_id))
                .map(|c| c.id);

            if let Some(target_cid) = target_city_id {
                if let Some(def) = state.building_defs.iter().find(|d| d.name == *building).cloned() {
                    // Registry present: add the building instance to the city.
                    let building_instance_id = state.id_gen.next_building_id();
                    if let Some(city_mut) = state.cities.iter_mut().find(|c| c.id == target_cid) {
                        city_mut.buildings.push(building_instance_id);
                    }
                    let _ = def; // yields/maintenance tracked via BuildingDef lookup, not stored per instance
                    diff.push(StateDelta::FreeBuildingGranted { civ: civ_id, building, city: target_cid });
                } else {
                    // No registry entry: emit placeholder delta.
                    diff.push(StateDelta::FreeBuildingGranted { civ: civ_id, building, city: target_cid });
                }
            }
        }

        OneShotEffect::UnlockGovernment(g) => {
            state.civilizations[civ_idx].unlocked_governments.push(g);
            diff.push(StateDelta::GovernmentUnlocked { civ: civ_id, government: g });
        }

        OneShotEffect::AdoptGovernment(g) => {
            // Look up the new government in the registry by name.
            let new_gov = state.governments.iter().find(|gov| gov.name == *g).cloned();

            if let Some(new_gov) = new_gov {
                // Count free slots in the new government.
                let mut mil_free  = new_gov.slots.military as i32;
                let mut eco_free  = new_gov.slots.economic as i32;
                let mut dip_free  = new_gov.slots.diplomatic as i32;
                let mut wc_free   = new_gov.slots.wildcard as i32;

                // Determine which active policies still fit; collect those to remove.
                let active: Vec<PolicyId> = state.civilizations[civ_idx].active_policies.clone();
                let mut kept    = Vec::new();
                let mut removed = Vec::new();

                for pid in active {
                    let policy_type = state.policies.iter()
                        .find(|p| p.id == pid)
                        .map(|p| p.policy_type);
                    let fits = match policy_type {
                        Some(PolicyType::Military)   => { if mil_free  > 0 { mil_free  -= 1; true } else { false } }
                        Some(PolicyType::Economic)   => { if eco_free  > 0 { eco_free  -= 1; true } else { false } }
                        Some(PolicyType::Diplomatic) => { if dip_free  > 0 { dip_free  -= 1; true } else { false } }
                        Some(PolicyType::Wildcard)   => { if wc_free   > 0 { wc_free   -= 1; true } else { false } }
                        None => false,
                    };
                    if fits { kept.push(pid); } else { removed.push(pid); }
                }

                state.civilizations[civ_idx].active_policies = kept;
                state.civilizations[civ_idx].current_government = Some(new_gov.id);
                state.civilizations[civ_idx].current_government_name = Some(g);

                for pid in removed {
                    diff.push(StateDelta::PolicyUnslotted { civ: civ_id, policy: pid });
                }
            } else {
                // Government not in registry; set name only (best-effort).
                state.civilizations[civ_idx].current_government_name = Some(g);
            }

            diff.push(StateDelta::GovernmentAdopted { civ: civ_id, government: g });
        }

        OneShotEffect::UnlockPolicy(p) => {
            state.civilizations[civ_idx].unlocked_policies.push(p);
            diff.push(StateDelta::PolicyUnlocked { civ: civ_id, policy: p });
        }

        OneShotEffect::GrantModifier(_) => {
            // No stored mutation: modifier is collected at query time via
            // `Civilization::get_tree_modifiers`. Nothing to do here.
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the `HexDir` from `from` to an adjacent `to`, handling board wrapping.
fn neighbor_dir(board: &WorldBoard, from: HexCoord, to: HexCoord) -> Option<HexDir> {
    HexDir::ALL.iter().find(|&&dir| {
        board.normalize(from + dir.unit_vec()) == Some(to)
    }).copied()
}

/// Apply a resolved set of `EffectType`s to a `YieldBundle`.
/// Flat bonuses are applied first, then percentage bonuses.
fn apply_effects(effects: &[EffectType], mut base: YieldBundle) -> YieldBundle {
    for &effect in effects {
        if let EffectType::YieldFlat(yt, amount) = effect {
            base.add_yield(yt, amount);
        }
    }
    for &effect in effects {
        if let EffectType::YieldPercent(yt, pct) = effect {
            let current = base.get(yt);
            let bonus   = (current * pct) / 100;
            base.add_yield(yt, bonus);
        }
    }
    base
}

/// Compute tile yields, suppressing the resource component when the civ lacks
/// the required reveal tech (PHASE3-4.2). Improvement yields are also skipped
/// when pillaged, consistent with `WorldTile::total_yields`.
fn tile_yields_gated(tile: &WorldTile, known_techs: &HashSet<&str>) -> YieldBundle {
    let mut yields = tile.terrain.base_yields();

    if let Some(feat) = tile.feature {
        yields += feat.yield_modifier();
    }

    if let Some(impr) = tile.improvement
        && !tile.improvement_pillaged
    {
        yields += impr.yield_bonus();
    }

    if let Some(res) = tile.resource {
        let reveal_tech = res.reveal_tech();
        // Include resource yields only when:
        //   1. No reveal tech is required, or the civ has already researched it.
        //   2. The resource is not concealed by an overlying feature
        //      (Forest/Rainforest hide resources until the feature is cleared).
        let tech_ok = reveal_tech.is_none_or(|t| known_techs.contains(t));
        let concealed = tile.feature
            .is_some_and(|f| f.conceals_resources());
        if tech_ok && !concealed {
            yields += res.base_yields();
        }
    }

    yields
}

/// Assign the highest-yield unworked tile within 3 rings of the city to the
/// city's worked set. Called automatically when a city's population grows.
/// Locked tiles are never displaced; unlocked tiles may be reassigned later.
fn auto_assign_citizen(board: &WorldBoard, city: &mut crate::civ::City) {
    let best = (1u32..=3)
        .flat_map(|r| city.coord.ring(r))
        .filter(|coord| {
            board.tile(*coord).is_some() && !city.worked_tiles.contains(coord)
        })
        .max_by_key(|coord| {
            board.tile(*coord)
                .map(|t| {
                    let y = t.total_yields();
                    y.food + y.production + y.gold + y.science + y.culture
                })
                .unwrap_or(0)
        });

    if let Some(coord) = best {
        city.worked_tiles.push(coord);
    }
}

// ── Diplomacy helpers ─────────────────────────────────────────────────────────

/// Find the index of the `DiplomaticRelation` between two civs in `state`,
/// creating a new `Neutral` relation and appending it if none exists.
fn find_or_create_relation(state: &mut GameState, a: CivId, b: CivId) -> usize {
    if let Some(idx) = state.diplomatic_relations.iter().position(|r| {
        (r.civ_a == a && r.civ_b == b) || (r.civ_a == b && r.civ_b == a)
    }) {
        return idx;
    }
    state.diplomatic_relations.push(DiplomaticRelation::new(a, b));
    state.diplomatic_relations.len() - 1
}

/// Compute the opinion score used for status thresholds: the arithmetic mean
/// of each side's net opinion (both directions averaged).
fn combined_score(rel: &DiplomaticRelation) -> i32 {
    (rel.opinion_score_a_toward_b() + rel.opinion_score_b_toward_a()) / 2
}

/// Map a combined opinion score to a `DiplomaticStatus`.
/// Does **not** apply the War-persistence rule; use `compute_diplomatic_status`
/// for the full transition logic including War persistence.
fn status_from_score(score: i32, active_agreements: &[AgreementId]) -> DiplomaticStatus {
    if score > 50 {
        if active_agreements.is_empty() {
            DiplomaticStatus::Friendly
        } else {
            DiplomaticStatus::Alliance
        }
    } else if score < -20 {
        DiplomaticStatus::Denounced
    } else {
        DiplomaticStatus::Neutral
    }
}

/// Determine the new status for a relation, honouring the War-persistence
/// rule: once at war, the relation stays at War while the combined score
/// remains below -50. All other transitions are driven purely by score.
fn compute_diplomatic_status(rel: &DiplomaticRelation) -> DiplomaticStatus {
    let score = combined_score(rel);
    if rel.status == DiplomaticStatus::War && score < -50 {
        DiplomaticStatus::War
    } else {
        status_from_score(score, &rel.active_agreements)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civ::{BasicUnit, Civilization, City, Leader};
    use crate::civ::civilization::Agenda;
    use libhexgrid::coord::HexCoord;
    use crate::{CivId, UnitCategory, UnitDomain};

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
            siege_bonus: 0,
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
