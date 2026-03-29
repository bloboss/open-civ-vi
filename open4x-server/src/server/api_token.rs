use std::sync::Arc;

use crate::types::ids::{CivId, GameId};
use crate::server::state::AppState;

/// Record for an API bearer token.
pub struct ApiTokenRecord {
    pub token: String,
    pub pubkey: [u8; 32],
    pub game_id: GameId,
    pub civ_id: CivId,
}

/// Generate a random 32-byte hex token string.
pub fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Resolve a bearer token to its associated game and civ.
pub fn resolve_token(
    state: &Arc<AppState>,
    token: &str,
) -> Option<(GameId, CivId)> {
    state.api_tokens.get(token).map(|r| (r.game_id, r.civ_id))
}
