use crate::{CityId, CivId, TradeRouteId, YieldBundle, YieldType};
use crate::civ::City;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Returns `true` when origin and destination belong to different civilizations.
    pub fn is_international(&self, cities: &[City]) -> bool {
        let origin_owner = cities.iter().find(|c| c.id == self.origin).map(|c| c.owner);
        let dest_owner   = cities.iter().find(|c| c.id == self.destination).map(|c| c.owner);
        matches!((origin_owner, dest_owner), (Some(a), Some(b)) if a != b)
    }
}

/// Returns `(origin_yields, destination_yields)` per-turn for a trade route.
///
/// Yields are civ-level (gold only). Per-city food/production delivery is a
/// future TODO(PHASE3-8.4-FOOD).
pub fn compute_route_yields(is_international: bool) -> (YieldBundle, YieldBundle) {
    if is_international {
        (
            YieldBundle::default().with(YieldType::Gold, 6),
            YieldBundle::default().with(YieldType::Gold, 4),
        )
    } else {
        (
            YieldBundle::default().with(YieldType::Gold, 3),
            YieldBundle::default().with(YieldType::Gold, 1),
        )
    }
}
