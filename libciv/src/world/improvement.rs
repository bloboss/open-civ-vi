use crate::YieldBundle;

pub trait TileImprovement: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn yield_bonus(&self) -> YieldBundle;
    /// Number of turns to build (base, without modifiers).
    fn build_turns(&self) -> u32;
    fn pillaged(&self) -> bool { false }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Farm;
impl TileImprovement for Farm {
    fn name(&self) -> &'static str { "Farm" }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Food, 1)
    }
    fn build_turns(&self) -> u32 { 5 }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Mine;
impl TileImprovement for Mine {
    fn name(&self) -> &'static str { "Mine" }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Production, 1)
    }
    fn build_turns(&self) -> u32 { 5 }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LumberMill;
impl TileImprovement for LumberMill {
    fn name(&self) -> &'static str { "Lumber Mill" }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Production, 2)
    }
    fn build_turns(&self) -> u32 { 5 }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TradingPost;
impl TileImprovement for TradingPost {
    fn name(&self) -> &'static str { "Trading Post" }
    fn yield_bonus(&self) -> YieldBundle {
        YieldBundle::new().with(crate::YieldType::Gold, 1)
    }
    fn build_turns(&self) -> u32 { 5 }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Fort;
impl TileImprovement for Fort {
    fn name(&self) -> &'static str { "Fort" }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn build_turns(&self) -> u32 { 10 }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Airstrip;
impl TileImprovement for Airstrip {
    fn name(&self) -> &'static str { "Airstrip" }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn build_turns(&self) -> u32 { 10 }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MissileSilo;
impl TileImprovement for MissileSilo {
    fn name(&self) -> &'static str { "Missile Silo" }
    fn yield_bonus(&self) -> YieldBundle { YieldBundle::new() }
    fn build_turns(&self) -> u32 { 15 }
}
