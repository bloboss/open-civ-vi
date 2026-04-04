//! Game state file I/O with atomic writes.
//!
//! `save_game_file` writes to a temporary file and renames it into place,
//! preventing corruption if the process is killed mid-write.

use std::path::Path;

use libciv::game::save_load;
use libciv::GameState;

/// Load a `GameState` from a JSON file.
pub fn load_game_file(path: &Path) -> Result<GameState, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    save_load::load_game(&json)
}

/// Save a `GameState` to a JSON file atomically (write tmp + rename).
pub fn save_game_file(path: &Path, state: &GameState) -> Result<(), String> {
    let json = save_load::save_game(state)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json)
        .map_err(|e| format!("failed to write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("failed to rename {} -> {}: {e}", tmp.display(), path.display()))?;
    Ok(())
}

/// Append a JSON line to the turn log file.
pub fn append_log(log_path: &Path, entry: &serde_json::Value) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| format!("failed to open log {}: {e}", log_path.display()))?;
    let line = serde_json::to_string(entry)
        .map_err(|e| format!("failed to serialize log entry: {e}"))?;
    writeln!(file, "{line}")
        .map_err(|e| format!("failed to write log: {e}"))?;
    Ok(())
}
