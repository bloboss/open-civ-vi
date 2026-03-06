use libcommon::{CivId, CityId, YieldBundle};
use libhexgrid::coord::HexCoord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CityStateType {
    Cultural,
    Industrial,
    Militaristic,
    Religious,
    Scientific,
    Trade,
}

pub trait CityStateBonus: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn yields_for_suzerain(&self) -> YieldBundle;
}

#[derive(Debug, Clone)]
pub struct CityState {
    pub id: CityId,
    pub name: &'static str,
    pub state_type: CityStateType,
    pub coord: HexCoord,
    pub suzerain: Option<CivId>,
    /// Influence points per civilization.
    pub influence: std::collections::HashMap<CivId, i32>,
}

impl CityState {
    pub fn new(id: CityId, name: &'static str, state_type: CityStateType, coord: HexCoord) -> Self {
        Self {
            id,
            name,
            state_type,
            coord,
            suzerain: None,
            influence: std::collections::HashMap::new(),
        }
    }

    pub fn is_suzerain(&self, civ: CivId) -> bool {
        self.suzerain == Some(civ)
    }

    pub fn get_influence(&self, civ: CivId) -> i32 {
        self.influence.get(&civ).copied().unwrap_or(0)
    }

    pub fn recalculate_suzerain(&mut self) {
        self.suzerain = self.influence.iter().max_by_key(|&(_, v)| v).map(|(k, _)| *k);
    }
}
