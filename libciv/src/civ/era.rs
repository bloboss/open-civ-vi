use crate::{AgeType, EraId};

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
