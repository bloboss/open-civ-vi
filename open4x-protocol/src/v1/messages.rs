use serde::{Deserialize, Serialize};

use super::coord::HexCoord;
use super::enums::*;
use super::ids::*;
use super::profile::{CivTemplate, ProfileView};
use super::view::GameView;

// ── Client → Server ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ClientMessage {
    // Auth
    Authenticate { pubkey: Vec<u8>, signature: Vec<u8> },

    // Profile
    SetProfile(ProfileUpdate),

    // Lobby
    CreateGame(CreateGameRequest),
    JoinGame { game_id: GameId },
    ListGames,

    // In-game actions (applied within the current turn)
    Action(GameAction),
    EndTurn,

    Ping,
}

/// A single game action submitted by a player.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
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
    /// Set the player-selected production focus for a city. Stored on
    /// `City.focus` and surfaced through `/api/v1/cities/*` so the wire
    /// shape can show the selection. Citizen auto-assignment driven by
    /// focus is not yet implemented.
    AssignCityFocus {
        city: CityId,
        focus: CityFocus,
    },
    /// Rename a city. The new name is stored on `City.name`. Server
    /// validates length (1..=64) and ownership. No engine-side side
    /// effects beyond the rename.
    RenameCity {
        city: CityId,
        name: String,
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
    /// Drop the active (front) entry from the civ's research queue.
    /// Idempotent — a no-op when the queue is empty. Partial progress on
    /// the dropped tech is discarded (matches Civ VI's switch-research
    /// semantics).
    CancelResearch,
    /// Clear the civ's `civic_in_progress` slot. Idempotent — a no-op
    /// when no civic is being studied. Partial progress is discarded.
    CancelCivic,
    /// Switch the civ's active government to the named one. The
    /// requested government must be in the civ's `unlocked_governments`
    /// (typically populated by completing the prereq civic). Active
    /// policies that don't fit the new slot configuration are unslotted.
    /// No anarchy / cooldown is modeled.
    ChangeGovernment {
        name: String,
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
    FoundPantheon {
        belief: BeliefId,
    },
    FoundReligion {
        prophet: UnitId,
        name: String,
        beliefs: Vec<BeliefId>,
    },
    SpreadReligion {
        unit: UnitId,
    },
    TheologicalCombat {
        attacker: UnitId,
        defender: UnitId,
    },
    PurchaseWithFaith {
        city: CityId,
        item: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
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
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProfileUpdate {
    pub display_name: String,
    pub selected_template: CivTemplateId,
}

// ── Server → Client ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
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
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GameListEntry {
    pub game_id: GameId,
    pub name: String,
    pub players_joined: u32,
    pub max_players: u32,
    pub turn: u32,
    pub status: GameStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GameStatus {
    Lobby,
    InProgress,
    Finished,
}
