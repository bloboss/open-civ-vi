use crate::{CivId, VictoryId};

#[derive(Debug, Clone)]
pub struct VictoryProgress {
    pub victory_id: VictoryId,
    pub civ_id: CivId,
    /// Current progress value (interpretation depends on victory type).
    pub current: u32,
    /// Target value to win.
    pub target: u32,
}

impl VictoryProgress {
    pub fn is_won(&self) -> bool {
        self.current >= self.target
    }

    pub fn percentage(&self) -> f32 {
        if self.target == 0 {
            return 100.0;
        }
        (self.current as f32 / self.target as f32) * 100.0
    }
}

// TODO(PHASE3-8.9): Implement concrete types:
//   struct DominationVictory {}   — controls all original capitals
//   struct ScienceVictory    {}   — specific tech milestone
//   struct CultureVictory    {}   — tourism > all civs' home culture
//   struct DiplomaticVictory {}   — diplomatic favour threshold
//   struct ScoreVictory { turn_limit: u32 }  — highest score at turn limit
// check_progress() needs &GameState, not just CivId.
// Add victory_conditions: Vec<Box<dyn VictoryCondition>> and game_over: bool to GameState.
pub trait VictoryCondition: std::fmt::Debug {
    fn id(&self) -> VictoryId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check_progress(&self, civ_id: CivId) -> VictoryProgress;
}
