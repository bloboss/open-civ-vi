use libcommon::{CivId, VictoryId};

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

pub trait VictoryCondition: std::fmt::Debug {
    fn id(&self) -> VictoryId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check_progress(&self, civ_id: CivId) -> VictoryProgress;
}
