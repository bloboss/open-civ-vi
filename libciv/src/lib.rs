pub mod enums;
pub mod ids;
pub mod yields;

pub mod ai;
pub mod civ;
pub mod game;
pub mod rules;
pub mod visualize;
pub mod world;

// Flat re-exports: callers can use `libciv::CivId`, `libciv::YieldBundle`, etc.
pub use enums::*;
pub use ids::*;
pub use yields::*;

// Top-level re-exports for the most commonly used game-loop types.
pub use game::{
    all_scores, compute_score,
    DefaultRulesEngine, GameOver, GameState, GameStateDiff, RulesEngine, ScoreVictory,
    TurnEngine, VictoryCondition, VictoryKind,
};
