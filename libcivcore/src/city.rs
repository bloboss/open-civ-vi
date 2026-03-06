use libcommon::{BuildingId, CityId, CivId, DistrictTypeId, YieldBundle};
use libhexgrid::coord::HexCoord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CityStatus {
    Capital,
    City,
    Occupied,
    Puppet,
    Razed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WallLevel {
    None,
    Ancient,
    Medieval,
    Renaissance,
}

#[derive(Debug, Clone)]
pub enum ProductionItem {
    Unit(&'static str),
    Building(BuildingId),
    District(DistrictTypeId),
    Wonder(&'static str),
}

#[derive(Debug, Clone)]
pub struct City {
    pub id: CityId,
    pub name: String,
    pub owner: CivId,
    pub founded_by: CivId,
    pub coord: HexCoord,
    pub status: CityStatus,
    pub population: u32,
    pub food_stored: u32,
    pub food_to_grow: u32,
    pub production_stored: u32,
    pub current_production: Option<ProductionItem>,
    pub walls: WallLevel,
    pub buildings: Vec<BuildingId>,
    pub districts: Vec<DistrictTypeId>,
    pub yields: YieldBundle,
}

impl City {
    pub fn new(id: CityId, name: String, owner: CivId, coord: HexCoord) -> Self {
        Self {
            id,
            name,
            owner,
            founded_by: owner,
            coord,
            status: CityStatus::City,
            population: 1,
            food_stored: 0,
            food_to_grow: 15,
            production_stored: 0,
            current_production: None,
            walls: WallLevel::None,
            buildings: Vec::new(),
            districts: Vec::new(),
            yields: YieldBundle::default(),
        }
    }

    pub fn is_capital(&self) -> bool {
        matches!(self.status, CityStatus::Capital)
    }

    pub fn growth_progress(&self) -> f32 {
        if self.food_to_grow == 0 {
            return 1.0;
        }
        self.food_stored as f32 / self.food_to_grow as f32
    }
}
