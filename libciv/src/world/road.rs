use libhexgrid::types::MovementCost;

pub trait RoadDef: std::fmt::Debug {
    fn name(&self) -> &'static str;
    /// Movement cost when travelling *along* this road.
    fn movement_cost(&self) -> MovementCost;
    /// Gold maintenance per turn.
    fn maintenance(&self) -> u32;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AncientRoad;
impl RoadDef for AncientRoad {
    fn name(&self) -> &'static str { "Ancient Road" }
    fn movement_cost(&self) -> MovementCost { MovementCost::Cost(50) }
    fn maintenance(&self) -> u32 { 0 }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MedievalRoad;
impl RoadDef for MedievalRoad {
    fn name(&self) -> &'static str { "Medieval Road" }
    fn movement_cost(&self) -> MovementCost { MovementCost::Cost(50) }
    fn maintenance(&self) -> u32 { 1 }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndustrialRoad;
impl RoadDef for IndustrialRoad {
    fn name(&self) -> &'static str { "Industrial Road" }
    fn movement_cost(&self) -> MovementCost { MovementCost::Cost(25) }
    fn maintenance(&self) -> u32 { 2 }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Railroad;
impl RoadDef for Railroad {
    fn name(&self) -> &'static str { "Railroad" }
    fn movement_cost(&self) -> MovementCost { MovementCost::Cost(10) }
    fn maintenance(&self) -> u32 { 3 }
}

/// Enum wrapping all built-in road types (Clone-friendly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinRoad {
    Ancient(AncientRoad),
    Medieval(MedievalRoad),
    Industrial(IndustrialRoad),
    Railroad(Railroad),
}

impl BuiltinRoad {
    pub fn as_def(&self) -> &dyn RoadDef {
        match self {
            BuiltinRoad::Ancient(r) => r,
            BuiltinRoad::Medieval(r) => r,
            BuiltinRoad::Industrial(r) => r,
            BuiltinRoad::Railroad(r) => r,
        }
    }

    /// Returns a numeric tier for ordering: Ancient=0, Medieval=1, Industrial=2, Railroad=3.
    /// Higher tier roads are strictly better. Used to reject downgrades in `place_road()`.
    pub fn tier(&self) -> u8 {
        match self {
            BuiltinRoad::Ancient(_)    => 0,
            BuiltinRoad::Medieval(_)   => 1,
            BuiltinRoad::Industrial(_) => 2,
            BuiltinRoad::Railroad(_)   => 3,
        }
    }

    /// Returns the tech name required to build this road tier, or `None` if no tech is needed.
    pub fn required_tech(&self) -> Option<&'static str> {
        match self {
            BuiltinRoad::Ancient(_)    => None,
            BuiltinRoad::Medieval(_)   => Some("Engineering"),
            BuiltinRoad::Industrial(_) => Some("Steam Power"),
            BuiltinRoad::Railroad(_)   => Some("Railroads"),
        }
    }
}
