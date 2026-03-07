use crate::YieldBundle;
use libhexgrid::types::MovementCost;

pub trait FeatureDef: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn yield_modifier(&self) -> YieldBundle;
    fn movement_cost_modifier(&self) -> MovementCost;
    /// Whether this feature hides resources beneath it (until revealed).
    fn conceals_resources(&self) -> bool { false }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Forest;
impl FeatureDef for Forest {
    fn name(&self) -> &'static str { "Forest" }
    fn yield_modifier(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Production, 1)
    }
    fn movement_cost_modifier(&self) -> MovementCost { MovementCost::ONE }
    fn conceals_resources(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rainforest;
impl FeatureDef for Rainforest {
    fn name(&self) -> &'static str { "Rainforest" }
    fn yield_modifier(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 1)
    }
    fn movement_cost_modifier(&self) -> MovementCost { MovementCost::ONE }
    fn conceals_resources(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Marsh;
impl FeatureDef for Marsh {
    fn name(&self) -> &'static str { "Marsh" }
    fn yield_modifier(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 1)
    }
    fn movement_cost_modifier(&self) -> MovementCost { MovementCost::TWO }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Floodplain;
impl FeatureDef for Floodplain {
    fn name(&self) -> &'static str { "Floodplain" }
    fn yield_modifier(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 3)
    }
    fn movement_cost_modifier(&self) -> MovementCost { MovementCost::ZERO }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Reef;
impl FeatureDef for Reef {
    fn name(&self) -> &'static str { "Reef" }
    fn yield_modifier(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 1)
            .with(crate::YieldType::Production, 1)
    }
    fn movement_cost_modifier(&self) -> MovementCost { MovementCost::ONE }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Ice;
impl FeatureDef for Ice {
    fn name(&self) -> &'static str { "Ice" }
    fn yield_modifier(&self) -> YieldBundle { YieldBundle::new() }
    fn movement_cost_modifier(&self) -> MovementCost { MovementCost::Impassable }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VolcanicSoil;
impl FeatureDef for VolcanicSoil {
    fn name(&self) -> &'static str { "Volcanic Soil" }
    fn yield_modifier(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 3)
    }
    fn movement_cost_modifier(&self) -> MovementCost { MovementCost::ZERO }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Oasis;
impl FeatureDef for Oasis {
    fn name(&self) -> &'static str { "Oasis" }
    fn yield_modifier(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 3)
            .with(crate::YieldType::Gold, 1)
    }
    fn movement_cost_modifier(&self) -> MovementCost { MovementCost::ZERO }
}

use libhexgrid::types::MovementCost as MC;

/// Enum wrapping built-in features.
#[derive(Debug, Clone, Copy)]
pub enum BuiltinFeature {
    Forest(Forest),
    Rainforest(Rainforest),
    Marsh(Marsh),
    Floodplain(Floodplain),
    Reef(Reef),
    Ice(Ice),
    VolcanicSoil(VolcanicSoil),
    Oasis(Oasis),
}

impl BuiltinFeature {
    pub fn as_def(&self) -> &dyn FeatureDef {
        match self {
            BuiltinFeature::Forest(f) => f,
            BuiltinFeature::Rainforest(f) => f,
            BuiltinFeature::Marsh(f) => f,
            BuiltinFeature::Floodplain(f) => f,
            BuiltinFeature::Reef(f) => f,
            BuiltinFeature::Ice(f) => f,
            BuiltinFeature::VolcanicSoil(f) => f,
            BuiltinFeature::Oasis(f) => f,
        }
    }
}

// Suppress unused import warning
const _: MC = MC::ZERO;
