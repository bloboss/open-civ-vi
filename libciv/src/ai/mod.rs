//! AI agent implementations. The [`HeuristicAgent`] provides a deterministic
//! opponent for testing, simulation, and RL training baselines.

pub mod deterministic;

pub use deterministic::{Agent, HeuristicAgent};
