use crate::CivId;
use std::collections::HashMap;

/// Periodic global assembly where civs propose and vote on resolutions using
/// diplomatic favor. Replaces the simple favor-threshold diplomatic victory
/// with a Congress-based system (GS-3).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorldCongress {
    /// Turns between sessions (default 30).
    pub session_interval: u32,
    /// The turn number when the next session fires.
    pub next_session_turn: u32,
    /// Resolutions currently in effect.
    pub active_resolutions: Vec<ActiveResolution>,
    /// Cumulative diplomatic victory points per civilization.
    /// GS-14 revises the Diplomatic victory condition to use these VPs
    /// (threshold 20) instead of the simple favor-threshold check.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_hashmap_as_vec"))]
    pub diplomatic_victory_points: HashMap<CivId, u32>,
}

impl Default for WorldCongress {
    fn default() -> Self {
        Self {
            session_interval: 30,
            next_session_turn: 30,
            active_resolutions: Vec::new(),
            diplomatic_victory_points: HashMap::new(),
        }
    }
}

/// A resolution that has been voted on and is currently active.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActiveResolution {
    pub kind: ResolutionKind,
    pub proposed_by: CivId,
    pub passed: bool,
    pub turns_remaining: u32,
}

/// The type of resolution being voted on. This is a simplified subset;
/// the full 23-variant set from the XML data will be added in a future pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResolutionKind {
    DiplomaticVictory,
    TradeBonus,
    MilitaryBonus,
    ScienceBonus,
    CultureBonus,
    FaithBonus,
}
