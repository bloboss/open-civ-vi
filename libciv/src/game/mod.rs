//! Core game engine: state management, rules engine, turn processing, combat,
//! diff system, save/load, replay, production helpers, and victory conditions.
//!
//! The central type is [`GameState`] which holds all game data. The
//! [`RulesEngine`] trait defines all game actions; [`DefaultRulesEngine`]
//! provides the standard implementation.

pub mod apply_delta;
pub mod board;
pub mod diff;
pub mod production_helpers;
pub mod replay;
pub mod rules;
pub mod rules_helpers;
pub mod save_load;
pub mod score;
pub mod state;
pub mod turn;
pub mod victory;
pub mod visibility;

pub use apply_delta::{apply_delta, apply_diff};
pub use board::WorldBoard;
pub use diff::{AttackType, GameStateDiff, StateDelta};
pub use production_helpers::{
    ALWAYS_AVAILABLE_BUILDINGS, ALWAYS_AVAILABLE_UNITS, available_building_defs,
    available_buildings_for_city, available_unit_defs, can_produce_building, can_produce_unit,
    resolve_building_replacement, resolve_unit_replacement,
};
#[cfg(feature = "serde")]
pub use replay::{ReplayRecorder, ReplayViewer};
pub use rules::{
    CombatPreview, DefaultRulesEngine, FaithPurchaseItem, PendingAction, PendingActionKind,
    PolicyCardEntry, PolicyCardStatus, RulesEngine, RulesError, UnitAction, UnitActionKind,
};
#[cfg(feature = "serde")]
pub use save_load::{load_game, save_game};
pub use score::{all_scores, compute_score};
pub use state::{GameState, IdGenerator};
pub use turn::TurnEngine;
pub use victory::{BuiltinVictoryCondition, GameOver, SCIENCE_MILESTONES, VictoryKind};
pub use visibility::recalculate_visibility;
