use serde::{Deserialize, Serialize};

use crate::coord::HexCoord;
use crate::enums::*;
use crate::ids::*;
use crate::profile::{CivTemplate, ProfileView};
use crate::view::GameView;

// ── Client → Server ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    // Auth
    Authenticate {
        pubkey: Vec<u8>,
        signature: Vec<u8>,
    },

    // Profile
    SetProfile(ProfileUpdate),

    // Lobby
    CreateGame(CreateGameRequest),
    JoinGame {
        game_id: GameId,
    },
    ListGames,

    // In-game actions (applied within the current turn)
    Action(GameAction),
    EndTurn,

    Ping,
}

/// A single game action submitted by a player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameAction {
    MoveUnit {
        unit: UnitId,
        to: HexCoord,
    },
    Attack {
        attacker: UnitId,
        defender: UnitId,
    },
    FoundCity {
        settler: UnitId,
        name: String,
    },
    PlaceImprovement {
        coord: HexCoord,
        improvement: BuiltinImprovement,
    },
    AssignCitizen {
        city: CityId,
        tile: HexCoord,
        lock: bool,
    },
    UnassignCitizen {
        city: CityId,
        tile: HexCoord,
    },
    QueueProduction {
        city: CityId,
        item: ProductionItemView,
    },
    CancelProduction {
        city: CityId,
        /// Index into the production queue (0 = front).
        index: usize,
    },
    EstablishTradeRoute {
        trader: UnitId,
        destination: CityId,
    },
    QueueResearch {
        tech: TechId,
    },
    QueueCivic {
        civic: CivicId,
    },
    DeclareWar {
        target: CivId,
    },
    MakePeace {
        target: CivId,
    },
    AssignPolicy {
        policy: PolicyId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGameRequest {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub num_ai: u32,
    pub max_players: u32,
    pub turn_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileUpdate {
    pub display_name: String,
    pub selected_template: CivTemplateId,
}

// ── Server → Client ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    // Auth
    Challenge {
        nonce: Vec<u8>,
    },
    AuthSuccess {
        session_token: String,
        profile: ProfileView,
    },
    AuthFailure {
        reason: String,
    },

    // Lobby
    GamesList(Vec<GameListEntry>),
    GameCreated {
        game_id: GameId,
    },
    GameJoined {
        game_id: GameId,
        view: GameView,
    },

    // In-game
    GameUpdate(GameView),
    ActionResult {
        ok: bool,
        error: Option<String>,
    },
    TurnResolved {
        new_turn: u32,
        view: GameView,
    },
    PlayerEndedTurn {
        civ_id: CivId,
    },
    GameOver {
        view: GameView,
    },

    // Profile
    ProfileUpdated(ProfileView),

    // Templates
    TemplatesList(Vec<CivTemplate>),

    Pong,
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameListEntry {
    pub game_id: GameId,
    pub name: String,
    pub players_joined: u32,
    pub max_players: u32,
    pub turn: u32,
    pub status: GameStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameStatus {
    Lobby,
    InProgress,
    Finished,
}
