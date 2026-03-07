use libhexgrid::board::HexEdge;
use libhexgrid::coord::HexCoord;
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
#[derive(Debug, Clone)]
pub struct WorldEdge {
    pub from: HexCoord,
    pub to: HexCoord,
    pub feature: Option<BuiltinEdgeFeature>,
}

impl WorldEdge {
    pub fn new(from: HexCoord, to: HexCoord) -> Self {
        Self { from, to, feature: None }
    }

    pub fn with_feature(mut self, feature: BuiltinEdgeFeature) -> Self {
        self.feature = Some(feature);
        self
    }
}

impl HexEdge for WorldEdge {
    fn endpoints(&self) -> (HexCoord, HexCoord) {
        (self.from, self.to)
    }

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
        let from = HexCoord::from_qr(0, 0);
        let to = HexCoord::from_qr(1, 0);
        let edge = WorldEdge::new(from, to).with_feature(BuiltinEdgeFeature::River(River));
        assert_eq!(edge.crossing_cost(), MovementCost::THREE);
    }
}
