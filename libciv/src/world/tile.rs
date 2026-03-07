use libhexgrid::board::HexTile;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::{Elevation, MovementCost, Vision};

use super::edge::BuiltinEdgeFeature;
use super::feature::BuiltinFeature;
use super::improvement::BuiltinImprovement;
use super::resource::BuiltinResource;
use super::road::BuiltinRoad;
use super::terrain::BuiltinTerrain;

/// The concrete tile type for the world map.
#[derive(Debug, Clone)]
pub struct WorldTile {
    pub coord: HexCoord,
    pub terrain: BuiltinTerrain,
    pub feature: Option<BuiltinFeature>,
    pub resource: Option<BuiltinResource>,
    pub improvement: Option<BuiltinImprovement>,
    /// True while the improvement has been pillaged and not yet repaired.
    pub improvement_pillaged: bool,
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
            improvement: None,
            improvement_pillaged: false,
            road: None,
            rivers: Vec::new(),
            owner: None,
        }
    }

    /// Sum all yield sources on this tile:
    /// terrain base + feature modifier + improvement bonus (unless pillaged) + resource base.
    ///
    /// Note: resource tech-gating is the caller's responsibility — callers that
    /// know a civ's researched techs should skip resource yields when
    /// `resource.reveal_tech()` returns a tech the civ has not yet researched.
    pub fn total_yields(&self) -> crate::YieldBundle {
        let mut yields = self.terrain.as_def().base_yields();

        if let Some(ref feat) = self.feature {
            yields += feat.as_def().yield_modifier();
        }

        if let Some(ref impr) = self.improvement {
            if !self.improvement_pillaged {
                yields += impr.as_def().yield_bonus();
            }
        }

        if let Some(ref res) = self.resource {
            yields += res.as_def().base_yields();
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

    #[test]
    fn test_total_yields_terrain_only() {
        let tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland(Grassland));
        assert_eq!(tile.total_yields().food, 2);
        assert_eq!(tile.total_yields().production, 0);
    }

    #[test]
    fn test_total_yields_with_improvement() {
        use crate::world::improvement::{BuiltinImprovement, Farm};
        let mut tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland(Grassland));
        tile.improvement = Some(BuiltinImprovement::Farm(Farm));
        // Grassland 2 food + Farm 1 food = 3 food
        assert_eq!(tile.total_yields().food, 3);
    }

    #[test]
    fn test_total_yields_pillaged_improvement_excluded() {
        use crate::world::improvement::{BuiltinImprovement, Farm};
        let mut tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland(Grassland));
        tile.improvement = Some(BuiltinImprovement::Farm(Farm));
        tile.improvement_pillaged = true;
        // Pillaged: only terrain yields
        assert_eq!(tile.total_yields().food, 2);
    }

    #[test]
    fn test_total_yields_with_resource() {
        use crate::world::resource::{BuiltinResource, Wheat};
        let mut tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland(Grassland));
        tile.resource = Some(BuiltinResource::Wheat(Wheat));
        // Grassland 2 food + Wheat 1 food = 3 food
        assert_eq!(tile.total_yields().food, 3);
    }

    #[test]
    fn test_total_yields_all_sources() {
        use crate::world::improvement::{BuiltinImprovement, Farm};
        use crate::world::resource::{BuiltinResource, Wheat};
        let mut tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland(Grassland));
        tile.improvement = Some(BuiltinImprovement::Farm(Farm));
        tile.resource    = Some(BuiltinResource::Wheat(Wheat));
        // Grassland 2 + Farm 1 + Wheat 1 = 4 food
        assert_eq!(tile.total_yields().food, 4);
    }
}
