use crate::YieldBundle;
use libhexgrid::types::{Elevation, MovementCost};

pub trait TerrainDef: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn base_yields(&self) -> YieldBundle;
    fn movement_cost(&self) -> MovementCost;
    fn elevation(&self) -> Elevation;
    fn is_land(&self) -> bool;
    fn is_water(&self) -> bool {
        !self.is_land()
    }
}

// ---- Built-in terrain types ----

#[derive(Debug, Clone, Copy, Default)]
pub struct Grassland;

impl TerrainDef for Grassland {
    fn name(&self) -> &'static str { "Grassland" }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 2)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }
    fn elevation(&self) -> Elevation { Elevation::FLAT }
    fn is_land(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Plains;

impl TerrainDef for Plains {
    fn name(&self) -> &'static str { "Plains" }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 1)
            .with(crate::YieldType::Production, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }
    fn elevation(&self) -> Elevation { Elevation::FLAT }
    fn is_land(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Desert;

impl TerrainDef for Desert {
    fn name(&self) -> &'static str { "Desert" }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }
    fn elevation(&self) -> Elevation { Elevation::FLAT }
    fn is_land(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tundra;

impl TerrainDef for Tundra {
    fn name(&self) -> &'static str { "Tundra" }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }
    fn elevation(&self) -> Elevation { Elevation::FLAT }
    fn is_land(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Snow;

impl TerrainDef for Snow {
    fn name(&self) -> &'static str { "Snow" }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }
    fn elevation(&self) -> Elevation { Elevation::FLAT }
    fn is_land(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Coast;

impl TerrainDef for Coast {
    fn name(&self) -> &'static str { "Coast" }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 1)
            .with(crate::YieldType::Gold, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }
    fn elevation(&self) -> Elevation { Elevation::FLAT }
    fn is_land(&self) -> bool { false }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Ocean;

impl TerrainDef for Ocean {
    fn name(&self) -> &'static str { "Ocean" }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 1)
    }
    fn movement_cost(&self) -> MovementCost { MovementCost::ONE }
    fn elevation(&self) -> Elevation { Elevation::FLAT }
    fn is_land(&self) -> bool { false }
}

/// Impassable high-elevation terrain. Blocks line-of-sight for units at lower elevations.
#[derive(Debug, Clone, Copy, Default)]
pub struct Mountain;

impl TerrainDef for Mountain {
    fn name(&self) -> &'static str { "Mountain" }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn elevation(&self) -> Elevation { Elevation::High }
    fn is_land(&self) -> bool { true }
}

/// Enum wrapping all built-in terrain types for easy storage.
#[derive(Debug, Clone, Copy)]
pub enum BuiltinTerrain {
    Grassland(Grassland),
    Plains(Plains),
    Desert(Desert),
    Tundra(Tundra),
    Snow(Snow),
    Coast(Coast),
    Ocean(Ocean),
    Mountain(Mountain),
}

impl BuiltinTerrain {
    pub fn as_def(&self) -> &dyn TerrainDef {
        match self {
            BuiltinTerrain::Grassland(t) => t,
            BuiltinTerrain::Plains(t)    => t,
            BuiltinTerrain::Desert(t)    => t,
            BuiltinTerrain::Tundra(t)    => t,
            BuiltinTerrain::Snow(t)      => t,
            BuiltinTerrain::Coast(t)     => t,
            BuiltinTerrain::Ocean(t)     => t,
            BuiltinTerrain::Mountain(t)  => t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_builtin_yields() {
        assert_eq!(Grassland.base_yields().food, 2);
        assert_eq!(Plains.base_yields().food, 1);
        assert_eq!(Plains.base_yields().production, 1);
        assert_eq!(Desert.base_yields().food, 0);
        assert!(Coast.is_water());
        assert!(Ocean.is_water());
        assert!(Grassland.is_land());
    }
}
