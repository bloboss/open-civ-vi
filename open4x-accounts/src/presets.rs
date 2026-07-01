//! User-saved wizard presets (Phase 5 polish).
//!
//! Opaque `body_json` — the lobby SPA echos its `WizardState`
//! shape through this storage layer; accounts crate doesn't
//! interpret it.


use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use thiserror::Error;
use ulid::Ulid;

use crate::PlayerId;

#[derive(Debug, Clone, Serialize)]
pub struct PresetRow {
    pub id: String,
    pub name: String,
    pub body_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Error)]
pub enum PresetsError {
    #[error("storage backend: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
    #[error("invalid input: {0}")]
    Invalid(&'static str),
}

pub type PresetsResult<T> = Result<T, PresetsError>;

#[async_trait]
pub trait PresetsStore: Send + Sync {
    async fn list_for(&self, player_id: PlayerId) -> PresetsResult<Vec<PresetRow>>;
    async fn create(
        &self,
        player_id: PlayerId,
        name: &str,
        body_json: &str,
    ) -> PresetsResult<PresetRow>;
    async fn delete(&self, player_id: PlayerId, id: &str) -> PresetsResult<()>;
}

pub struct SqlitePresetsStore {
    pool: Pool<Sqlite>,
}

impl SqlitePresetsStore {
    pub fn from_pool(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

fn pid_text(id: PlayerId) -> String {
    format!("{:016X}", id.0)
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    name: String,
    body_json: String,
    created_at: String,
    updated_at: String,
}

#[async_trait]
impl PresetsStore for SqlitePresetsStore {
    async fn list_for(&self, player_id: PlayerId) -> PresetsResult<Vec<PresetRow>> {
        let pid = pid_text(player_id);
        let rows: Vec<Row> = sqlx::query_as::<_, Row>(
            "SELECT id, name, body_json, created_at, updated_at \
             FROM presets WHERE player_id = ?1 \
             ORDER BY updated_at DESC",
        )
        .bind(&pid)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| PresetRow {
                id: r.id,
                name: r.name,
                body_json: r.body_json,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    async fn create(
        &self,
        player_id: PlayerId,
        name: &str,
        body_json: &str,
    ) -> PresetsResult<PresetRow> {
        let name = name.trim();
        if name.is_empty() {
            return Err(PresetsError::Invalid("name is required"));
        }
        if name.len() > 64 {
            return Err(PresetsError::Invalid("name capped at 64 chars"));
        }
        if body_json.len() > 32 * 1024 {
            return Err(PresetsError::Invalid("body capped at 32 KiB"));
        }
        // Cheap validity check — refuse non-JSON so the SPA can't
        // poison the row with garbage. We don't enforce schema.
        if serde_json::from_str::<serde_json::Value>(body_json).is_err() {
            return Err(PresetsError::Invalid("body must be valid JSON"));
        }
        let id = Ulid::new().to_string();
        let pid = pid_text(player_id);
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO presets (id, player_id, name, body_json, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        )
        .bind(&id)
        .bind(&pid)
        .bind(name)
        .bind(body_json)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(PresetRow {
            id,
            name: name.to_string(),
            body_json: body_json.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    async fn delete(&self, player_id: PlayerId, id: &str) -> PresetsResult<()> {
        let pid = pid_text(player_id);
        let res = sqlx::query(
            "DELETE FROM presets WHERE id = ?1 AND player_id = ?2",
        )
        .bind(id)
        .bind(&pid)
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(PresetsError::NotFound);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{AccountStore, SqliteAccountStore};
    use crate::Identity;

    struct TempFile(std::path::PathBuf);
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("sqlite-wal"));
            let _ = std::fs::remove_file(self.0.with_extension("sqlite-shm"));
        }
    }

    #[tokio::test]
    async fn create_list_delete_round_trip() {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "open4x_presets_{}_{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let tmp = TempFile(p.clone());
        let _store = SqliteAccountStore::connect(&p).await.unwrap();
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&p)
            .pragma("foreign_keys", "ON");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(opts)
            .await
            .unwrap();
        let accounts = SqliteAccountStore::connect(&p).await.unwrap();
        let alice = accounts
            .find_or_create_account_for_identity(Identity::Email {
                address: "alice@x".into(),
                verified: true,
                primary: true,
            })
            .await
            .unwrap();
        let presets = SqlitePresetsStore::from_pool(pool);

        assert!(presets.list_for(alice.player_id).await.unwrap().is_empty());

        let created = presets
            .create(alice.player_id, "Standard prince", r#"{"diff":"prince"}"#)
            .await
            .unwrap();
        assert_eq!(created.name, "Standard prince");
        assert!(!created.id.is_empty());

        // Validation paths
        assert!(matches!(
            presets.create(alice.player_id, "", "{}").await.unwrap_err(),
            PresetsError::Invalid(_)
        ));
        assert!(matches!(
            presets
                .create(alice.player_id, "valid", "not json")
                .await
                .unwrap_err(),
            PresetsError::Invalid(_)
        ));

        let listed = presets.list_for(alice.player_id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].body_json, r#"{"diff":"prince"}"#);

        presets.delete(alice.player_id, &created.id).await.unwrap();
        assert!(presets.list_for(alice.player_id).await.unwrap().is_empty());

        // Double-delete returns NotFound.
        assert!(matches!(
            presets.delete(alice.player_id, &created.id).await.unwrap_err(),
            PresetsError::NotFound
        ));

        drop(tmp);
    }
}
