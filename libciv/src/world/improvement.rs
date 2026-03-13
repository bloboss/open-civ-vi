use crate::YieldBundle;

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
    /// Tech node name (matches TechNode::name) that must be researched.
    /// Use "Unreachable" for improvements not yet tied to a real tech.
    pub required_tech: Option<&'static str>,
    /// Civic node name (matches CivicNode::name) that must be completed.
    pub required_civic: Option<&'static str>,
    /// Adjacency constraint: at least one of the 6 neighbors must satisfy this.
    pub proximity: Option<ProximityReq>,
}

/// All built-in tile improvements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinImprovement {
    Farm,
    Mine,
    LumberMill,
    TradingPost,
    Fort,
    Airstrip,
    MissileSilo,
}

impl BuiltinImprovement {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinImprovement::Farm        => "Farm",
            BuiltinImprovement::Mine        => "Mine",
            BuiltinImprovement::LumberMill  => "Lumber Mill",
            BuiltinImprovement::TradingPost => "Trading Post",
            BuiltinImprovement::Fort        => "Fort",
            BuiltinImprovement::Airstrip    => "Airstrip",
            BuiltinImprovement::MissileSilo => "Missile Silo",
        }
    }

    pub fn yield_bonus(self) -> YieldBundle {
        match self {
            BuiltinImprovement::Farm        => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinImprovement::Mine        => YieldBundle::new().with(crate::YieldType::Production, 1),
            BuiltinImprovement::LumberMill  => YieldBundle::new().with(crate::YieldType::Production, 2),
            BuiltinImprovement::TradingPost => YieldBundle::new().with(crate::YieldType::Gold, 1),
            BuiltinImprovement::Fort        => YieldBundle::new(),
            BuiltinImprovement::Airstrip    => YieldBundle::new(),
            BuiltinImprovement::MissileSilo => YieldBundle::new(),
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
            BuiltinImprovement::MissileSilo => 15,
        }
    }

    pub fn requirements(self) -> ImprovementRequirements {
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
                required_tech:        Some("Pottery"),
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
                required_tech:        Some("Mining"),
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
                required_tech:        Some("Unreachable"),
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
                required_tech:        Some("Unreachable"),
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
                required_tech:        Some("Unreachable"),
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
                required_tech:        Some("Unreachable"),
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
                required_tech:        Some("Unreachable"),
                required_civic:       None,
                proximity:            None,
            },
        }
    }
}
