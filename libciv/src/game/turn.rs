use super::diff::GameStateDiff;
use super::rules::RulesEngine;
use super::state::GameState;

/// Orchestrates a full game turn.
#[derive(Debug, Default)]
pub struct TurnEngine;

impl TurnEngine {
    pub fn new() -> Self {
        Self
    }

    /// Process all civilization turns (AI + human input stubs) and return aggregate diff.
    pub fn process_turn(
        &self,
        state: &mut GameState,
        rules: &dyn RulesEngine,
    ) -> GameStateDiff {
        let _diff = rules.advance_turn(state);
        // Phase 2: apply diff to state, collect AI decisions, etc.
        GameStateDiff::new()
    }
}
