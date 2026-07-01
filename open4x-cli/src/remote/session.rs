//! Session token persistence. `new-game` writes a small JSON file with
//! the bearer token + game/civ IDs returned by `POST /games/new`;
//! every subsequent subcommand reads it back.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub server: String,
    pub game_id: String,
    pub civ_id: String,
    pub token: String,
    pub turn: u32,
}

impl Session {
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize session: {e}"))?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, &json)
            .map_err(|e| format!("failed to write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| {
            format!(
                "failed to rename {} -> {}: {e}",
                tmp.display(),
                path.display()
            )
        })?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| {
            format!(
                "failed to read session file {} ({e}); run `open4x --server <URL> new-game ...` first",
                path.display()
            )
        })?;
        serde_json::from_slice(&bytes)
            .map_err(|e| format!("invalid session file {}: {e}", path.display()))
    }
}
