use crate::{BuildingId, CityId, CivicId, CivicRefs, NaturalWonderId, TechId, TechRefs, YieldBundle};
use libhexgrid::coord::HexCoord;

use super::super::world::terrain::BuiltinTerrain;

/// Placement constraints for a `BuiltinDistrict`.
#[derive(Debug, Clone, Copy)]
pub struct DistrictRequirements {
    /// Tile must be land (not Coast or Ocean).
    pub requires_land: bool,
    /// Tile must be a water tile (Coast or Ocean).
    pub requires_water: bool,
    /// Terrains that are never valid for this district.
    pub forbidden_terrains: &'static [BuiltinTerrain],
    /// Tech that must be researched. Use `tech_refs.unreachable` for districts
    /// not yet tied to a real tech (prerequisites_met() is always false for it).
    pub required_tech: Option<TechId>,
    /// Civic that must be completed. Use `civic_refs.unreachable` for districts
    /// not yet tied to a real civic.
    pub required_civic: Option<CivicId>,
}

/// All built-in district types, following Civilization VI's district system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinDistrict {
    /// Science district — produces Great Scientists and science.
    Campus,
    /// Culture district — produces Great Writers, Artists, and Musicians.
    TheaterSquare,
    /// Commerce district — produces Great Merchants and gold.
    CommercialHub,
    /// Maritime district — built on coast; produces Great Admirals and gold.
    Harbor,
    /// Faith district — produces Great Prophets and faith.
    HolySite,
    /// Military district — trains units and provides early defenses.
    Encampment,
    /// Production district — produces Great Engineers and production.
    IndustrialZone,
    /// Amenities district — provides entertainment and amenities.
    EntertainmentComplex,
    /// Coastal amenities district — built on coast; provides amenities.
    WaterPark,
    /// Infrastructure district — provides housing and water.
    Aqueduct,
    /// Water infrastructure — built on rivers; provides flood protection.
    Dam,
    /// Maritime passage — connects water bodies through land.
    Canal,
    /// Air unit district — houses and maintains aircraft.
    Aerodrome,
    /// Housing district — provides housing based on appeal.
    Neighborhood,
    /// Space race district — required for Science Victory projects.
    Spaceport,
    /// Implicit city center district.
    CityCenter,
    /// Russian unique district — replaces Holy Site.
    Lavra,
    /// Kongolese unique district — replaces Neighborhood.
    Mbanza,
    /// Brazilian unique district — replaces Entertainment Complex.
    StreetCarnival,
    /// English unique district — replaces Harbor.
    RoyalNavyDockyard,
}

impl BuiltinDistrict {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinDistrict::Campus               => "Campus",
            BuiltinDistrict::TheaterSquare        => "Theater Square",
            BuiltinDistrict::CommercialHub        => "Commercial Hub",
            BuiltinDistrict::Harbor               => "Harbor",
            BuiltinDistrict::HolySite             => "Holy Site",
            BuiltinDistrict::Encampment           => "Encampment",
            BuiltinDistrict::IndustrialZone       => "Industrial Zone",
            BuiltinDistrict::EntertainmentComplex => "Entertainment Complex",
            BuiltinDistrict::WaterPark            => "Water Park",
            BuiltinDistrict::Aqueduct             => "Aqueduct",
            BuiltinDistrict::Dam                  => "Dam",
            BuiltinDistrict::Canal                => "Canal",
            BuiltinDistrict::Aerodrome            => "Aerodrome",
            BuiltinDistrict::Neighborhood         => "Neighborhood",
            BuiltinDistrict::Spaceport            => "Spaceport",
            BuiltinDistrict::CityCenter           => "City Center",
            BuiltinDistrict::Lavra                => "Lavra",
            BuiltinDistrict::Mbanza               => "Mbanza",
            BuiltinDistrict::StreetCarnival       => "Street Carnival",
            BuiltinDistrict::RoyalNavyDockyard    => "Royal Navy Dockyard",
        }
    }

    pub fn base_cost(self) -> u32 {
        match self {
            BuiltinDistrict::Campus               => 54,
            BuiltinDistrict::TheaterSquare        => 54,
            BuiltinDistrict::CommercialHub        => 54,
            BuiltinDistrict::Harbor               => 54,
            BuiltinDistrict::HolySite             => 54,
            BuiltinDistrict::Encampment           => 54,
            BuiltinDistrict::IndustrialZone       => 54,
            BuiltinDistrict::EntertainmentComplex => 54,
            BuiltinDistrict::WaterPark            => 54,
            BuiltinDistrict::Aqueduct             => 36,
            BuiltinDistrict::Dam                  => 54,
            BuiltinDistrict::Canal                => 54,
            BuiltinDistrict::Aerodrome            => 54,
            BuiltinDistrict::Neighborhood         => 54,
            BuiltinDistrict::Spaceport            => 1800,
            BuiltinDistrict::CityCenter           => 0,
            BuiltinDistrict::Lavra                => 27,
            BuiltinDistrict::Mbanza               => 27,
            BuiltinDistrict::StreetCarnival       => 27,
            BuiltinDistrict::RoyalNavyDockyard    => 27,
        }
    }

    pub fn requirements(self, tech_refs: &TechRefs, civic_refs: &CivicRefs) -> DistrictRequirements {
        match self {
            BuiltinDistrict::Campus => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      Some(tech_refs.writing),
                required_civic:     None,
            },
            BuiltinDistrict::TheaterSquare => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      None,
                required_civic:     Some(civic_refs.craftsmanship),
            },
            BuiltinDistrict::CommercialHub => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                // "Currency" is not yet in the tech tree — use unreachable sentinel.
                required_tech:      Some(tech_refs.unreachable),
                required_civic:     None,
            },
            BuiltinDistrict::Harbor => DistrictRequirements {
                requires_land:      false,
                requires_water:     true,
                forbidden_terrains: &[],
                required_tech:      Some(tech_refs.sailing),
                required_civic:     None,
            },
            BuiltinDistrict::HolySite => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      Some(tech_refs.astrology),
                required_civic:     None,
            },
            BuiltinDistrict::Encampment => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      Some(tech_refs.bronze_working),
                required_civic:     None,
            },
            BuiltinDistrict::IndustrialZone => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                // "Apprenticeship" is not yet in the tech tree — use unreachable sentinel.
                required_tech:      Some(tech_refs.unreachable),
                required_civic:     None,
            },
            BuiltinDistrict::EntertainmentComplex => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      None,
                required_civic:     Some(civic_refs.early_empire),
            },
            BuiltinDistrict::WaterPark => DistrictRequirements {
                requires_land:      false,
                requires_water:     true,
                forbidden_terrains: &[],
                required_tech:      None,
                required_civic:     Some(civic_refs.unreachable),
            },
            BuiltinDistrict::Aqueduct => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      Some(tech_refs.masonry),
                required_civic:     None,
            },
            BuiltinDistrict::Dam => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      Some(tech_refs.unreachable),
                required_civic:     None,
            },
            BuiltinDistrict::Canal => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      Some(tech_refs.unreachable),
                required_civic:     None,
            },
            BuiltinDistrict::Aerodrome => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                // "Flight" is not yet in the tech tree — use unreachable sentinel.
                required_tech:      Some(tech_refs.unreachable),
                required_civic:     None,
            },
            BuiltinDistrict::Neighborhood => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      None,
                // "Urbanization" is not yet in the civic tree — use unreachable sentinel.
                required_civic:     Some(civic_refs.unreachable),
            },
            BuiltinDistrict::Spaceport => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                // "Rocketry" is not yet in the tech tree — use unreachable sentinel.
                required_tech:      Some(tech_refs.unreachable),
                required_civic:     None,
            },
            BuiltinDistrict::CityCenter => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain, BuiltinTerrain::Ocean],
                required_tech:      None,
                required_civic:     None,
            },
            BuiltinDistrict::Lavra => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      Some(tech_refs.astrology),
                required_civic:     None,
            },
            BuiltinDistrict::Mbanza => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      None,
                // "Guilds" is not yet in the civic tree — use unreachable sentinel.
                required_civic:     Some(civic_refs.unreachable),
            },
            BuiltinDistrict::StreetCarnival => DistrictRequirements {
                requires_land:      true,
                requires_water:     false,
                forbidden_terrains: &[BuiltinTerrain::Mountain],
                required_tech:      None,
                // "Games & Recreation" is not yet in the civic tree — use unreachable sentinel.
                required_civic:     Some(civic_refs.unreachable),
            },
            BuiltinDistrict::RoyalNavyDockyard => DistrictRequirements {
                requires_land:      false,
                requires_water:     true,
                forbidden_terrains: &[],
                // "Celestial Navigation" is not yet in the tech tree — use unreachable sentinel.
                required_tech:      Some(tech_refs.unreachable),
                required_civic:     None,
            },
        }
    }
}

/// Trait for custom (downstream) district types. Built-in districts use `BuiltinDistrict`.
pub trait DistrictDef: std::fmt::Debug {
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
    fn requires_district(&self) -> Option<BuiltinDistrict>;
}

/// Adjacency bonus context for a district placement.
///
/// `adjacent_natural_wonders` holds the IDs of specific natural wonders adjacent
/// to the district tile, allowing district definitions to apply wonder-specific
/// bonuses (e.g. Uluru grants +2 Faith to an adjacent Holy Site).
#[derive(Debug, Clone, Default)]
pub struct AdjacencyContext {
    pub adjacent_districts: Vec<BuiltinDistrict>,
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

/// A district that has been placed on the map.
#[derive(Debug, Clone)]
pub struct PlacedDistrict {
    pub district_type: BuiltinDistrict,
    pub city_id: CityId,
    pub coord: HexCoord,
    pub buildings: Vec<BuildingId>,
    pub is_pillaged: bool,
    /// If this is a unique district variant (e.g., Hansa for Germany), the civ it belongs to.
    pub unique_variant: Option<super::civ_identity::BuiltinCiv>,
}

impl PlacedDistrict {
    pub fn new(district_type: BuiltinDistrict, city_id: CityId, coord: HexCoord) -> Self {
        Self {
            district_type,
            city_id,
            coord,
            buildings: Vec::new(),
            is_pillaged: false,
            unique_variant: None,
        }
    }
}
