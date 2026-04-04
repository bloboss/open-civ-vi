pub mod apply_delta;
pub mod board;
pub mod diff;
pub mod production_helpers;
pub mod rules;
pub mod rules_helpers;
pub mod save_load;
pub mod score;
pub mod state;
pub mod turn;
pub mod victory;
pub mod visibility;

pub use board::WorldBoard;
pub use diff::{AttackType, GameStateDiff, StateDelta};
pub use rules::{DefaultRulesEngine, FaithPurchaseItem, RulesEngine, RulesError};
pub use score::{all_scores, compute_score};
pub use state::{GameState, IdGenerator};
pub use turn::TurnEngine;
pub use victory::{BuiltinVictoryCondition, GameOver, VictoryKind, SCIENCE_MILESTONES};
pub use visibility::recalculate_visibility;
pub use apply_delta::{apply_delta, apply_diff};
pub use production_helpers::{
    available_unit_defs, available_building_defs,
    resolve_unit_replacement, resolve_building_replacement,
    can_produce_unit, can_produce_building,
    ALWAYS_AVAILABLE_UNITS, ALWAYS_AVAILABLE_BUILDINGS,
};
#[cfg(feature = "serde")]
pub use save_load::{save_game, load_game};
