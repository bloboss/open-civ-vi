//! Server-wide shared state and per-game room state.

use std::collections::VecDeque;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::broadcast;

use libciv::ai::HeuristicAgent;
use libciv::game::state::GameState;
use libciv::{CivId, DefaultRulesEngine};

use open4x_protocol::v1::ids::GameId;
use open4x_protocol::v1::messages::{GameStatus, ServerMessage};
use open4x_protocol::v1::profile::ProfileView;

/// Global server state shared across all WebSocket connections.
pub struct AppState {
    pub games: DashMap<GameId, GameRoom>,
    pub players: DashMap<[u8; 32], PlayerRecord>,
    pub templates: Vec<open4x_protocol::v1::profile::CivTemplate>,
    pub api_tokens: DashMap<String, crate::server::api_token::ApiTokenRecord>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            games: DashMap::new(),
            players: DashMap::new(),
            templates: crate::server::templates::builtin_templates(),
            api_tokens: DashMap::new(),
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
    /// Per-civ ring buffer of notifications. Populated from `advance_turn`
    /// deltas; capped per civ at `NOTIFICATION_CAP` (oldest evicted).
    pub notifications: NotificationBuffer,
}

/// Bounded notification log keyed by civilization. Newest notification at the
/// back (so iteration yields chronological order).
pub const NOTIFICATION_CAP: usize = 64;

/// Keyed by the **wire-side** `CivId` (`open4x_protocol::v1::ids::CivId`), not the
/// libciv one — the projector reads this with the auth_or_401 result.
pub type NotifCivId = open4x_protocol::v1::ids::CivId;

#[derive(Debug, Clone, Default)]
pub struct NotificationBuffer {
    pub by_civ: std::collections::HashMap<NotifCivId, VecDeque<NotificationRecord>>,
    pub next_id: u64,
}

#[derive(Debug, Clone)]
pub struct NotificationRecord {
    pub id: String,
    pub turn: u32,
    pub kind: NotificationKind,
    pub category: &'static str,
    pub title: String,
    pub desc: String,
    pub target: Option<NotificationTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Accent,
    Good,
    Warn,
    Bad,
    Neutral,
}

impl NotificationKind {
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Accent => "accent",
            Self::Good => "good",
            Self::Warn => "warn",
            Self::Bad => "bad",
            Self::Neutral => "",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotificationTarget {
    pub screen: String,
    pub q: Option<i32>,
    pub r: Option<i32>,
}

impl NotificationBuffer {
    pub fn push(&mut self, civ: NotifCivId, mut rec: NotificationRecord) {
        if rec.id.is_empty() {
            self.next_id += 1;
            rec.id = format!("n{}", self.next_id);
        }
        let q = self.by_civ.entry(civ).or_default();
        if q.len() >= NOTIFICATION_CAP {
            q.pop_front();
        }
        q.push_back(rec);
    }

    pub fn for_civ(&self, civ: NotifCivId) -> Vec<NotificationRecord> {
        self.by_civ
            .get(&civ)
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn dismiss(&mut self, civ: NotifCivId, id: &str) -> bool {
        let Some(q) = self.by_civ.get_mut(&civ) else {
            return false;
        };
        let len_before = q.len();
        q.retain(|r| r.id != id);
        len_before != q.len()
    }

    pub fn dismiss_all(&mut self, civ: NotifCivId) {
        if let Some(q) = self.by_civ.get_mut(&civ) {
            q.clear();
        }
    }
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
    pub selected_template: open4x_protocol::v1::ids::CivTemplateId,
    pub games_played: u32,
}
