//! Handler for the `end-turn` CLI command.
//!
//! Marks a player's turn as done. When all human players are done,
//! runs AI turns, advances the game turn, and resets movement.

use std::path::Path;

use libciv::ai::{Agent, HeuristicAgent};
use libciv::game::visibility::recalculate_visibility;
use libciv::{DefaultRulesEngine, GameStateDiff, RulesEngine};

use crate::output::{self, ActionResult};
use crate::state_io;

use super::{find_civ_by_name, validate_human};

/// End a player's turn. If all human players are done, advance the game.
pub fn handle_end_turn(game_file: &Path, player: &str) -> Result<(), String> {
    let mut state = state_io::load_game_file(game_file)?;
    let _civ_id = find_civ_by_name(&state, player)?;
    validate_human(&state, player)?;

    if state.turn_done.contains(player) {
        return Err(format!("'{player}' has already ended their turn"));
    }

    state.turn_done.insert(player.to_string());

    // Check if all human players are done.
    let all_humans_done = state
        .player_config
        .iter()
        .filter(|s| !s.is_ai)
        .all(|s| state.turn_done.contains(&s.civ_name));

    // If player_config is empty (legacy), treat as single-player: always advance.
    let should_advance = state.player_config.is_empty() || all_humans_done;

    let mut combined_diff = GameStateDiff {
        deltas: Vec::new(),
    };

    if should_advance {
        let rules = DefaultRulesEngine;

        // Collect AI civ IDs first to avoid borrow conflict.
        let ai_civ_ids: Vec<_> = state
            .player_config
            .iter()
            .filter(|s| s.is_ai)
            .filter_map(|s| {
                state
                    .civilizations
                    .iter()
                    .find(|c| c.name == s.civ_name.as_str())
                    .map(|c| c.id)
            })
            .collect();

        // Run AI turns.
        for cid in &ai_civ_ids {
            let agent = HeuristicAgent::new(*cid);
            let diff = agent.take_turn(&mut state, &rules);
            combined_diff.deltas.extend(diff.deltas);
        }

        // Advance turn.
        let turn_diff = rules.advance_turn(&mut state);
        combined_diff.deltas.extend(turn_diff.deltas);

        // Reset movement for all units.
        for unit in &mut state.units {
            unit.movement_left = unit.max_movement;
        }

        // Recalculate visibility for all civs.
        let civ_ids: Vec<_> = state.civilizations.iter().map(|c| c.id).collect();
        for cid in civ_ids {
            recalculate_visibility(&mut state, cid);
        }

        // Clear turn_done for the new turn.
        state.turn_done.clear();
    }

    state_io::save_game_file(game_file, &state)?;

    let result = ActionResult::ok(state.turn, combined_diff);
    output::print_result(&result);

    Ok(())
}
