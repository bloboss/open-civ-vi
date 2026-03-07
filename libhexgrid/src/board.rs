use crate::coord::HexCoord;
use crate::types::{Elevation, MovementCost, Vision};

/// A tile on the hex board.
pub trait HexTile {
    fn coord(&self) -> HexCoord;
    fn elevation(&self) -> Elevation;
    fn movement_cost(&self) -> MovementCost;
    fn vision_bonus(&self) -> Vision;
}

/// An edge between two adjacent tiles, identified by a tile coordinate and the
/// direction from that tile toward its neighbour. The canonical form always uses
/// a direction from the forward half {E, NE, NW}; the backward half {W, SW, SE}
/// is normalised by stepping to the adjacent tile and flipping the direction.
pub trait HexEdge {
    /// The tile coordinate of the canonical (forward-half) endpoint.
    fn coord(&self) -> HexCoord;
    /// The direction from `coord` toward the other tile (always forward-half).
    fn dir(&self) -> crate::coord::HexDir;
    /// Both tile coordinates separated by this edge, derived from coord + dir.
    fn endpoints(&self) -> (HexCoord, HexCoord) {
        (self.coord(), self.coord() + self.dir().unit_vec())
    }
    /// Additional movement cost imposed by crossing this edge (e.g., river penalty).
    fn crossing_cost(&self) -> MovementCost;
}

/// Topology describing how the board wraps (or doesn't).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardTopology {
    /// Finite flat board — coords outside bounds are invalid.
    Flat,
    /// Wraps east-west (cylindrical map).
    CylindricalEW,
    /// Wraps in both axes (toroidal).
    Toroidal,
}

/// A hex board parameterised by tile and edge types.
pub trait HexBoard {
    type Tile: HexTile;
    type Edge: HexEdge;

    fn topology(&self) -> BoardTopology;

    fn width(&self) -> u32;
    fn height(&self) -> u32;

    fn tile(&self, coord: HexCoord) -> Option<&Self::Tile>;
    fn tile_mut(&mut self, coord: HexCoord) -> Option<&mut Self::Tile>;

    fn edge(&self, coord: HexCoord, dir: crate::coord::HexDir) -> Option<&Self::Edge>;

    fn neighbors(&self, coord: HexCoord) -> Vec<HexCoord>;

    /// Normalise a coordinate according to board topology (wrap-around).
    fn normalize(&self, coord: HexCoord) -> Option<HexCoord>;

    fn all_coords(&self) -> Vec<HexCoord>;
}

#[cfg(test)]
mod tests {
    // Board trait tests are written in libgame where a concrete impl exists.
    // Placeholder so the module compiles cleanly.
    #[test]
    fn board_module_compiles() {}
}
