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
///
/// Ordering: `Low < Level(0) < Level(1) < … < Level(255) < High`
/// - `Low`  — below sea level (ocean floor); never blocks LOS.
/// - `Level(n)` — above-sea-level terrain; higher n = higher ground.
/// - `High` — impassable mountain peak; always blocks LOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Elevation {
    Low,
    Level(u8),
    High,
}

impl Elevation {
    /// Deep open-water (Ocean terrain) — below sea level.
    pub const SEA_LEVEL: Elevation = Elevation::Low;
    /// Near-shore coastal terrain (Coast terrain). Normal diff from ocean = 1.
    pub const COASTAL: Elevation = Elevation::Level(0);
    /// Default flat inland terrain (Grassland, Plains, Desert, Tundra, Snow).
    pub const FLAT: Elevation = Elevation::Level(1);
    /// Hills: flat inland + 1 level of elevation.
    pub const HILLS: Elevation = Elevation::Level(2);
    /// Impassable mountain peak — always blocks LOS.
    pub const MOUNTAIN: Elevation = Elevation::High;
}

impl Default for Elevation {
    fn default() -> Self {
        Elevation::FLAT
    }
}

/// Vision range a unit or tile can provide.
///
/// - `Blind`       — cannot see anything (e.g. embarked unit in fog).
/// - `Radius(n)`   — can see tiles within n hexes (subject to LOS).
/// - `Omniscient`  — sees the entire map regardless of distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vision {
    Blind,
    Radius(u8),
    Omniscient,
}

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
    /// Unit that moves on both land and water at full efficiency (e.g. Giant Death Robot).
    /// Uses land movement costs on land tiles and naval costs on water tiles.
    Amphibious,
}
