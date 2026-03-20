//! Server-wide shared state and per-game room state.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::broadcast;

use libciv::ai::HeuristicAgent;
use libciv::game::state::GameState;
use libciv::{CivId, DefaultRulesEngine};

use open4x_api::ids::GameId;
use open4x_api::messages::{GameStatus, ServerMessage};
use open4x_api::profile::ProfileView;

/// Global server state shared across all WebSocket connections.
pub struct AppState {
    pub games: DashMap<GameId, GameRoom>,
    pub players: DashMap<[u8; 32], PlayerRecord>,
    pub templates: Vec<open4x_api::profile::CivTemplate>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            games: DashMap::new(),
            players: DashMap::new(),
            templates: crate::templates::builtin_templates(),
        })
    }
}

/// A single game session on the server.
pub struct GameRoom {
    pub game_id: GameId,
    pub name: String,
    pub state: GameState,
    pub rules: DefaultRulesEngine,
    pub players: Vec<PlayerSlot>,
    pub ai_agents: Vec<(CivId, HeuristicAgent)>,
    pub status: GameStatus,
    pub config: GameRoomConfig,
    /// Broadcast channel for sending updates to connected players.
    pub tx: broadcast::Sender<ServerMessage>,
}

pub struct GameRoomConfig {
    pub max_players: u32,
    pub turn_limit: Option<u32>,
}

/// A player slot within a game room.
pub struct PlayerSlot {
    pub civ_id: CivId,
    pub pubkey: [u8; 32],
    pub profile: ProfileView,
    pub submitted_turn: bool,
}

/// Persistent player record (in-memory for now).
pub struct PlayerRecord {
    pub pubkey: [u8; 32],
    pub display_name: String,
    pub selected_template: open4x_api::ids::CivTemplateId,
    pub games_played: u32,
}
