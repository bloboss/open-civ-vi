//! # libciv — Civilization VI Game Engine
//!
//! A pure Rust implementation of the Civilization VI game engine, designed as an
//! RL training environment. Implements full content parity with the base game,
//! Rise & Fall, Gathering Storm, and all DLC civilization packs.
//!
//! ## Content (539 tests)
//!
//! 50 civilizations · 132 units · 56 buildings · 53 world wonders · 26 natural
//! wonders · 76 techs · 60 civics · 127 policies · 13 governments · 46
//! city-states · 177 great people · 118 promotions · 7 projects · 6 victory
//! types · 5 alliance types
//!
//! ## Crate Structure
//!
//! - [`civ`] — Civilization, city, unit, diplomacy, religion, great people, governors
//! - [`game`] — Game state, rules engine, turn processing, combat, save/load, replay
//! - [`world`] — Terrain, features, resources, improvements, roads, natural wonders, climate
//! - [`rules`] — Tech/civic trees, modifiers, policies, governments, civ registry, promotions
//! - [`ai`] — Deterministic AI agent ([`ai::HeuristicAgent`])
//! - [`rl`] — Gym-like RL training harness ([`rl::CivEnv`])
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use libciv::{GameState, DefaultRulesEngine, RulesEngine};
//!
//! let mut state = GameState::new(42, 40, 24);
//! let rules = DefaultRulesEngine;
//! let diff = rules.advance_turn(&mut state);
//! ```
//!
//! ## RL Training
//!
//! ```rust,no_run
//! use libciv::rl::CivEnv;
//!
//! let mut env = CivEnv::new(42, 40, 24);
//! let obs = env.reset(42);
//! let actions = env.available_actions();
//! // let result = env.step(actions[0].clone());
//! ```
//!
//! ## Features
//!
//! - `serde` — JSON save/load via `save_game()`/`load_game()`, replay recording

pub mod enums;
pub mod ids;
pub mod yields;

/// Serde helpers for `&'static str` fields. Deserialization allocates a `String`
/// and leaks it to get a `&'static str`; this is acceptable for a game that
/// loads once per session.
#[cfg(feature = "serde")]
pub mod serde_static_str {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(val: &&'static str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(val)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<&'static str, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Box::leak(s.into_boxed_str()))
    }
}

/// Serde helper for `Vec<&'static str>` fields.
#[cfg(feature = "serde")]
pub mod serde_static_str_vec {
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(val: &[&'static str], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        val.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<&'static str>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<String> = Vec::deserialize(deserializer)?;
        Ok(v.into_iter().map(|s| &*Box::leak(s.into_boxed_str())).collect())
    }
}

/// Serde helper for `Option<&'static str>` fields.
#[cfg(feature = "serde")]
pub mod serde_static_str_opt {
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(val: &Option<&'static str>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        val.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<&'static str>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        Ok(opt.map(|s| &*Box::leak(s.into_boxed_str())))
    }
}

/// Serde helper for `HashSet<&'static str>` fields.
#[cfg(feature = "serde")]
pub mod serde_static_str_set {
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashSet;

    pub fn serialize<S>(val: &HashSet<&'static str>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        val.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashSet<&'static str>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<String> = Vec::deserialize(deserializer)?;
        Ok(v.into_iter().map(|s| &*Box::leak(s.into_boxed_str())).collect())
    }
}

/// Generic serde helper for `HashMap<K, V>` where K is not a string.
/// Serializes as a Vec of `(K, V)` pairs.
#[cfg(feature = "serde")]
pub mod serde_hashmap_as_vec {
    use std::collections::HashMap;
    use std::hash::Hash;
    use serde::{Serialize, Deserialize, Serializer, Deserializer};

    pub fn serialize<S, K, V>(map: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer, K: Serialize + Eq + Hash, V: Serialize {
        let pairs: Vec<(&K, &V)> = map.iter().collect();
        pairs.serialize(serializer)
    }

    pub fn deserialize<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
    where D: Deserializer<'de>, K: Deserialize<'de> + Eq + Hash, V: Deserialize<'de> {
        let pairs: Vec<(K, V)> = Vec::deserialize(deserializer)?;
        Ok(pairs.into_iter().collect())
    }
}

pub mod ai;
pub mod civ;
pub mod game;
pub mod rl;
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
    BuiltinVictoryCondition, DefaultRulesEngine, GameOver,
    GameState, GameStateDiff, RulesEngine,
    SCIENCE_MILESTONES, TurnEngine, VictoryKind,
};
pub use civ::era::EraAge;
pub use game::{apply_delta, apply_diff};
