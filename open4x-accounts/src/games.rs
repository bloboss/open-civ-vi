//! Games index — Phase 4.1 of
//! `book/src/roadmap/accounts-and-login.md`.
//!
//! `GameStore` is the durable view of every game a player owns or is
//! a member of. The orchestrator (Phase 4.3) writes to it after
//! booting an `open4x-server` `GameRoom`; the lobby reads from it to
//! render the OngoingGames screen and to mint Resume tokens.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use ulid::Ulid;

use crate::PlayerId;

#[derive(Debug, thiserror::Error)]
pub enum GameStoreError {
    #[error("storage backend: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("game not found")]
    NotFound,
    #[error("forbidden: not a member of this game")]
    Forbidden,
}

pub type GameStoreResult<T> = Result<T, GameStoreError>;

/// Wire shape: durable game record. Refresh-on-poll fields
/// (`turn` / `era` / `score` / `status`) are written by the
/// orchestrator after each turn; the wizard params are immutable
/// after creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub game_id: String,
    pub owner_player_id: PlayerId,
    pub name: String,
    pub leader: String,
    pub civ_id: String,
    pub difficulty: String,
    pub players_human: u32,
    pub players_ai: u32,
    pub map_type: String,
    pub map_size: String,
    pub seed: String,
    pub turn: u32,
    pub era: String,
    pub score: i32,
    pub status: GameStatus,
    pub server_url: String,
    /// Bearer token the lobby mints + hands to the in-game server so
    /// inbound REST calls authenticate as the owner. NEVER returned
    /// over the wire to the browser — only used inside Resume.
    pub server_token: String,
    pub last_played_at: Option<String>,
    pub created_at: String,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    YourTurn,
    Waiting,
    Completed,
    Archived,
}

impl GameStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            GameStatus::YourTurn => "your_turn",
            GameStatus::Waiting => "waiting",
            GameStatus::Completed => "completed",
            GameStatus::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "your_turn" => GameStatus::YourTurn,
            "completed" => GameStatus::Completed,
            "archived" => GameStatus::Archived,
            _ => GameStatus::Waiting,
        }
    }
}

/// Input for [`GameStore::create_game`]: every column the wizard
/// fixes at game-creation time.
#[derive(Debug, Clone)]
pub struct NewGame {
    pub owner_player_id: PlayerId,
    pub name: String,
    pub leader: String,
    pub civ_id: String,
    pub difficulty: String,
    pub players_human: u32,
    pub players_ai: u32,
    pub map_type: String,
    pub map_size: String,
    pub seed: String,
    pub server_url: String,
    pub server_token: String,
}

#[async_trait]
pub trait GameStore: Send + Sync {
    async fn create_game(&self, body: NewGame) -> GameStoreResult<GameRecord>;
    async fn get_game(&self, game_id: &str) -> GameStoreResult<Option<GameRecord>>;
    async fn list_for_player(&self, player_id: PlayerId) -> GameStoreResult<Vec<GameRecord>>;
    async fn soft_delete(&self, game_id: &str, requester: PlayerId) -> GameStoreResult<()>;
    async fn touch_last_played(&self, game_id: &str) -> GameStoreResult<()>;
    async fn update_runtime_view(
        &self,
        game_id: &str,
        turn: u32,
        era: &str,
        score: i32,
        status: GameStatus,
    ) -> GameStoreResult<()>;

    /// Replace the per-game markdown notes. Owner-only — the handler
    /// is responsible for the auth check before calling this.
    async fn set_notes(&self, game_id: &str, notes: &str) -> GameStoreResult<()>;
}

// ───────────────────────────── Sqlite impl ────────────────────────────────────

pub struct SqliteGameStore {
    pool: Pool<Sqlite>,
}

impl SqliteGameStore {
    pub fn from_pool(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn connect(database_url: &str) -> GameStoreResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
}

fn player_id_text(id: &PlayerId) -> String {
    format!("{:016X}", id.0)
}

fn parse_player_id(s: &str) -> Result<PlayerId, GameStoreError> {
    u64::from_str_radix(s, 16)
        .map(PlayerId::new)
        .map_err(|_| GameStoreError::NotFound)
}

#[async_trait]
impl GameStore for SqliteGameStore {
    async fn create_game(&self, body: NewGame) -> GameStoreResult<GameRecord> {
        let game_id = Ulid::new().to_string();
        let now = Utc::now().to_rfc3339();
        let owner_text = player_id_text(&body.owner_player_id);

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO games (game_id, owner_player_id, name, leader, civ_id, \
                                difficulty, players_human, players_ai, map_type, \
                                map_size, seed, turn, era, score, status, \
                                server_url, server_token, last_played_at, \
                                created_at, deleted_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, 'Ancient', \
                     0, 'waiting', ?12, ?13, NULL, ?14, NULL)",
        )
        .bind(&game_id)
        .bind(&owner_text)
        .bind(&body.name)
        .bind(&body.leader)
        .bind(&body.civ_id)
        .bind(&body.difficulty)
        .bind(body.players_human as i64)
        .bind(body.players_ai as i64)
        .bind(&body.map_type)
        .bind(&body.map_size)
        .bind(&body.seed)
        .bind(&body.server_url)
        .bind(&body.server_token)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        // Seed the owner row in game_members.
        sqlx::query(
            "INSERT INTO game_members (game_id, player_id, role, invited_at, \
                                       joined_at) \
             VALUES (?1, ?2, 'owner', ?3, ?3)",
        )
        .bind(&game_id)
        .bind(&owner_text)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.get_game(&game_id)
            .await?
            .ok_or(GameStoreError::NotFound)
    }

    async fn get_game(&self, game_id: &str) -> GameStoreResult<Option<GameRecord>> {
        let row: Option<GameRow> = sqlx::query_as::<_, GameRow>(
            "SELECT * FROM games WHERE game_id = ?1 AND deleted_at IS NULL",
        )
        .bind(game_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| r.try_into()).transpose()
    }

    async fn list_for_player(&self, player_id: PlayerId) -> GameStoreResult<Vec<GameRecord>> {
        let pid = player_id_text(&player_id);
        let rows: Vec<GameRow> = sqlx::query_as::<_, GameRow>(
            "SELECT g.* FROM games g \
             INNER JOIN game_members gm ON gm.game_id = g.game_id \
             WHERE gm.player_id = ?1 AND g.deleted_at IS NULL \
             ORDER BY COALESCE(g.last_played_at, g.created_at) DESC",
        )
        .bind(&pid)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn soft_delete(&self, game_id: &str, requester: PlayerId) -> GameStoreResult<()> {
        let pid = player_id_text(&requester);
        let res = sqlx::query(
            "UPDATE games SET deleted_at = ?1 \
             WHERE game_id = ?2 AND owner_player_id = ?3 AND deleted_at IS NULL",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(game_id)
        .bind(&pid)
        .execute(&self.pool)
        .await?;
        match res.rows_affected() {
            0 => {
                // Distinguish missing vs forbidden: if the row exists
                // and is owned by someone else, return Forbidden.
                let other: Option<(String,)> =
                    sqlx::query_as("SELECT owner_player_id FROM games WHERE game_id = ?1")
                        .bind(game_id)
                        .fetch_optional(&self.pool)
                        .await?;
                match other {
                    Some(_) => Err(GameStoreError::Forbidden),
                    None => Err(GameStoreError::NotFound),
                }
            }
            _ => Ok(()),
        }
    }

    async fn touch_last_played(&self, game_id: &str) -> GameStoreResult<()> {
        sqlx::query(
            "UPDATE games SET last_played_at = ?1 \
             WHERE game_id = ?2 AND deleted_at IS NULL",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(game_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_runtime_view(
        &self,
        game_id: &str,
        turn: u32,
        era: &str,
        score: i32,
        status: GameStatus,
    ) -> GameStoreResult<()> {
        sqlx::query(
            "UPDATE games SET turn = ?1, era = ?2, score = ?3, status = ?4 \
             WHERE game_id = ?5 AND deleted_at IS NULL",
        )
        .bind(turn as i64)
        .bind(era)
        .bind(score)
        .bind(status.as_db_str())
        .bind(game_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_notes(&self, game_id: &str, notes: &str) -> GameStoreResult<()> {
        let res =
            sqlx::query("UPDATE games SET notes = ?1 WHERE game_id = ?2 AND deleted_at IS NULL")
                .bind(notes)
                .bind(game_id)
                .execute(&self.pool)
                .await?;
        if res.rows_affected() == 0 {
            return Err(GameStoreError::NotFound);
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct GameRow {
    game_id: String,
    owner_player_id: String,
    name: String,
    leader: String,
    civ_id: String,
    difficulty: String,
    players_human: i64,
    players_ai: i64,
    map_type: String,
    map_size: String,
    seed: String,
    turn: i64,
    era: String,
    score: i64,
    status: String,
    server_url: String,
    server_token: String,
    last_played_at: Option<String>,
    created_at: String,
    #[allow(dead_code)]
    deleted_at: Option<String>,
    #[sqlx(default)]
    notes: String,
}

impl TryFrom<GameRow> for GameRecord {
    type Error = GameStoreError;
    fn try_from(r: GameRow) -> Result<Self, Self::Error> {
        Ok(Self {
            game_id: r.game_id,
            owner_player_id: parse_player_id(&r.owner_player_id)?,
            name: r.name,
            leader: r.leader,
            civ_id: r.civ_id,
            difficulty: r.difficulty,
            players_human: r.players_human as u32,
            players_ai: r.players_ai as u32,
            map_type: r.map_type,
            map_size: r.map_size,
            seed: r.seed,
            turn: r.turn as u32,
            era: r.era,
            score: r.score as i32,
            status: GameStatus::parse(&r.status),
            server_url: r.server_url,
            server_token: r.server_token,
            last_played_at: r.last_played_at,
            created_at: r.created_at,
            notes: r.notes,
        })
    }
}

// ───────────────────────────── Tests ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Identity;
    use crate::store::{AccountStore, SqliteAccountStore};

    async fn fresh() -> (SqliteAccountStore, SqliteGameStore, PlayerId) {
        // We need a single sqlite file for the FK between games and
        // accounts to actually fire — pick a uniquely-named tempfile.
        let tmp = std::env::temp_dir().join(format!("o4x-games-{}.sqlite", Ulid::new()));
        let _ = std::fs::remove_file(&tmp);
        let url = format!("sqlite://{}?mode=rwc", tmp.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();

        let acct_store = SqliteAccountStore::connect(&tmp).await.unwrap();
        let game_store = SqliteGameStore::from_pool(pool);
        let acct = acct_store
            .find_or_create_account_for_identity(Identity::Email {
                address: "alice@example.com".into(),
                verified: true,
                primary: true,
            })
            .await
            .unwrap();
        (acct_store, game_store, acct.player_id)
    }

    fn sample_new_game(owner: PlayerId) -> NewGame {
        NewGame {
            owner_player_id: owner,
            name: "Cradle of the Indus".into(),
            leader: "Saladin".into(),
            civ_id: "arabia".into(),
            difficulty: "prince".into(),
            players_human: 1,
            players_ai: 7,
            map_type: "continents".into(),
            map_size: "std".into(),
            seed: "0xCAFE·B33F".into(),
            server_url: "http://127.0.0.1:3001".into(),
            server_token: "lobby_test_token".into(),
        }
    }

    #[tokio::test]
    async fn create_then_list_for_player() {
        let (_acct, games, owner) = fresh().await;
        let g = games.create_game(sample_new_game(owner)).await.unwrap();
        assert_eq!(g.owner_player_id, owner);
        assert_eq!(g.status, GameStatus::Waiting);
        let list = games.list_for_player(owner).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].game_id, g.game_id);
    }

    #[tokio::test]
    async fn soft_delete_hides_from_list() {
        let (_acct, games, owner) = fresh().await;
        let g = games.create_game(sample_new_game(owner)).await.unwrap();
        games.soft_delete(&g.game_id, owner).await.unwrap();
        let list = games.list_for_player(owner).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn update_runtime_view_round_trips() {
        let (_acct, games, owner) = fresh().await;
        let g = games.create_game(sample_new_game(owner)).await.unwrap();
        games
            .update_runtime_view(&g.game_id, 42, "Medieval", 814, GameStatus::YourTurn)
            .await
            .unwrap();
        let again = games.get_game(&g.game_id).await.unwrap().unwrap();
        assert_eq!(again.turn, 42);
        assert_eq!(again.era, "Medieval");
        assert_eq!(again.score, 814);
        assert_eq!(again.status, GameStatus::YourTurn);
    }
}
