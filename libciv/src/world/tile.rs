use libhexgrid::board::HexTile;
use libhexgrid::coord::HexCoord;
use libhexgrid::types::{Elevation, MovementCost, Vision};

use super::edge::BuiltinEdgeFeature;
use super::feature::BuiltinFeature;
use super::improvement::BuiltinImprovement;
use super::resource::BuiltinResource;
use super::road::BuiltinRoad;
use super::terrain::BuiltinTerrain;
use super::wonder::BuiltinNaturalWonder;

/// The concrete tile type for the world map.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldTile {
    pub coord: HexCoord,
    pub terrain: BuiltinTerrain,
    /// When `true`, elevation is raised by 1 (e.g. Grassland Hills = `Level(2)`).
    pub hills: bool,
    pub feature: Option<BuiltinFeature>,
    pub resource: Option<BuiltinResource>,
    pub improvement: Option<BuiltinImprovement>,
    /// True while the improvement has been pillaged and not yet repaired.
    pub improvement_pillaged: bool,
    pub road: Option<BuiltinRoad>,
    pub rivers: Vec<BuiltinEdgeFeature>,
    /// Natural wonder on this tile, if any. Overrides feature/improvement yield logic.
    pub natural_wonder: Option<BuiltinNaturalWonder>,
    /// Owning civilization (None = unclaimed).
    pub owner: Option<crate::CivId>,
    /// Coastal lowland elevation category (1, 2, or 3 meters above sea level).
    /// `None` means this tile is not a coastal lowland. Tagged during mapgen or
    /// manually for testing.
    #[cfg_attr(feature = "serde", serde(default))]
    pub coastal_lowland: Option<u8>,
    /// True if this tile has been submerged by sea level rise.
    #[cfg_attr(feature = "serde", serde(default))]
    pub submerged: bool,
}

impl WorldTile {
    pub fn new(coord: HexCoord, terrain: BuiltinTerrain) -> Self {
        Self {
            coord,
            terrain,
            hills: false,
            feature: None,
            resource: None,
            improvement: None,
            improvement_pillaged: false,
            road: None,
            rivers: Vec::new(),
            natural_wonder: None,
            owner: None,
            coastal_lowland: None,
            submerged: false,
        }
    }

    /// Flat combat-strength defense bonus for a unit defending on this tile.
    ///
    /// Bonuses are additive: Forest on Hills = +6.
    /// Returns a negative value for penalty terrain (Marsh = -2).
    /// Natural wonder tiles return 0 (wonders are not defensive positions).
    pub fn terrain_defense_bonus(&self) -> i32 {
        if self.natural_wonder.is_some() {
            return 0;
        }

        let mut bonus = 0i32;

        // Hills give +3 (terrain elevation bump).
        if self.hills {
            bonus += 3;
        }

        // Feature bonuses.
        if let Some(feat) = self.feature {
            bonus += match feat {
                super::feature::BuiltinFeature::Forest     => 3,
                super::feature::BuiltinFeature::Rainforest => 3,
                super::feature::BuiltinFeature::Marsh      => -2,
                _ => 0,
            };
        }

        bonus
    }

    /// Sum all yield sources on this tile:
    /// terrain base + feature modifier + improvement bonus (unless pillaged) + resource base.
    ///
    /// When a natural wonder is present its yield bonus is added on top of
    /// terrain yields; feature and improvement bonuses are suppressed for
    /// wonder tiles (but resource tech-gating is still the caller's responsibility).
    pub fn total_yields(&self) -> crate::YieldBundle {
        let mut yields = self.terrain.base_yields();

        if let Some(ref wonder) = self.natural_wonder {
            yields += wonder.as_def().yield_bonus();
            // Feature/improvement bonuses do not apply to wonder tiles.
            return yields;
        }

        if let Some(ref feat) = self.feature {
            yields += feat.yield_modifier();
        }

        if let Some(impr) = self.improvement
            && !self.improvement_pillaged
        {
            yields += impr.yield_bonus();
        }

        if let Some(res) = self.resource {
            yields += res.base_yields();
        }

        yields
    }
}

impl HexTile for WorldTile {
    fn coord(&self) -> HexCoord {
        self.coord
    }

    fn elevation(&self) -> Elevation {
        let base = self.terrain.elevation();
        if self.hills {
            match base {
                Elevation::Level(n) => Elevation::Level(n + 1),
                other => other,  // High stays High, Low stays Low
            }
        } else {
            base
        }
    }

    fn movement_cost(&self) -> MovementCost {
        // Natural wonder overrides movement cost (may be Impassable).
        if let Some(ref wonder) = self.natural_wonder {
            return wonder.as_def().movement_cost();
        }

        let base = self.terrain.movement_cost();
        // Feature can increase cost (Impassable wins)
        if let Some(ref feat) = self.feature {
            let fmod = feat.movement_cost_modifier();
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
            Elevation::Level(e) if e >= 2 => Vision::Radius(1),  // hills = Level(2)+
            Elevation::High => Vision::Radius(1),
            _ => Vision::Radius(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_tile_implements_hextile() {
        let coord = HexCoord::from_qr(0, 0);
        let tile = WorldTile::new(coord, BuiltinTerrain::Grassland);
        assert_eq!(tile.coord(), coord);
        assert_eq!(tile.movement_cost(), MovementCost::ONE);
    }

    #[test]
    fn test_total_yields_terrain_only() {
        let tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland);
        assert_eq!(tile.total_yields().food, 2);
        assert_eq!(tile.total_yields().production, 0);
    }

    #[test]
    fn test_total_yields_with_improvement() {
        use crate::world::improvement::BuiltinImprovement;
        let mut tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland);
        tile.improvement = Some(BuiltinImprovement::Farm);
        // Grassland 2 food + Farm 1 food = 3 food
        assert_eq!(tile.total_yields().food, 3);
    }

    #[test]
    fn test_total_yields_pillaged_improvement_excluded() {
        use crate::world::improvement::BuiltinImprovement;
        let mut tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland);
        tile.improvement = Some(BuiltinImprovement::Farm);
        tile.improvement_pillaged = true;
        // Pillaged: only terrain yields
        assert_eq!(tile.total_yields().food, 2);
    }

    #[test]
    fn test_total_yields_with_resource() {
        use crate::world::resource::BuiltinResource;
        let mut tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland);
        tile.resource = Some(BuiltinResource::Wheat);
        // Grassland 2 food + Wheat 1 food = 3 food
        assert_eq!(tile.total_yields().food, 3);
    }

    #[test]
    fn test_total_yields_all_sources() {
        use crate::world::improvement::BuiltinImprovement;
        use crate::world::resource::BuiltinResource;
        let mut tile = WorldTile::new(HexCoord::from_qr(0, 0), BuiltinTerrain::Grassland);
        tile.improvement = Some(BuiltinImprovement::Farm);
        tile.resource    = Some(BuiltinResource::Wheat);
        // Grassland 2 + Farm 1 + Wheat 1 = 4 food
        assert_eq!(tile.total_yields().food, 4);
    }
}
