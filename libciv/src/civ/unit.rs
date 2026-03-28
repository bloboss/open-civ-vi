use crate::{CityId, CivId, PromotionId, UnitCategory, UnitDomain, UnitId, UnitTypeId};
use libhexgrid::coord::HexCoord;

pub trait Unit: std::fmt::Debug {
    fn id(&self) -> UnitId;
    fn unit_type(&self) -> UnitTypeId;
    fn owner(&self) -> CivId;
    fn coord(&self) -> HexCoord;
    fn domain(&self) -> UnitDomain;
    fn category(&self) -> UnitCategory;
    fn movement_left(&self) -> u32;
    fn max_movement(&self) -> u32;
    fn combat_strength(&self) -> Option<u32>;
    fn promotions(&self) -> &[PromotionId];
    fn health(&self) -> u32;
    fn max_health(&self) -> u32 { 100 }
    fn is_alive(&self) -> bool { self.health() > 0 }
}

/// A concrete simple unit implementation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasicUnit {
    pub id: UnitId,
    pub unit_type: UnitTypeId,
    pub owner: CivId,
    pub coord: HexCoord,
    pub domain: UnitDomain,
    pub category: UnitCategory,
    pub movement_left: u32,
    pub max_movement: u32,
    pub combat_strength: Option<u32>,
    pub promotions: Vec<PromotionId>,
    pub health: u32,
    /// Melee = 0; ranged units have range > 0 (tiles). Distinct from vision_range.
    pub range: u8,
    /// How many hexes this unit can see. Typically 2 for land units.
    pub vision_range: u8,
    /// Remaining build charges for builder-type units. `None` for units
    /// without charges (warriors, settlers, etc.). When this reaches
    /// `Some(0)` the unit should be destroyed.
    pub charges: Option<u8>,
    /// For trader units: the origin city from which the trade route starts.
    /// Set by `assign_trade_route`; cleared when the route is established.
    pub trade_origin: Option<CityId>,
    /// For trader units: the destination city the trader is moving toward.
    /// Set by `assign_trade_route`; cleared when the route is established.
    pub trade_destination: Option<CityId>,
}

impl Unit for BasicUnit {
    fn id(&self) -> UnitId { self.id }
    fn unit_type(&self) -> UnitTypeId { self.unit_type }
    fn owner(&self) -> CivId { self.owner }
    fn coord(&self) -> HexCoord { self.coord }
    fn domain(&self) -> UnitDomain { self.domain }
    fn category(&self) -> UnitCategory { self.category }
    fn movement_left(&self) -> u32 { self.movement_left }
    fn max_movement(&self) -> u32 { self.max_movement }
    fn combat_strength(&self) -> Option<u32> { self.combat_strength }
    fn promotions(&self) -> &[PromotionId] { &self.promotions }
    fn health(&self) -> u32 { self.health }
}
