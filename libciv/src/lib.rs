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
    BuiltinVictoryCondition, DefaultRulesEngine, GameOver,
    GameState, GameStateDiff, RulesEngine,
    SCIENCE_MILESTONES, TurnEngine, VictoryKind,
};
pub use civ::era::EraAge;
pub use game::{apply_delta, apply_diff};
