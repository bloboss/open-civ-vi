use crate::{AgeType, EraId};

// TODO(PHASE3-8.8): Add concrete implementations: TechEraThreshold { required_techs: u32 },
//   CivicEraThreshold { required_civics: u32 }. is_triggered() needs &GameState + CivId
//   to count completed techs/civics. Store era_triggers: Vec<Box<dyn EraTrigger>> in GameState.
pub trait EraTrigger: std::fmt::Debug {
    fn description(&self) -> &'static str;
    /// Returns true if the era transition condition is met.
    fn is_triggered(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct Era {
    pub id: EraId,
    pub name: &'static str,
    pub age: AgeType,
    pub tech_count: u32,
    pub civic_count: u32,
}

impl Era {
    pub fn new(id: EraId, name: &'static str, age: AgeType) -> Self {
        Self {
            id,
            name,
            age,
            tech_count: 0,
            civic_count: 0,
        }
    }
}
