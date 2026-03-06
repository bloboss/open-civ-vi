use libcommon::{BuildingId, CityId, DistrictTypeId, YieldBundle};
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
#[derive(Debug, Clone, Default)]
pub struct AdjacencyContext {
    pub adjacent_districts: Vec<DistrictTypeId>,
    pub adjacent_natural_wonders: u32,
    pub adjacent_mountains: u32,
    pub adjacent_rivers: u32,
    pub adjacent_rainforest: u32,
}

impl AdjacencyContext {
    pub fn new() -> Self {
        Self::default()
    }
}

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
