//! Friends graph (Phase 5 polish).
//!
//! Single row per directed (requester, target) pair. Reads
//! synthesize the inverse direction at query time so we don't
//! duplicate rows.

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use thiserror::Error;

use crate::PlayerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FriendStatus {
    /// I sent a request, the other party hasn't responded yet.
    PendingOutgoing,
    /// They sent a request, I haven't accepted yet.
    PendingIncoming,
    /// Mutual.
    Accepted,
    /// I blocked them. They never see this row.
    Blocked,
}

impl FriendStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            FriendStatus::PendingOutgoing => "pending_outgoing",
            FriendStatus::PendingIncoming => "pending_incoming",
            FriendStatus::Accepted => "accepted",
            FriendStatus::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendRow {
    pub other_player_id: PlayerId,
    pub status: FriendStatus,
    pub created_at: String,
}

#[derive(Debug, Error)]
pub enum FriendsError {
    #[error("storage backend: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("invalid input: {0}")]
    Invalid(&'static str),
    #[error("not found")]
    NotFound,
    #[error("conflict — relationship already exists")]
    AlreadyExists,
}

pub type FriendsResult<T> = Result<T, FriendsError>;

#[async_trait]
pub trait FriendsStore: Send + Sync {
    /// Every row touching `me`, with status oriented from `me`'s
    /// POV (incoming-pending vs outgoing-pending split out).
    async fn list_for(&self, me: PlayerId) -> FriendsResult<Vec<FriendRow>>;

    /// Create a fresh `pending` row. Refuses `from == to`. Returns
    /// `AlreadyExists` if any row already touches the pair.
    async fn request(&self, from: PlayerId, to: PlayerId) -> FriendsResult<()>;

    /// Flip the existing `(requester=other, target=accepter)` row
    /// to `accepted`. Refuses if no `pending` row exists.
    async fn accept(&self, accepter: PlayerId, requester: PlayerId) -> FriendsResult<()>;

    /// Drop any row connecting `me` and `other`, regardless of
    /// direction or status. Idempotent.
    async fn unfriend(&self, me: PlayerId, other: PlayerId) -> FriendsResult<()>;
}

pub struct SqliteFriendsStore {
    pool: Pool<Sqlite>,
}

impl SqliteFriendsStore {
    pub fn from_pool(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

fn pid_text(id: PlayerId) -> String {
    format!("{:016X}", id.0)
}

fn pid_from(text: &str) -> Option<PlayerId> {
    u64::from_str_radix(text, 16).ok().map(PlayerId::new)
}

#[derive(sqlx::FromRow)]
struct Row {
    a_player_id: String,
    b_player_id: String,
    status: String,
    created_at: String,
}

#[async_trait]
impl FriendsStore for SqliteFriendsStore {
    async fn list_for(&self, me: PlayerId) -> FriendsResult<Vec<FriendRow>> {
        let me_text = pid_text(me);
        let rows: Vec<Row> = sqlx::query_as::<_, Row>(
            "SELECT a_player_id, b_player_id, status, created_at \
             FROM friends \
             WHERE a_player_id = ?1 OR b_player_id = ?1 \
             ORDER BY created_at DESC",
        )
        .bind(&me_text)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let i_am_a = r.a_player_id == me_text;
                let other_text = if i_am_a {
                    &r.b_player_id
                } else {
                    &r.a_player_id
                };
                let other = pid_from(other_text)?;
                let status = match (r.status.as_str(), i_am_a) {
                    ("pending", true) => FriendStatus::PendingOutgoing,
                    ("pending", false) => FriendStatus::PendingIncoming,
                    ("accepted", _) => FriendStatus::Accepted,
                    ("blocked", true) => FriendStatus::Blocked,
                    // The blocked party never gets their row served.
                    ("blocked", false) => return None,
                    _ => return None,
                };
                Some(FriendRow {
                    other_player_id: other,
                    status,
                    created_at: r.created_at,
                })
            })
            .collect())
    }

    async fn request(&self, from: PlayerId, to: PlayerId) -> FriendsResult<()> {
        if from == to {
            return Err(FriendsError::Invalid("cannot friend yourself"));
        }
        let from_t = pid_text(from);
        let to_t = pid_text(to);
        let now = Utc::now().to_rfc3339();
        // Refuse if any row already exists in either direction so
        // we don't crisscross relationships.
        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM friends \
             WHERE (a_player_id = ?1 AND b_player_id = ?2) \
                OR (a_player_id = ?2 AND b_player_id = ?1)",
        )
        .bind(&from_t)
        .bind(&to_t)
        .fetch_one(&self.pool)
        .await?;
        if existing > 0 {
            return Err(FriendsError::AlreadyExists);
        }
        sqlx::query(
            "INSERT INTO friends (a_player_id, b_player_id, status, created_at) \
             VALUES (?1, ?2, 'pending', ?3)",
        )
        .bind(&from_t)
        .bind(&to_t)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn accept(&self, accepter: PlayerId, requester: PlayerId) -> FriendsResult<()> {
        let accepter_t = pid_text(accepter);
        let requester_t = pid_text(requester);
        let res = sqlx::query(
            "UPDATE friends SET status = 'accepted' \
             WHERE a_player_id = ?1 AND b_player_id = ?2 AND status = 'pending'",
        )
        .bind(&requester_t)
        .bind(&accepter_t)
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(FriendsError::NotFound);
        }
        Ok(())
    }

    async fn unfriend(&self, me: PlayerId, other: PlayerId) -> FriendsResult<()> {
        let me_t = pid_text(me);
        let other_t = pid_text(other);
        sqlx::query(
            "DELETE FROM friends \
             WHERE (a_player_id = ?1 AND b_player_id = ?2) \
                OR (a_player_id = ?2 AND b_player_id = ?1)",
        )
        .bind(&me_t)
        .bind(&other_t)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Identity;
    use crate::store::{AccountStore, SqliteAccountStore};

    struct TempFile(std::path::PathBuf);
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("sqlite-wal"));
            let _ = std::fs::remove_file(self.0.with_extension("sqlite-shm"));
        }
    }

    async fn fixture() -> (
        TempFile,
        SqliteAccountStore,
        SqliteFriendsStore,
        PlayerId,
        PlayerId,
    ) {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "open4x_friends_{}_{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let tmp = TempFile(p.clone());
        let store = SqliteAccountStore::connect(&p).await.unwrap();
        // SqliteAccountStore.pool is private; reach into it via the
        // trait-side construction by re-opening the pool and wiring
        // the friends store on top. SqliteAccountStore's connect
        // already ran the migrations.
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&p)
            .pragma("foreign_keys", "ON");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(opts)
            .await
            .unwrap();
        let friends = SqliteFriendsStore::from_pool(pool);
        let alice = store
            .find_or_create_account_for_identity(Identity::Email {
                address: "alice@x".into(),
                verified: true,
                primary: true,
            })
            .await
            .unwrap();
        let bob = store
            .find_or_create_account_for_identity(Identity::Email {
                address: "bob@x".into(),
                verified: true,
                primary: true,
            })
            .await
            .unwrap();
        (tmp, store, friends, alice.player_id, bob.player_id)
    }

    #[tokio::test]
    async fn request_accept_unfriend_round_trip() {
        let (_tmp, _accounts, friends, alice, bob) = fixture().await;
        // self-friend refused
        assert!(matches!(
            friends.request(alice, alice).await.unwrap_err(),
            FriendsError::Invalid(_)
        ));
        // alice → bob request
        friends.request(alice, bob).await.unwrap();
        // duplicate rejected
        assert!(matches!(
            friends.request(alice, bob).await.unwrap_err(),
            FriendsError::AlreadyExists
        ));
        assert!(matches!(
            friends.request(bob, alice).await.unwrap_err(),
            FriendsError::AlreadyExists
        ));
        // alice sees outgoing, bob sees incoming
        let alice_view = friends.list_for(alice).await.unwrap();
        assert_eq!(alice_view.len(), 1);
        assert_eq!(alice_view[0].status, FriendStatus::PendingOutgoing);
        let bob_view = friends.list_for(bob).await.unwrap();
        assert_eq!(bob_view.len(), 1);
        assert_eq!(bob_view[0].status, FriendStatus::PendingIncoming);
        // bob accepts
        friends.accept(bob, alice).await.unwrap();
        let alice_view = friends.list_for(alice).await.unwrap();
        assert_eq!(alice_view[0].status, FriendStatus::Accepted);
        let bob_view = friends.list_for(bob).await.unwrap();
        assert_eq!(bob_view[0].status, FriendStatus::Accepted);
        // either side unfriends
        friends.unfriend(alice, bob).await.unwrap();
        assert!(friends.list_for(alice).await.unwrap().is_empty());
        assert!(friends.list_for(bob).await.unwrap().is_empty());
    }
}
