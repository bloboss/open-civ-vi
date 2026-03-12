use crate::YieldBundle;
use libhexgrid::types::Elevation;

use super::feature::BuiltinFeature;
use super::terrain::BuiltinTerrain;

pub trait TileImprovement: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn yield_bonus(&self) -> YieldBundle;
    /// Number of turns to build (base, without modifiers).
    fn build_turns(&self) -> u32;
    /// Whether this improvement can be placed on a tile with the given terrain and feature.
    fn valid_on(&self, terrain: BuiltinTerrain, feature: Option<BuiltinFeature>, elevation: Elevation) -> bool;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Farm;
impl TileImprovement for Farm {
    fn name(&self) -> &'static str { "Farm" }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 1)
    }
    fn build_turns(&self) -> u32 { 5 }
    fn valid_on(&self, terrain: BuiltinTerrain, feature: Option<BuiltinFeature>, elevation: Elevation) -> bool {
        if !terrain.is_land() { return false; }
        if elevation == Elevation::High { return false; }  // Mountains
        // Flat inland only (not hills -- hills get Mine)
        if elevation >= Elevation::HILLS { return false; }
        // Not Snow
        if terrain == BuiltinTerrain::Snow { return false; }
        // Desert only if has Floodplain or Oasis feature
        if terrain == BuiltinTerrain::Desert {
            return feature.is_some_and(|f| matches!(f, BuiltinFeature::Floodplain | BuiltinFeature::Oasis));
        }
        true
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Mine;
impl TileImprovement for Mine {
    fn name(&self) -> &'static str { "Mine" }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Production, 1)
    }
    fn build_turns(&self) -> u32 { 5 }
    fn valid_on(&self, terrain: BuiltinTerrain, _feature: Option<BuiltinFeature>, elevation: Elevation) -> bool {
        terrain.is_land() && elevation >= Elevation::HILLS
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LumberMill;
impl TileImprovement for LumberMill {
    fn name(&self) -> &'static str { "Lumber Mill" }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Production, 2)
    }
    fn build_turns(&self) -> u32 { 5 }
    fn valid_on(&self, _terrain: BuiltinTerrain, feature: Option<BuiltinFeature>, _elevation: Elevation) -> bool {
        feature == Some(BuiltinFeature::Forest)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TradingPost;
impl TileImprovement for TradingPost {
    fn name(&self) -> &'static str { "Trading Post" }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Gold, 1)
    }
    fn build_turns(&self) -> u32 { 5 }
    fn valid_on(&self, terrain: BuiltinTerrain, _feature: Option<BuiltinFeature>, elevation: Elevation) -> bool {
        terrain.is_land() && elevation != Elevation::High
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Fort;
impl TileImprovement for Fort {
    fn name(&self) -> &'static str { "Fort" }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn build_turns(&self) -> u32 { 10 }
    fn valid_on(&self, terrain: BuiltinTerrain, _feature: Option<BuiltinFeature>, elevation: Elevation) -> bool {
        terrain.is_land() && elevation != Elevation::High
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Airstrip;
impl TileImprovement for Airstrip {
    fn name(&self) -> &'static str { "Airstrip" }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn build_turns(&self) -> u32 { 10 }
    fn valid_on(&self, terrain: BuiltinTerrain, _feature: Option<BuiltinFeature>, elevation: Elevation) -> bool {
        // Flat land only -- not hills, not mountain, not water
        terrain.is_land() && elevation == Elevation::FLAT
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MissileSilo;
impl TileImprovement for MissileSilo {
    fn name(&self) -> &'static str { "Missile Silo" }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn build_turns(&self) -> u32 { 15 }
    fn valid_on(&self, terrain: BuiltinTerrain, _feature: Option<BuiltinFeature>, elevation: Elevation) -> bool {
        terrain.is_land() && elevation == Elevation::FLAT
    }
}

/// Enum wrapping all built-in tile improvements for direct inline storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinImprovement {
    Farm(Farm),
    Mine(Mine),
    LumberMill(LumberMill),
    TradingPost(TradingPost),
    Fort(Fort),
    Airstrip(Airstrip),
    MissileSilo(MissileSilo),
}

impl BuiltinImprovement {
    pub fn as_def(&self) -> &dyn TileImprovement {
        match self {
            BuiltinImprovement::Farm(i)        => i,
            BuiltinImprovement::Mine(i)        => i,
            BuiltinImprovement::LumberMill(i)  => i,
            BuiltinImprovement::TradingPost(i) => i,
            BuiltinImprovement::Fort(i)        => i,
            BuiltinImprovement::Airstrip(i)    => i,
            BuiltinImprovement::MissileSilo(i) => i,
        }
    }
}
