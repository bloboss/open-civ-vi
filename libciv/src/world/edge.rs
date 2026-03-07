use libhexgrid::board::HexEdge;
use libhexgrid::coord::{HexCoord, HexDir};
use libhexgrid::types::MovementCost;

pub trait EdgeFeatureDef: std::fmt::Debug {
    fn name(&self) -> &'static str;
    /// Additional movement cost when crossing this edge.
    fn crossing_cost(&self) -> MovementCost;
    /// Whether this edge blocks line-of-sight.
    fn blocks_los(&self) -> bool { false }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct River;
impl EdgeFeatureDef for River {
    fn name(&self) -> &'static str { "River" }
    fn crossing_cost(&self) -> MovementCost { MovementCost::THREE }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Cliff;
impl EdgeFeatureDef for Cliff {
    fn name(&self) -> &'static str { "Cliff" }
    fn crossing_cost(&self) -> MovementCost { MovementCost::Impassable }
    fn blocks_los(&self) -> bool { true }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Canal;
impl EdgeFeatureDef for Canal {
    fn name(&self) -> &'static str { "Canal" }
    fn crossing_cost(&self) -> MovementCost { MovementCost::ONE }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MountainPass;
impl EdgeFeatureDef for MountainPass {
    fn name(&self) -> &'static str { "Mountain Pass" }
    fn crossing_cost(&self) -> MovementCost { MovementCost::TWO }
}

/// Enum of built-in edge features.
#[derive(Debug, Clone, Copy)]
pub enum BuiltinEdgeFeature {
    River(River),
    Cliff(Cliff),
    Canal(Canal),
    MountainPass(MountainPass),
}

impl BuiltinEdgeFeature {
    pub fn as_def(&self) -> &dyn EdgeFeatureDef {
        match self {
            BuiltinEdgeFeature::River(e) => e,
            BuiltinEdgeFeature::Cliff(e) => e,
            BuiltinEdgeFeature::Canal(e) => e,
            BuiltinEdgeFeature::MountainPass(e) => e,
        }
    }
}

/// Concrete edge type for the world board.
///
/// Stored in canonical form: `dir` is always in the forward half {E, NE, NW}.
/// The backward directions {W, SW, SE} are normalised by `WorldBoard::canonical`.
#[derive(Debug, Clone)]
pub struct WorldEdge {
    /// Canonical tile coordinate (forward-half endpoint).
    pub coord: HexCoord,
    /// Direction from `coord` toward the other tile (always forward-half: E, NE, or NW).
    pub dir: HexDir,
    pub feature: Option<BuiltinEdgeFeature>,
}

impl WorldEdge {
    /// Construct an edge in canonical form. Panics in debug if `dir` is not forward-half.
    pub fn new(coord: HexCoord, dir: HexDir) -> Self {
        debug_assert!(
            matches!(dir, HexDir::E | HexDir::NE | HexDir::NW),
            "WorldEdge::new requires a forward-half direction (E, NE, NW); use WorldBoard::set_edge for automatic canonicalization"
        );
        Self { coord, dir, feature: None }
    }

    pub fn with_feature(mut self, feature: BuiltinEdgeFeature) -> Self {
        self.feature = Some(feature);
        self
    }
}

impl HexEdge for WorldEdge {
    fn coord(&self) -> HexCoord { self.coord }
    fn dir(&self) -> HexDir { self.dir }

    fn crossing_cost(&self) -> MovementCost {
        self.feature
            .as_ref()
            .map(|f| f.as_def().crossing_cost())
            .unwrap_or(MovementCost::Cost(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_edge_river_cost() {
        let coord = HexCoord::from_qr(0, 0);
        let edge = WorldEdge::new(coord, HexDir::E).with_feature(BuiltinEdgeFeature::River(River));
        assert_eq!(edge.crossing_cost(), MovementCost::THREE);
    }
}
