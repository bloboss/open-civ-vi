use crate::{ImprovementId, ResourceId};
use libhexgrid::board::HexTile;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::{Elevation, MovementCost, Vision};

use super::edge::BuiltinEdgeFeature;
use super::feature::BuiltinFeature;
use super::road::BuiltinRoad;
use super::terrain::BuiltinTerrain;

/// Context about improvements on a tile (used when computing yields).
#[derive(Debug, Clone, Default)]
pub struct ImprovementContext {
    pub improvement_id: Option<ImprovementId>,
    pub is_pillaged: bool,
}

/// The concrete tile type for the world map.
#[derive(Debug, Clone)]
pub struct WorldTile {
    pub coord: HexCoord,
    pub terrain: BuiltinTerrain,
    pub feature: Option<BuiltinFeature>,
    pub resource: Option<ResourceId>,
    pub improvement: ImprovementContext,
    pub road: Option<BuiltinRoad>,
    pub rivers: Vec<BuiltinEdgeFeature>,
    /// Owning civilization (None = unclaimed).
    pub owner: Option<crate::CivId>,
}

impl WorldTile {
    pub fn new(coord: HexCoord, terrain: BuiltinTerrain) -> Self {
        Self {
            coord,
            terrain,
            feature: None,
            resource: None,
            improvement: ImprovementContext::default(),
            road: None,
            rivers: Vec::new(),
            owner: None,
        }
    }

    pub fn total_yields(&self) -> crate::YieldBundle {
        let mut yields = self.terrain.as_def().base_yields();
        if let Some(ref feat) = self.feature {
            yields += feat.as_def().yield_modifier();
        }
        yields
    }
}

impl HexTile for WorldTile {
    fn coord(&self) -> HexCoord {
        self.coord
    }

    fn elevation(&self) -> Elevation {
        self.terrain.as_def().elevation()
    }

    fn movement_cost(&self) -> MovementCost {
        let base = self.terrain.as_def().movement_cost();
        // Feature can increase cost (Impassable wins)
        if let Some(ref feat) = self.feature {
            let fmod = feat.as_def().movement_cost_modifier();
            match (base, fmod) {
                (MovementCost::Impassable, _) | (_, MovementCost::Impassable) => {
                    MovementCost::Impassable
                }
                (MovementCost::Cost(b), MovementCost::Cost(f)) => MovementCost::Cost(b + f),
            }
        } else {
            base
        }
    }

    fn vision_bonus(&self) -> Vision {
        match self.elevation() {
            Elevation::Level(e) if e >= 1 => Vision::Radius(1),
            Elevation::High => Vision::Radius(1),
            _ => Vision::Radius(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::terrain::Grassland;

    #[test]
    fn test_world_tile_implements_hextile() {
        let coord = HexCoord::from_qr(0, 0);
        let tile = WorldTile::new(coord, BuiltinTerrain::Grassland(Grassland));
        assert_eq!(tile.coord(), coord);
        assert_eq!(tile.movement_cost(), MovementCost::ONE);
    }
}
