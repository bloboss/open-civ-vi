use crate::{BuildingId, CityId, DistrictTypeId, NaturalWonderId, YieldBundle};
use libhexgrid::coord::HexCoord;

pub trait DistrictDef: std::fmt::Debug {
    fn id(&self) -> DistrictTypeId;
    fn name(&self) -> &'static str;
    fn base_cost(&self) -> u32;
    fn max_per_city(&self) -> u32 { 1 }
}

pub trait BuildingDef: std::fmt::Debug {
    fn id(&self) -> BuildingId;
    fn name(&self) -> &'static str;
    fn cost(&self) -> u32;
    fn maintenance(&self) -> u32;
    fn yields(&self) -> YieldBundle;
    fn requires_district(&self) -> Option<DistrictTypeId>;
}

/// Adjacency bonus context for a district placement.
///
/// `adjacent_natural_wonders` holds the IDs of specific natural wonders adjacent
/// to the district tile, allowing district definitions to apply wonder-specific
/// bonuses (e.g. Uluru grants +2 Faith to an adjacent Holy Site).
#[derive(Debug, Clone, Default)]
pub struct AdjacencyContext {
    pub adjacent_districts: Vec<DistrictTypeId>,
    pub adjacent_natural_wonders: Vec<NaturalWonderId>,
    pub adjacent_mountains: u32,
    pub adjacent_rivers: u32,
    pub adjacent_rainforest: u32,
}

impl AdjacencyContext {
    pub fn new() -> Self {
        Self::default()
    }
}

// TODO(PHASE3-8.2): PlacedDistrict is ready; wire it into City.districts (replace
//   Vec<DistrictTypeId>) and into place_district() on RulesEngine. See also 4.3
//   which triggers DistrictBuilt StateDelta.
/// A district that has been placed on the map.
#[derive(Debug, Clone)]
pub struct PlacedDistrict {
    pub district_type: DistrictTypeId,
    pub city_id: CityId,
    pub coord: HexCoord,
    pub buildings: Vec<BuildingId>,
    pub is_pillaged: bool,
}

impl PlacedDistrict {
    pub fn new(district_type: DistrictTypeId, city_id: CityId, coord: HexCoord) -> Self {
        Self {
            district_type,
            city_id,
            coord,
            buildings: Vec::new(),
            is_pillaged: false,
        }
    }
}
