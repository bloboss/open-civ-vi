use crate::{CivId, YieldBundle};

/// The functional category of a city-state, determining its suzerain bonus type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// City-state–specific data stored inside a `City` with `CityKind::CityState`.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CityStateData {
    pub state_type: CityStateType,
    /// The civilization currently holding suzerain status (most influence envoys).
    pub suzerain: Option<CivId>,
    /// Influence points per civilization; used to determine suzerain.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_hashmap_as_vec"))]
    pub influence: std::collections::HashMap<CivId, i32>,
}

impl CityStateData {
    pub fn new(state_type: CityStateType) -> Self {
        Self {
            state_type,
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

    /// Recalculate and cache the suzerain from current influence totals.
    pub fn recalculate_suzerain(&mut self) {
        self.suzerain = self
            .influence
            .iter()
            .max_by_key(|&(_, v)| v)
            .map(|(k, _)| *k);
    }
}
