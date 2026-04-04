pub mod action;
pub mod end_turn;
pub mod list;
pub mod new_game;
pub mod status;
pub mod view;

use libciv::{CivId, GameState};

/// Find a civilization by name, returning its CivId.
pub fn find_civ_by_name(state: &GameState, name: &str) -> Result<CivId, String> {
    state
        .civilizations
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.id)
        .ok_or_else(|| format!("no civilization named '{name}'"))
}

/// Validate that the player is a human (non-AI) slot.
pub fn validate_human(state: &GameState, player: &str) -> Result<(), String> {
    let slot = state
        .player_config
        .iter()
        .find(|s| s.civ_name == player);
    match slot {
        Some(s) if s.is_ai => Err(format!("'{player}' is an AI player")),
        Some(_) => Ok(()),
        // If player_config is empty (legacy games), allow anyone.
        None if state.player_config.is_empty() => Ok(()),
        None => Err(format!("'{player}' is not a registered player")),
    }
}

/// Parse a ULID string into a typed ID.
///
/// Accepts the raw 26-character Crockford Base32 ULID string (e.g. "01HPQR...").
/// Also accepts the `TypeName(ULID)` display format by extracting the inner part.
pub fn parse_ulid(s: &str) -> Result<ulid::Ulid, String> {
    // If the string contains '(' it might be in Display format: "UnitId(01HPQR...)"
    let raw = if let Some(start) = s.find('(') {
        let end = s.find(')').unwrap_or(s.len());
        &s[start + 1..end]
    } else {
        s
    };
    ulid::Ulid::from_string(raw).map_err(|e| format!("invalid ULID '{s}': {e}"))
}
