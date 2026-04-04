//! Reward shaping for the RL training loop.
//!
//! The default reward is a small per-step score delta plus a large terminal
//! bonus/penalty.  This keeps the signal informative without overwhelming the
//! agent with sparse rewards.

/// Compute the reward for a single step.
///
/// * `prev_score` / `curr_score` — the civ's score before and after the step.
/// * `done` — whether the game has ended.
/// * `won` — whether the agent's civ won.
pub fn compute_reward(prev_score: u32, curr_score: u32, done: bool, won: bool) -> f64 {
    let mut reward = (curr_score as f64 - prev_score as f64) * 0.01;
    if done {
        reward += if won { 100.0 } else { -100.0 };
    }
    reward
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_delta_positive() {
        let r = compute_reward(10, 15, false, false);
        assert!((r - 0.05).abs() < 1e-9);
    }

    #[test]
    fn score_delta_negative() {
        let r = compute_reward(20, 18, false, false);
        assert!((r - (-0.02)).abs() < 1e-9);
    }

    #[test]
    fn terminal_win() {
        let r = compute_reward(50, 55, true, true);
        assert!((r - 100.05).abs() < 1e-9);
    }

    #[test]
    fn terminal_loss() {
        let r = compute_reward(50, 50, true, false);
        assert!((r - (-100.0)).abs() < 1e-9);
    }
}
