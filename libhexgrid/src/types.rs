/// Movement cost for crossing a hex tile or edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MovementCost {
    /// Tiles/edges that cannot be crossed.
    Impassable,
    /// Flat cost in movement-point units (scaled by 100 for integer math).
    Cost(u32),
}

impl MovementCost {
    pub const ZERO: MovementCost = MovementCost::Cost(0);
    pub const ONE: MovementCost = MovementCost::Cost(100);
    pub const TWO: MovementCost = MovementCost::Cost(200);
    pub const THREE: MovementCost = MovementCost::Cost(300);

    pub fn is_passable(self) -> bool {
        matches!(self, MovementCost::Cost(_))
    }

    pub fn as_u32(self) -> Option<u32> {
        match self {
            MovementCost::Cost(v) => Some(v),
            MovementCost::Impassable => None,
        }
    }
}

/// Elevation level of a tile (affects LOS and movement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Elevation(pub u8);

impl Elevation {
    pub const FLAT: Elevation = Elevation(0);
    pub const HILLS: Elevation = Elevation(1);
    pub const MOUNTAIN: Elevation = Elevation(2);
}

/// Vision range a unit has from this tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Vision(pub u8);

/// Movement profile of a unit — determines which tiles/edges it can cross.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MovementProfile {
    /// Standard land movement.
    Ground,
    /// Naval movement (sea tiles only, cannot enter land without port).
    Naval,
    /// Air movement (ignores terrain movement costs).
    Air,
    /// Embarkation (land unit on water with reduced movement).
    Embarked,
}
