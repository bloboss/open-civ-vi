use std::collections::{HashSet, VecDeque};
use crate::{BuildingId, CityId, CivId, DistrictTypeId, UnitTypeId, WonderId};
use libhexgrid::coord::HexCoord;
use super::city_state::CityStateData;

/// Whether this city is a regular player city or an independent city-state.
#[derive(Debug)]
pub enum CityKind {
    /// A standard player or AI city.
    Regular,
    /// An independent city-state. Suzerain/influence mechanics live in `CityStateData`.
    CityState(CityStateData),
}

/// Political/ownership state of a city.
///
/// Transient conditions (Starving, LowHousing, UnderSiege) are computed each
/// turn by the rules engine — they are not stored here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CityOwnership {
    /// Owned and fully managed by the current civilization.
    Normal,
    /// Captured; owner manages production queue but suffers loyalty/amenity penalties.
    Occupied,
    /// Captured but not annexed; AI manages production queue on the owner's behalf.
    /// Still generates yields and counts toward empire size. Distinct from `Occupied`
    /// in that the owner does not directly control production choices.
    Puppet,
    /// Being razed; removed from the map when raze_turns reaches zero (Phase 2).
    Razed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WallLevel {
    None,
    Ancient,
    Medieval,
    Renaissance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProductionItem {
    Unit(UnitTypeId),
    Building(BuildingId),
    District(DistrictTypeId),
    Wonder(WonderId),
}

#[derive(Debug)]
pub struct City {
    pub id: CityId,
    pub name: String,
    pub owner: CivId,
    pub founded_by: CivId,
    pub coord: HexCoord,
    pub kind: CityKind,
    pub ownership: CityOwnership,
    pub is_capital: bool,
    pub population: u32,
    pub food_stored: u32,
    pub food_to_grow: u32,
    pub production_stored: u32,
    /// Ordered production queue. `advance_turn` works on `front()` and calls
    /// `pop_front()` when an item completes (requires registry — Part 6.2).
    pub production_queue: VecDeque<ProductionItem>,
    pub walls: WallLevel,
    pub wall_hp: u32,
    pub buildings: Vec<BuildingId>,
    // TODO(PHASE3-8.2): Change to Vec<PlacedDistrict> (from civ/district.rs) to store
    //   coord alongside district type; needed for adjacency bonus computation.
    pub districts: Vec<DistrictTypeId>,
    /// Tiles currently being worked by citizens. Always includes the city center
    /// (set at founding). Citizens are auto-assigned on population growth and can
    /// be overridden via `RulesEngine::assign_citizen`.
    pub worked_tiles: Vec<HexCoord>,
    /// Tiles pinned by player/AI override; survive auto-reassignment.
    pub locked_tiles: HashSet<HexCoord>,
}

impl City {
    pub fn new(id: CityId, name: String, owner: CivId, coord: HexCoord) -> Self {
        Self {
            id,
            name,
            owner,
            founded_by: owner,
            coord,
            kind: CityKind::Regular,
            ownership: CityOwnership::Normal,
            is_capital: false,
            population: 1,
            food_stored: 0,
            food_to_grow: 15,
            production_stored: 0,
            production_queue: VecDeque::new(),
            walls: WallLevel::None,
            wall_hp: WallLevel::None.max_hp(),
            buildings: Vec::new(),
            districts: Vec::new(),
            worked_tiles: vec![coord],
            locked_tiles: HashSet::new(),
        }
    }

    pub fn is_capital(&self) -> bool {
        self.is_capital
    }

    pub fn growth_progress(&self) -> f32 {
        if self.food_to_grow == 0 {
            return 1.0;
        }
        self.food_stored as f32 / self.food_to_grow as f32
    }
}

impl WallLevel {
    /// Combat strength bonus granted to the city's ranged attack and defense.
    pub fn defense_bonus(&self) -> i32 {
        match self {
            WallLevel::None        => 0,
            WallLevel::Ancient     => 3,
            WallLevel::Medieval    => 5,
            WallLevel::Renaissance => 8,
        }
    }

    /// Maximum HP of walls at this tier.
    pub fn max_hp(&self) -> u32 {
        match self {
            WallLevel::None        => 0,
            WallLevel::Ancient     => 50,
            WallLevel::Medieval    => 100,
            WallLevel::Renaissance => 200,
        }
    }
}
