use crate::{CivicId, CivicRefs, TechId, TechRefs, YieldBundle};

use super::feature::BuiltinFeature;
use super::resource::BuiltinResource;
use super::terrain::BuiltinTerrain;

/// Elevation constraint for improvement placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevationReq {
    /// No restriction.
    Any,
    /// Elevation is flat (not hills, not mountain).
    Flat,
    /// Elevation is hills or higher (but not mountain).
    HillsOrMore,
    /// Any passable elevation (not mountain).
    NotMountain,
}

/// Adjacency constraint: at least one of the 6 neighboring tiles must satisfy this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProximityReq {
    AdjacentTerrain(BuiltinTerrain),
    AdjacentFeature(BuiltinFeature),
    AdjacentResource(BuiltinResource),
}

/// Pure data describing all placement constraints for a `BuiltinImprovement`.
#[derive(Debug, Clone, Copy)]
pub struct ImprovementRequirements {
    /// Tile must be a land tile (not Coast or Ocean).
    pub requires_land: bool,
    /// Tile must be a water tile.
    pub requires_water: bool,
    /// Elevation constraint on the target tile.
    pub elevation: ElevationReq,
    /// Terrains that are never valid, regardless of other fields.
    pub blocked_terrains: &'static [BuiltinTerrain],
    /// If Some(f), the tile must have exactly this feature.
    pub required_feature: Option<BuiltinFeature>,
    /// Per-terrain conditional: on matching terrain, one of the listed features must be present.
    /// Example: Farm on Desert requires Floodplain or Oasis.
    pub conditional_features: &'static [(BuiltinTerrain, &'static [BuiltinFeature])],
    /// Resource that must be present on the tile itself.
    pub required_resource: Option<BuiltinResource>,
    /// Tech that must be researched. Use `tech_refs.unreachable` for improvements
    /// not yet tied to a real tech (prerequisites_met() is always false for it).
    pub required_tech: Option<TechId>,
    /// Civic that must be completed.
    pub required_civic: Option<CivicId>,
    /// Adjacency constraint: at least one of the 6 neighbors must satisfy this.
    pub proximity: Option<ProximityReq>,
}

/// All built-in tile improvements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinImprovement {
    Farm,
    Mine,
    LumberMill,
    TradingPost,
    Fort,
    Airstrip,
    MissileSilo,
    Quarry,
    Plantation,
    Camp,
    FishingBoats,
    Pasture,
    /// Egyptian unique improvement.
    Sphinx,
    /// Indian unique improvement.
    Stepwell,
    /// Oil resource improvement (land).
    OilWell,
    /// Oil resource improvement (sea).
    OffshoreOilRig,
    /// Tourism improvement requiring high appeal.
    BeachResort,
    /// French unique improvement.
    Chateau,
    /// La Venta city-state unique improvement.
    ColossalHead,
    /// Chinese unique improvement.
    GreatWall,
    /// Scythian unique improvement.
    Kurgan,
    /// Spanish unique improvement.
    Mission,
    /// Roman unique improvement (built by Legion).
    RomanFort,
    /// Sumerian unique improvement.
    Ziggurat,
    // ── Gathering Storm improvements ──────────────────────────────────────
    /// Renewable energy — generates 2 power.
    SolarFarm,
    /// Renewable energy — generates 2 power.
    WindFarm,
    /// Renewable energy (sea tile) — generates 2 power.
    OffshoreWindFarm,
    /// Renewable energy (geothermal fissure) — generates 4 power.
    GeothermalPlant,
    /// Ocean habitation improvement.
    Seastead,
    /// Allows crossing mountain tiles.
    MountainTunnel,
    /// Tourism improvement on snow/tundra.
    SkiResort,
}

impl BuiltinImprovement {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinImprovement::Farm         => "Farm",
            BuiltinImprovement::Mine         => "Mine",
            BuiltinImprovement::LumberMill   => "Lumber Mill",
            BuiltinImprovement::TradingPost  => "Trading Post",
            BuiltinImprovement::Fort         => "Fort",
            BuiltinImprovement::Airstrip     => "Airstrip",
            BuiltinImprovement::MissileSilo  => "Missile Silo",
            BuiltinImprovement::Quarry       => "Quarry",
            BuiltinImprovement::Plantation   => "Plantation",
            BuiltinImprovement::Camp         => "Camp",
            BuiltinImprovement::FishingBoats => "Fishing Boats",
            BuiltinImprovement::Pasture      => "Pasture",
            BuiltinImprovement::Sphinx          => "Sphinx",
            BuiltinImprovement::Stepwell        => "Stepwell",
            BuiltinImprovement::OilWell         => "Oil Well",
            BuiltinImprovement::OffshoreOilRig  => "Offshore Oil Rig",
            BuiltinImprovement::BeachResort     => "Beach Resort",
            BuiltinImprovement::Chateau         => "Chateau",
            BuiltinImprovement::ColossalHead    => "Colossal Head",
            BuiltinImprovement::GreatWall       => "Great Wall",
            BuiltinImprovement::Kurgan          => "Kurgan",
            BuiltinImprovement::Mission         => "Mission",
            BuiltinImprovement::RomanFort       => "Roman Fort",
            BuiltinImprovement::Ziggurat        => "Ziggurat",
            BuiltinImprovement::SolarFarm       => "Solar Farm",
            BuiltinImprovement::WindFarm        => "Wind Farm",
            BuiltinImprovement::OffshoreWindFarm => "Offshore Wind Farm",
            BuiltinImprovement::GeothermalPlant => "Geothermal Plant",
            BuiltinImprovement::Seastead        => "Seastead",
            BuiltinImprovement::MountainTunnel  => "Mountain Tunnel",
            BuiltinImprovement::SkiResort       => "Ski Resort",
        }
    }

    pub fn yield_bonus(self) -> YieldBundle {
        use crate::YieldType;
        match self {
            BuiltinImprovement::Farm        => YieldBundle::new().with(YieldType::Food, 1),
            BuiltinImprovement::Mine        => YieldBundle::new().with(YieldType::Production, 1),
            BuiltinImprovement::LumberMill  => YieldBundle::new().with(YieldType::Production, 1),
            BuiltinImprovement::TradingPost => YieldBundle::new().with(YieldType::Gold, 1),
            BuiltinImprovement::Fort         => YieldBundle::new(),
            BuiltinImprovement::Airstrip     => YieldBundle::new(),
            BuiltinImprovement::MissileSilo  => YieldBundle::new(),
            BuiltinImprovement::Quarry       => YieldBundle::new().with(YieldType::Production, 1),
            BuiltinImprovement::Plantation   => YieldBundle::new().with(YieldType::Gold, 2),
            BuiltinImprovement::Camp         => YieldBundle::new().with(YieldType::Gold, 1),
            BuiltinImprovement::FishingBoats => YieldBundle::new().with(YieldType::Food, 1),
            BuiltinImprovement::Pasture      => YieldBundle::new().with(YieldType::Production, 1),
            BuiltinImprovement::Sphinx          => YieldBundle::new().with(YieldType::Culture, 1).with(YieldType::Faith, 1),
            BuiltinImprovement::Stepwell        => YieldBundle::new().with(YieldType::Food, 1).with(YieldType::Housing, 2),
            BuiltinImprovement::OilWell         => YieldBundle::new().with(YieldType::Production, 2),
            BuiltinImprovement::OffshoreOilRig  => YieldBundle::new().with(YieldType::Production, 2),
            BuiltinImprovement::BeachResort     => YieldBundle::new().with(YieldType::Gold, 1),
            BuiltinImprovement::Chateau         => YieldBundle::new().with(YieldType::Culture, 2).with(YieldType::Gold, 1),
            BuiltinImprovement::ColossalHead    => YieldBundle::new().with(YieldType::Faith, 2),
            BuiltinImprovement::GreatWall       => YieldBundle::new().with(YieldType::Gold, 1),
            BuiltinImprovement::Kurgan          => YieldBundle::new().with(YieldType::Gold, 3).with(YieldType::Faith, 1),
            BuiltinImprovement::Mission         => YieldBundle::new().with(YieldType::Faith, 2),
            BuiltinImprovement::RomanFort       => YieldBundle::new(),
            BuiltinImprovement::Ziggurat        => YieldBundle::new().with(YieldType::Science, 2),
            BuiltinImprovement::SolarFarm       => YieldBundle::new().with(YieldType::Production, 2),
            BuiltinImprovement::WindFarm        => YieldBundle::new().with(YieldType::Production, 2),
            BuiltinImprovement::OffshoreWindFarm => YieldBundle::new().with(YieldType::Production, 2),
            BuiltinImprovement::GeothermalPlant => YieldBundle::new().with(YieldType::Science, 2).with(YieldType::Production, 1),
            BuiltinImprovement::Seastead        => YieldBundle::new().with(YieldType::Food, 2).with(YieldType::Housing, 1),
            BuiltinImprovement::MountainTunnel  => YieldBundle::new(),
            BuiltinImprovement::SkiResort       => YieldBundle::new().with(YieldType::Tourism, 1),
        }
    }

    pub fn build_turns(self) -> u32 {
        match self {
            BuiltinImprovement::Farm        => 5,
            BuiltinImprovement::Mine        => 5,
            BuiltinImprovement::LumberMill  => 5,
            BuiltinImprovement::TradingPost => 5,
            BuiltinImprovement::Fort        => 10,
            BuiltinImprovement::Airstrip    => 10,
            BuiltinImprovement::MissileSilo  => 15,
            BuiltinImprovement::Quarry       => 5,
            BuiltinImprovement::Plantation   => 5,
            BuiltinImprovement::Camp         => 5,
            BuiltinImprovement::FishingBoats => 5,
            BuiltinImprovement::Pasture      => 5,
            BuiltinImprovement::Sphinx          => 5,
            BuiltinImprovement::Stepwell        => 5,
            BuiltinImprovement::OilWell         => 5,
            BuiltinImprovement::OffshoreOilRig  => 5,
            BuiltinImprovement::BeachResort     => 5,
            BuiltinImprovement::Chateau         => 5,
            BuiltinImprovement::ColossalHead    => 5,
            BuiltinImprovement::GreatWall       => 5,
            BuiltinImprovement::Kurgan          => 5,
            BuiltinImprovement::Mission         => 5,
            BuiltinImprovement::RomanFort       => 10,
            BuiltinImprovement::Ziggurat        => 5,
            BuiltinImprovement::SolarFarm       => 5,
            BuiltinImprovement::WindFarm        => 5,
            BuiltinImprovement::OffshoreWindFarm => 5,
            BuiltinImprovement::GeothermalPlant => 5,
            BuiltinImprovement::Seastead        => 5,
            BuiltinImprovement::MountainTunnel  => 5,
            BuiltinImprovement::SkiResort       => 5,
        }
    }

    pub fn requirements(self, tech_refs: &TechRefs, _civic_refs: &CivicRefs) -> ImprovementRequirements {
        match self {
            BuiltinImprovement::Farm => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Flat,
                blocked_terrains:     &[BuiltinTerrain::Snow],
                required_feature:     None,
                conditional_features: &[(
                    BuiltinTerrain::Desert,
                    &[BuiltinFeature::Floodplain, BuiltinFeature::Oasis],
                )],
                required_resource:    None,
                required_tech:        Some(tech_refs.pottery),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Mine => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::HillsOrMore,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.mining),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::LumberMill => ImprovementRequirements {
                requires_land:        false,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     Some(BuiltinFeature::Forest),
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::TradingPost => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::NotMountain,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Fort => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::NotMountain,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Airstrip => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Flat,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::MissileSilo => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Flat,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Quarry => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.mining),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Plantation => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.irrigation),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Camp => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.animal_husbandry),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::FishingBoats => ImprovementRequirements {
                requires_land:        false,
                requires_water:       true,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.sailing),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Pasture => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.animal_husbandry),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Sphinx => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[BuiltinTerrain::Snow, BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        None,
                required_civic:       Some(_civic_refs.craftsmanship),
                proximity:            None,
            },
            BuiltinImprovement::Stepwell => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[BuiltinTerrain::Snow, BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        None,
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::OilWell => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    Some(BuiltinResource::Oil),
                required_tech:        Some(tech_refs.unreachable), // Steel not in refs
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::OffshoreOilRig => ImprovementRequirements {
                requires_land:        false,
                requires_water:       true,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    Some(BuiltinResource::Oil),
                required_tech:        Some(tech_refs.unreachable), // Plastics not in refs
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::BeachResort => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Flat,
                blocked_terrains:     &[BuiltinTerrain::Snow, BuiltinTerrain::Tundra, BuiltinTerrain::Desert],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable), // Radio not in refs
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Chateau => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[BuiltinTerrain::Snow, BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        None,
                required_civic:       Some(_civic_refs.unreachable), // Humanism not in refs
                proximity:            None,
            },
            BuiltinImprovement::ColossalHead => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[BuiltinTerrain::Snow, BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        None,
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::GreatWall => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.masonry),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Kurgan => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[BuiltinTerrain::Snow, BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.animal_husbandry),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Mission => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[BuiltinTerrain::Snow, BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable), // Education not in refs
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::RomanFort => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::NotMountain,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        None, // Built by Legion unit, no tech required
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Ziggurat => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Flat,
                blocked_terrains:     &[BuiltinTerrain::Snow, BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        None, // No tech required
                required_civic:       None,
                proximity:            None,
            },
            // ── Gathering Storm improvements ─────────────────────────────────
            BuiltinImprovement::SolarFarm => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Flat,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::WindFarm => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Flat,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::OffshoreWindFarm => ImprovementRequirements {
                requires_land:        false,
                requires_water:       true,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::GeothermalPlant => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     Some(BuiltinFeature::VolcanicSoil),
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::Seastead => ImprovementRequirements {
                requires_land:        false,
                requires_water:       true,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::MountainTunnel => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
            BuiltinImprovement::SkiResort => ImprovementRequirements {
                requires_land:        true,
                requires_water:       false,
                elevation:            ElevationReq::Any,
                blocked_terrains:     &[BuiltinTerrain::Grassland, BuiltinTerrain::Plains, BuiltinTerrain::Desert, BuiltinTerrain::Coast, BuiltinTerrain::Ocean, BuiltinTerrain::Mountain],
                required_feature:     None,
                conditional_features: &[],
                required_resource:    None,
                required_tech:        Some(tech_refs.unreachable),
                required_civic:       None,
                proximity:            None,
            },
        }
    }
}
