//! Replay recording and playback.
//!
//! `ReplayRecorder` captures the initial game state (as JSON) and per-turn
//! diffs. `ReplayViewer` reconstructs any turn's state by loading the initial
//! snapshot and applying diffs 0..n via `apply_diff`.
//!
//! Both types are gated behind the `serde` feature since they depend on
//! `save_game` / `load_game`.

#[cfg(feature = "serde")]
use super::diff::GameStateDiff;
#[cfg(feature = "serde")]
use super::state::GameState;

/// Records a game by saving the initial state and per-turn diffs.
#[cfg(feature = "serde")]
#[derive(Debug)]
pub struct ReplayRecorder {
    /// Serialized initial GameState (JSON string).
    pub initial_state_json: String,
    /// Diffs per turn, in order.
    pub turn_diffs: Vec<GameStateDiff>,
}

#[cfg(feature = "serde")]
impl ReplayRecorder {
    /// Create a new recorder by capturing the current game state.
    pub fn new(state: &GameState) -> Result<Self, String> {
        let json = crate::game::save_load::save_game(state)?;
        Ok(Self {
            initial_state_json: json,
            turn_diffs: Vec::new(),
        })
    }

    /// Record a turn's diff.
    pub fn record_turn(&mut self, diff: GameStateDiff) {
        self.turn_diffs.push(diff);
    }

    /// How many turns have been recorded.
    pub fn turn_count(&self) -> usize {
        self.turn_diffs.len()
    }
}

/// Plays back a recorded game by applying diffs to the initial state.
#[cfg(feature = "serde")]
pub struct ReplayViewer {
    initial_state_json: String,
    diffs: Vec<GameStateDiff>,
    current_turn: usize,
}

#[cfg(feature = "serde")]
impl ReplayViewer {
    /// Create a viewer from a recorder.
    pub fn from_recorder(recorder: &ReplayRecorder) -> Result<Self, String> {
        // Validate that the JSON is loadable.
        let _state = crate::game::save_load::load_game(&recorder.initial_state_json)?;
        Ok(Self {
            initial_state_json: recorder.initial_state_json.clone(),
            diffs: recorder.turn_diffs.clone(),
            current_turn: 0,
        })
    }

    /// Get the current game state (rebuilt by applying diffs up to current_turn).
    ///
    /// Since `GameState` does not implement `Clone`, we reload from JSON each
    /// time and replay diffs 0..current_turn. This is O(n) per call but correct.
    pub fn current_state(&self) -> Result<GameState, String> {
        let mut state = crate::game::save_load::load_game(&self.initial_state_json)?;
        for diff in &self.diffs[..self.current_turn] {
            super::apply_delta::apply_diff(&mut state, diff);
        }
        Ok(state)
    }

    /// Step forward one turn. Returns `true` if the step succeeded.
    pub fn step_forward(&mut self) -> bool {
        if self.current_turn < self.diffs.len() {
            self.current_turn += 1;
            true
        } else {
            false
        }
    }

    /// Step backward one turn. Returns `true` if the step succeeded.
    pub fn step_backward(&mut self) -> bool {
        if self.current_turn > 0 {
            self.current_turn -= 1;
            true
        } else {
            false
        }
    }

    /// Jump to a specific turn. Returns `true` if the turn is in range.
    pub fn jump_to_turn(&mut self, turn: usize) -> bool {
        if turn <= self.diffs.len() {
            self.current_turn = turn;
            true
        } else {
            false
        }
    }

    /// Current turn index (0 = initial state, N = after N diffs applied).
    pub fn turn(&self) -> usize {
        self.current_turn
    }

    /// Total number of recorded turns.
    pub fn total_turns(&self) -> usize {
        self.diffs.len()
    }
}
