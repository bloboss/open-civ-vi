use crate::{BeliefId, CivId, ReligionId, YieldBundle};

pub trait Belief: std::fmt::Debug {
    fn id(&self) -> BeliefId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
}

/// Context for evaluating belief effects.
#[derive(Debug, Clone, Default)]
pub struct BeliefContext {
    pub followers: u32,
    pub holy_cities: u32,
}

#[derive(Debug, Clone)]
pub struct Religion {
    pub id: ReligionId,
    pub name: String,
    pub founded_by: CivId,
    pub holy_city: crate::CityId,
    pub beliefs: Vec<BeliefId>,
    pub followers: std::collections::HashMap<crate::CityId, u32>,
}

impl Religion {
    pub fn new(id: ReligionId, name: String, founder: CivId, holy_city: crate::CityId) -> Self {
        Self {
            id,
            name,
            founded_by: founder,
            holy_city,
            beliefs: Vec::new(),
            followers: std::collections::HashMap::new(),
        }
    }

    pub fn total_followers(&self) -> u32 {
        self.followers.values().sum()
    }
}

/// Yields a religion provides to its founder per turn (placeholder).
pub fn religion_founder_yields(_religion: &Religion) -> YieldBundle {
    YieldBundle::default()
}
