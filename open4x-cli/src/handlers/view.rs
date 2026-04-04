//! Handler for the `view` CLI command.
//!
//! Exports the fog-of-war filtered game state for a specific player as JSON.

use std::path::Path;

use crate::player_view;
use crate::state_io;

use super::find_civ_by_name;

/// Build and print a player's fog-of-war filtered view as JSON.
pub fn handle_view(game_file: &Path, player: &str) -> Result<(), String> {
    let state = state_io::load_game_file(game_file)?;
    let civ_id = find_civ_by_name(&state, player)?;

    let view = player_view::build_player_view(&state, civ_id);
    let json = serde_json::to_string_pretty(&view)
        .map_err(|e| format!("failed to serialize player view: {e}"))?;
    println!("{json}");

    Ok(())
}
