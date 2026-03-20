use crate::{CivId, GreatWorkId};

/// Category of great work — determines which slot types it can occupy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GreatWorkType {
    Writing,
    Art,
    Music,
    Relic,
    Artifact,
}

/// A created great work that generates tourism and culture each turn.
#[derive(Debug, Clone)]
pub struct GreatWork {
    pub id: GreatWorkId,
    pub name: &'static str,
    pub work_type: GreatWorkType,
    pub creator: Option<CivId>,
    /// Base tourism per turn when slotted.
    pub tourism: u32,
    /// Base culture per turn when slotted.
    pub culture: u32,
}

/// What types of great works a slot can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GreatWorkSlotType {
    Writing,
    Art,
    Music,
    /// Accepts any great work type.
    Any,
}

impl GreatWorkSlotType {
    /// Returns true if this slot can hold a work of the given type.
    pub fn accepts(&self, work_type: GreatWorkType) -> bool {
        match self {
            Self::Any => true,
            Self::Writing => work_type == GreatWorkType::Writing,
            Self::Art => work_type == GreatWorkType::Art,
            Self::Music => work_type == GreatWorkType::Music,
        }
    }
}

/// A slot in a city that can hold one great work.
#[derive(Debug, Clone)]
pub struct GreatWorkSlot {
    pub slot_type: GreatWorkSlotType,
    pub work: Option<GreatWorkId>,
}

impl GreatWorkSlot {
    pub fn new(slot_type: GreatWorkSlotType) -> Self {
        Self { slot_type, work: None }
    }

    pub fn is_empty(&self) -> bool {
        self.work.is_none()
    }
}
