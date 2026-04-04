//! Gym-like RL training harness.
//!
//! Wraps the game engine in a step-based interface compatible with standard
//! RL frameworks (Gymnasium / PettingZoo conventions).
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use libciv::rl::{CivEnv, Action};
//!
//! let mut env = CivEnv::new(42, 20, 12);
//! let obs = env.reset(42);
//! let result = env.step(Action::EndTurn);
//! ```

pub mod action;
pub mod env;
pub mod observation;
pub mod reward;

pub use action::Action;
pub use env::{CivEnv, StepInfo, StepResult};
pub use observation::{observe, Observation};
pub use reward::compute_reward;
