use crate::{CityId, CivId, TradeRouteId, YieldBundle};

#[derive(Debug, Clone)]
pub struct TradeRoute {
    pub id: TradeRouteId,
    pub origin: CityId,
    pub destination: CityId,
    pub owner: CivId,
    /// Yields per turn delivered to origin city.
    pub origin_yields: YieldBundle,
    /// Yields per turn delivered to destination city.
    pub destination_yields: YieldBundle,
    pub turns_remaining: Option<u32>,
}

impl TradeRoute {
    pub fn new(id: TradeRouteId, origin: CityId, destination: CityId, owner: CivId) -> Self {
        Self {
            id,
            origin,
            destination,
            owner,
            origin_yields: YieldBundle::default(),
            destination_yields: YieldBundle::default(),
            turns_remaining: None,
        }
    }

    pub fn is_international(&self) -> bool {
        // Will be determined by comparing owning civs of cities in Phase 2.
        false
    }
}
