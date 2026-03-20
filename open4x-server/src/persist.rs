//! Simple file-based game state persistence.
//!
//! Saves each game room's serializable state to a JSON file in the data
//! directory.  On server startup, any saved games are loaded back into memory.

use std::path::PathBuf;

use open4x_api::ids::GameId;
use open4x_api::view::GameView;

/// Directory where game snapshots are stored.
fn data_dir() -> PathBuf {
    std::env::var("OPEN4X_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data"))
}

/// Save a game snapshot (the GameView for each player) after turn resolution.
pub fn save_game_snapshot(game_id: GameId, turn: u32, views: &[(open4x_api::ids::CivId, GameView)]) {
    let dir = data_dir().join("games").join(game_id.to_string());
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("persist: failed to create dir {}: {e}", dir.display());
        return;
    }
    let path = dir.join(format!("turn_{turn}.json"));
    let data: Vec<_> = views.iter()
        .map(|(cid, view)| serde_json::json!({ "civ_id": cid.to_string(), "view": view }))
        .collect();
    match serde_json::to_string_pretty(&data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                eprintln!("persist: failed to write {}: {e}", path.display());
            }
        }
        Err(e) => eprintln!("persist: serialize error: {e}"),
    }
    // Also write a "latest" symlink/copy for easy access.
    let latest = dir.join("latest.json");
    let _ = std::fs::copy(&path, &latest);
}

/// List all saved game IDs (directory names under data/games/).
pub fn list_saved_games() -> Vec<String> {
    let dir = data_dir().join("games");
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}
