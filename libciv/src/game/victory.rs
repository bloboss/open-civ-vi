use crate::{CivId, VictoryId};
use crate::rules::VictoryProgress;
use super::score::compute_score;
use super::state::GameState;

/// Determines how and when a victory condition is evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VictoryKind {
    /// The first civ to satisfy the condition wins immediately (e.g. Domination, Science).
    /// Evaluated every turn.
    ImmediateWin,
    /// Only evaluated once `state.turn >= turn_limit`; the highest-scoring civ wins.
    TurnLimit { turn_limit: u32 },
}

/// Returned by `advance_turn` when a victory has been declared.
#[derive(Debug, Clone)]
pub struct GameOver {
    pub winner: CivId,
    /// Display name of the winning condition (e.g. `"Score Victory"`).
    pub condition: &'static str,
    pub turn: u32,
}

/// Generic victory condition interface.
///
/// Implement this trait to add new victory types. Register instances on
/// `GameState::victory_conditions` before the game loop starts.
///
/// # Evaluation order
/// `advance_turn` checks `ImmediateWin` conditions each turn (first match wins),
/// then checks `TurnLimit` conditions when the turn limit is reached (highest scorer wins).
pub trait VictoryCondition: std::fmt::Debug + Send + Sync {
    fn id(&self) -> VictoryId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn kind(&self) -> VictoryKind;
    /// Returns the current progress for `civ_id`. `is_won()` signals an immediate win
    /// for `ImmediateWin` conditions; for `TurnLimit` conditions the score is used
    /// by the engine to pick the winner when the limit is reached.
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress;
}

// ── Score Victory ─────────────────────────────────────────────────────────────

/// Score-based victory: the civilization with the highest score when the turn
/// limit is reached wins. Score is computed by `game::score::compute_score`.
#[derive(Debug)]
pub struct ScoreVictory {
    pub id: VictoryId,
    /// The game ends on this turn (inclusive).
    pub turn_limit: u32,
}

impl VictoryCondition for ScoreVictory {
    fn id(&self) -> VictoryId { self.id }
    fn name(&self) -> &'static str { "Score Victory" }
    fn description(&self) -> &'static str {
        "The civilization with the highest score when the turn limit is reached wins."
    }
    fn kind(&self) -> VictoryKind { VictoryKind::TurnLimit { turn_limit: self.turn_limit } }
    fn check_progress(&self, civ_id: CivId, state: &GameState) -> VictoryProgress {
        VictoryProgress {
            victory_id: self.id,
            civ_id,
            // `current` carries the score; `target` is the turn limit for display.
            current: compute_score(state, civ_id),
            target: self.turn_limit,
        }
    }
}
