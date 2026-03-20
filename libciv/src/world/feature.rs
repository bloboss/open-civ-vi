use crate::YieldBundle;
use libhexgrid::types::MovementCost;

/// All built-in tile features as a plain enum.
/// Deriving PartialEq/Eq/Hash allows direct comparison without string matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinFeature {
    Forest,
    Rainforest,
    Marsh,
    Floodplain,
    Reef,
    Ice,
    VolcanicSoil,
    Oasis,
}

impl BuiltinFeature {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinFeature::Forest      => "Forest",
            BuiltinFeature::Rainforest  => "Rainforest",
            BuiltinFeature::Marsh       => "Marsh",
            BuiltinFeature::Floodplain  => "Floodplain",
            BuiltinFeature::Reef        => "Reef",
            BuiltinFeature::Ice         => "Ice",
            BuiltinFeature::VolcanicSoil => "Volcanic Soil",
            BuiltinFeature::Oasis       => "Oasis",
        }
    }

    pub fn yield_modifier(self) -> YieldBundle {
        match self {
            BuiltinFeature::Forest       => YieldBundle::new().with(crate::YieldType::Production, 1),
            BuiltinFeature::Rainforest   => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinFeature::Marsh        => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinFeature::Floodplain   => YieldBundle::new().with(crate::YieldType::Food, 3),
            BuiltinFeature::Reef         => YieldBundle::new()
                .with(crate::YieldType::Food, 1)
                .with(crate::YieldType::Production, 1),
            BuiltinFeature::Ice          => YieldBundle::new(),
            BuiltinFeature::VolcanicSoil => YieldBundle::new().with(crate::YieldType::Food, 3),
            BuiltinFeature::Oasis        => YieldBundle::new()
                .with(crate::YieldType::Food, 3)
                .with(crate::YieldType::Gold, 1),
        }
    }

    pub fn movement_cost_modifier(self) -> MovementCost {
        match self {
            BuiltinFeature::Ice        => MovementCost::Impassable,
            BuiltinFeature::Marsh      => MovementCost::TWO,
            BuiltinFeature::Forest     => MovementCost::ONE,
            BuiltinFeature::Rainforest => MovementCost::ONE,
            BuiltinFeature::Reef       => MovementCost::ONE,
            _                          => MovementCost::ZERO,
        }
    }

    /// Whether this feature hides resources beneath it (until revealed).
    pub fn conceals_resources(self) -> bool {
        matches!(self, BuiltinFeature::Forest | BuiltinFeature::Rainforest)
    }
}
