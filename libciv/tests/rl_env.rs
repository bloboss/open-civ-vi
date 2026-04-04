//! Integration tests for the RL training harness.

use libciv::rl::{Action, CivEnv};

#[test]
fn env_reset_returns_valid_observation() {
    let mut env = CivEnv::new(42, 20, 12);
    let obs = env.reset(42);

    assert_eq!(obs.turn, 0);
    assert!(obs.num_cities >= 1, "agent should have at least one city");
    assert!(obs.num_units >= 1, "agent should have at least one unit");
    assert!(!obs.game_over, "game should not be over on reset");
    assert!(!obs.is_winner);
    assert!(obs.score > 0, "initial score should be positive (city + territory)");
}

#[test]
fn env_step_end_turn_advances() {
    let mut env = CivEnv::new(99, 20, 12);
    let obs0 = env.reset(99);
    assert_eq!(obs0.turn, 0);

    let result = env.step(Action::EndTurn);
    assert_eq!(result.observation.turn, 1);
    assert!(result.info.action_result.is_ok());
    assert!(!result.done, "game should not be over after one turn");
}

#[test]
fn env_step_move_unit() {
    let mut env = CivEnv::new(42, 20, 12);
    env.reset(42);

    // Find the agent's unit and a valid neighbor.
    let actions = env.available_actions();
    let move_action = actions.iter().find(|a| matches!(a, Action::MoveUnit { .. }));

    if let Some(action) = move_action {
        let result = env.step(action.clone());
        assert!(
            result.info.action_result.is_ok(),
            "move should succeed: {:?}",
            result.info.action_result
        );
    } else {
        panic!("no MoveUnit action available from starting position");
    }
}

#[test]
fn env_deterministic() {
    // Same seed + same actions should yield the same observations.
    let obs_a = {
        let mut env = CivEnv::new(77, 20, 12);
        env.reset(77);
        let r1 = env.step(Action::EndTurn);
        let r2 = env.step(Action::EndTurn);
        (r1.observation.score, r2.observation.score, r2.observation.turn)
    };
    let obs_b = {
        let mut env = CivEnv::new(77, 20, 12);
        env.reset(77);
        let r1 = env.step(Action::EndTurn);
        let r2 = env.step(Action::EndTurn);
        (r1.observation.score, r2.observation.score, r2.observation.turn)
    };
    assert_eq!(obs_a, obs_b, "same seed + same actions should be deterministic");
}

#[test]
fn env_game_completes() {
    // Run the game with only EndTurn actions until it completes or we hit a
    // safety limit. The score victory is set at turn 100.
    let mut env = CivEnv::new(42, 20, 12);
    env.reset(42);

    let mut done = false;
    for _ in 0..120 {
        let result = env.step(Action::EndTurn);
        if result.done {
            done = true;
            // The game should declare a winner.
            assert!(result.observation.game_over);
            break;
        }
    }
    assert!(done, "game should complete within 120 turns (score victory at turn 100)");
}

#[test]
fn env_available_actions_always_includes_end_turn() {
    let mut env = CivEnv::new(42, 20, 12);
    env.reset(42);
    let actions = env.available_actions();
    assert!(
        actions.iter().any(|a| matches!(a, Action::EndTurn)),
        "EndTurn should always be available"
    );
}

#[test]
fn env_queue_production() {
    let mut env = CivEnv::new(42, 20, 12);
    env.reset(42);

    // Find a QueueProduction action (cities should start with empty queues).
    let actions = env.available_actions();
    let prod_action = actions
        .iter()
        .find(|a| matches!(a, Action::QueueProduction { .. }));

    if let Some(action) = prod_action {
        let result = env.step(action.clone());
        assert!(result.info.action_result.is_ok());
    } else {
        panic!("no QueueProduction action available from starting position");
    }
}
