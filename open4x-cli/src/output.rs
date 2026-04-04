//! JSON output formatting for CLI commands.

use libciv::game::diff::{GameStateDiff, StateDelta};
use serde::Serialize;

/// Result of a mutating CLI action, emitted as JSON on stdout.
#[derive(Serialize)]
pub struct ActionResult {
    pub success: bool,
    pub turn: u32,
    pub deltas: Vec<StateDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_over: Option<GameOverInfo>,
}

/// Minimal game-over info for JSON output.
#[derive(Serialize)]
pub struct GameOverInfo {
    pub winner: String,
    pub condition: String,
    pub turn: u32,
}

impl ActionResult {
    pub fn ok(turn: u32, diff: GameStateDiff) -> Self {
        Self {
            success: true,
            turn,
            deltas: diff.deltas,
            error: None,
            game_over: None,
        }
    }

    pub fn err(turn: u32, error: String) -> Self {
        Self {
            success: false,
            turn,
            deltas: Vec::new(),
            error: Some(error),
            game_over: None,
        }
    }

    pub fn with_game_over(mut self, info: GameOverInfo) -> Self {
        self.game_over = Some(info);
        self
    }
}

/// Print an `ActionResult` as JSON to stdout.
pub fn print_result(result: &ActionResult) {
    let json = serde_json::to_string_pretty(result).expect("failed to serialize result");
    println!("{json}");
}
