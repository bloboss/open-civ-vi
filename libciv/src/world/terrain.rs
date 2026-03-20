use crate::YieldBundle;
use libhexgrid::types::{Elevation, MovementCost};

/// All built-in terrain types as a plain enum.
/// Deriving PartialEq/Eq/Hash allows direct comparison without string matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinTerrain {
    #[default]
    Grassland,
    Plains,
    Desert,
    Tundra,
    Snow,
    Coast,
    Ocean,
    Mountain,
}

impl BuiltinTerrain {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinTerrain::Grassland => "Grassland",
            BuiltinTerrain::Plains    => "Plains",
            BuiltinTerrain::Desert    => "Desert",
            BuiltinTerrain::Tundra    => "Tundra",
            BuiltinTerrain::Snow      => "Snow",
            BuiltinTerrain::Coast     => "Coast",
            BuiltinTerrain::Ocean     => "Ocean",
            BuiltinTerrain::Mountain  => "Mountain",
        }
    }

    pub fn base_yields(self) -> YieldBundle {
        match self {
            BuiltinTerrain::Grassland => YieldBundle::new().with(crate::YieldType::Food, 2),
            BuiltinTerrain::Plains    => YieldBundle::new()
                .with(crate::YieldType::Food, 1)
                .with(crate::YieldType::Production, 1),
            BuiltinTerrain::Desert    => YieldBundle::new(),
            BuiltinTerrain::Tundra    => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinTerrain::Snow      => YieldBundle::new(),
            BuiltinTerrain::Coast     => YieldBundle::new()
                .with(crate::YieldType::Food, 1)
                .with(crate::YieldType::Gold, 1),
            BuiltinTerrain::Ocean     => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinTerrain::Mountain  => YieldBundle::new(),
        }
    }

    pub fn movement_cost(self) -> MovementCost {
        match self {
            BuiltinTerrain::Mountain => MovementCost::Impassable,
            _                        => MovementCost::ONE,
        }
    }

    /// Base elevation before the per-tile `hills` flag is applied.
    pub fn elevation(self) -> Elevation {
        match self {
            BuiltinTerrain::Ocean    => Elevation::SEA_LEVEL,  // Level(0)
            BuiltinTerrain::Coast    => Elevation::COASTAL,    // Level(1)
            BuiltinTerrain::Mountain => Elevation::High,
            _                        => Elevation::FLAT,       // Level(2)
        }
    }

    pub fn is_land(self) -> bool {
        !matches!(self, BuiltinTerrain::Coast | BuiltinTerrain::Ocean)
    }

    pub fn is_water(self) -> bool {
        !self.is_land()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_builtin_yields() {
        assert_eq!(BuiltinTerrain::Grassland.base_yields().food, 2);
        assert_eq!(BuiltinTerrain::Plains.base_yields().food, 1);
        assert_eq!(BuiltinTerrain::Plains.base_yields().production, 1);
        assert_eq!(BuiltinTerrain::Desert.base_yields().food, 0);
        assert!(BuiltinTerrain::Coast.is_water());
        assert!(BuiltinTerrain::Ocean.is_water());
        assert!(BuiltinTerrain::Grassland.is_land());
    }

    #[test]
    fn test_terrain_equality() {
        assert_eq!(BuiltinTerrain::Grassland, BuiltinTerrain::Grassland);
        assert_ne!(BuiltinTerrain::Grassland, BuiltinTerrain::Desert);
        assert_ne!(BuiltinTerrain::Snow, BuiltinTerrain::Tundra);
    }
}
