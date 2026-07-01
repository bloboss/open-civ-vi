//! Persistence layer — `AccountStore` trait + the sqlite-backed
//! `SqliteAccountStore` implementation.
//!
//! Gated on the `persistence` Cargo feature; type-only consumers
//! (the lobby's csr/wasm build) don't drag sqlx in.
//!
//! Phase 2.1 of `book/src/roadmap/accounts-and-login.md`. Magic-link
//! minting / OIDC client / atproto resolver / session token issuance
//! land in 2.2-2.5; this module just owns the durable rows.

use std::path::Path;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use thiserror::Error;
use ulid::Ulid;

use crate::{Account, Identity, PlayerId, Preferences};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("storage backend: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("identity already linked to a different account")]
    IdentityConflict,
    #[error("account not found")]
    NotFound,
    #[error("invalid input: {0}")]
    Invalid(&'static str),
    #[error("serialization: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type StoreResult<T> = Result<T, StoreError>;

/// Persistence interface used by the lobby HTTP layer (Phase 3) and
/// by the auth-flow runtimes (Phase 2.2 onwards). Implemented today by
/// [`SqliteAccountStore`] and the in-memory `MemAccountStore` test
/// double.
#[async_trait]
pub trait AccountStore: Send + Sync {
    /// Look up an account by its `PlayerId`. `None` if the player has
    /// been deleted. Used by `GET /api/v1/me`.
    async fn get_by_player_id(&self, player_id: PlayerId) -> StoreResult<Option<Account>>;

    /// Resolve the account that owns this identity, if any. Used at
    /// the tail of every sign-in flow.
    async fn lookup_by_identity(&self, identity: &Identity) -> StoreResult<Option<Account>>;

    /// Sign-in landing point: if the identity is already linked, return
    /// its account; otherwise mint a fresh `PlayerId`, create an
    /// `accounts` row, and link the identity. Idempotent — repeated
    /// calls with the same identity return the same account.
    async fn find_or_create_account_for_identity(&self, identity: Identity)
    -> StoreResult<Account>;

    /// Add a new identity to an existing account. Refused with
    /// `IdentityConflict` if the (kind, primary_key) is already linked
    /// elsewhere.
    async fn link_identity(&self, player_id: PlayerId, identity: Identity) -> StoreResult<()>;

    /// Remove an identity. The account is left intact even if this was
    /// the last identity — the lobby decides whether to refuse the
    /// final unlink.
    async fn unlink_identity(&self, player_id: PlayerId, identity_id: &str) -> StoreResult<()>;

    /// Patch the editable profile fields. `None` leaves a field alone.
    async fn update_profile(
        &self,
        player_id: PlayerId,
        preferred_name: Option<String>,
        pronouns: Option<String>,
        bio: Option<String>,
        prefs: Option<Preferences>,
    ) -> StoreResult<Account>;

    /// Hard delete: cascades sessions + identities. Lobby Phase 6 GDPR
    /// path.
    async fn delete_account(&self, player_id: PlayerId) -> StoreResult<()>;

    /// List a player's identities alongside their stable row IDs so
    /// the wire can reference them by id (DELETE / set-primary).
    /// Default implementation falls back to enumerating
    /// [`AccountStore::lookup_by_identity`] which obviously can't
    /// recover ids — concrete impls override this. The mem store
    /// returns synthetic ids ("mem:0", "mem:1", …) so tests still
    /// see something stable.
    async fn list_identities_with_ids(
        &self,
        player_id: PlayerId,
    ) -> StoreResult<Vec<(String, Identity)>>;

    /// Flip `verified=true` on the email identity matching `address`
    /// (case-insensitive — `identity_key` already lowercases). Returns
    /// `Ok(true)` if a row was updated, `Ok(false)` if no email
    /// identity matches. Used by the /auth/email/verify path so
    /// every successful magic-link consumption marks the identity
    /// verified — including the dedicated "Verify email" CTA from
    /// Profile, which mints a fresh link for an already-linked
    /// identity.
    async fn mark_email_verified(&self, address: &str) -> StoreResult<bool>;
}

// ───────────────────────────── Sqlite impl ────────────────────────────────────

/// Sqlite-backed `AccountStore`. Default for self-host deploys; tests
/// can use [`MemAccountStore`].
pub struct SqliteAccountStore {
    pool: Pool<Sqlite>,
}

impl SqliteAccountStore {
    /// Open or create a sqlite database at `path` and run the embedded
    /// migrations to bring it to the current schema.
    pub async fn connect(path: impl AsRef<Path>) -> StoreResult<Self> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    /// Run any pending migrations under `open4x-accounts/migrations/`.
    async fn migrate(&self) -> StoreResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Friends-search: resolve `query` (16-hex PlayerId, email,
    /// atproto handle, OpenID URL) to one or more
    /// (player_id, kind, label) hits. Filtered by the candidate's
    /// `Preferences.discoverable_by_id` so users can opt out of
    /// being findable. Returns at most 8 matches to keep the UX
    /// honest.
    pub async fn search_for_friend(&self, query: &str) -> StoreResult<Vec<SearchHit>> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        // Path 1: PlayerId hex (canonical or bare). Single
        // candidate, look up by player_id directly.
        if let Some(pid) = parse_player_id_query(q) {
            let pid_text = format!("{:016X}", pid.0);
            let acct_opt = match load_account(&self.pool, &pid_text).await {
                Ok(a) => Some(a),
                Err(StoreError::NotFound) => None,
                Err(e) => return Err(e),
            };
            return Ok(acct_opt
                .into_iter()
                .filter(|a| a.prefs.discoverable_by_id)
                .map(|a| SearchHit {
                    player_id: a.player_id,
                    kind: "player_id".into(),
                    label: a.player_id.display(),
                })
                .collect());
        }

        // Path 2: identity primary_key match. Email is lower-cased
        // by `identity_key`; atproto + oidc match verbatim. We OR
        // both forms into a single query so the same input can
        // hit multiple identity kinds (rare but legal).
        let q_lower = q.to_lowercase();
        let rows: Vec<IdentityRow> = sqlx::query_as::<_, IdentityRow>(
            "SELECT id, player_id, kind, primary_key, label, is_primary, verified \
             FROM identities \
             WHERE primary_key = ?1 OR primary_key = ?2 \
             LIMIT 8",
        )
        .bind(q)
        .bind(&q_lower)
        .fetch_all(&self.pool)
        .await?;

        let mut hits = Vec::new();
        for r in rows {
            let acct = match load_account(&self.pool, &r.player_id).await {
                Ok(a) => a,
                Err(StoreError::NotFound) => continue,
                Err(e) => return Err(e),
            };
            if !acct.prefs.discoverable_by_id {
                continue;
            }
            hits.push(SearchHit {
                player_id: acct.player_id,
                kind: r.kind,
                label: r.label,
            });
        }
        Ok(hits)
    }
}

/// One hit from [`SqliteAccountStore::search_for_friend`].
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub player_id: PlayerId,
    /// `email` / `oidc` / `atproto` / `player_id`.
    pub kind: String,
    pub label: String,
}

fn parse_player_id_query(s: &str) -> Option<PlayerId> {
    let body = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    let stripped: String = body.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if stripped.len() != 16 {
        return None;
    }
    Some(PlayerId::new(u64::from_str_radix(&stripped, 16).ok()?))
}

#[async_trait]
impl AccountStore for SqliteAccountStore {
    async fn get_by_player_id(&self, player_id: PlayerId) -> StoreResult<Option<Account>> {
        let player_id_text = player_id_text(&player_id);
        match load_account(&self.pool, &player_id_text).await {
            Ok(a) => Ok(Some(a)),
            Err(StoreError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn lookup_by_identity(&self, identity: &Identity) -> StoreResult<Option<Account>> {
        let (kind, primary_key) = identity_key(identity);
        let row: Option<IdentityRow> = sqlx::query_as::<_, IdentityRow>(
            "SELECT id, player_id, kind, primary_key, label, is_primary, verified \
             FROM identities WHERE kind = ?1 AND primary_key = ?2",
        )
        .bind(kind)
        .bind(primary_key)
        .fetch_optional(&self.pool)
        .await?;

        let Some(idr) = row else { return Ok(None) };
        load_account(&self.pool, &idr.player_id).await.map(Some)
    }

    async fn find_or_create_account_for_identity(
        &self,
        identity: Identity,
    ) -> StoreResult<Account> {
        if let Some(existing) = self.lookup_by_identity(&identity).await? {
            return Ok(existing);
        }
        // No existing account — mint a new one and link in a single
        // transaction.
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().to_rfc3339();
        let player_id = PlayerId::new(rand_player_id());
        let player_id_text = player_id_text(&player_id);
        sqlx::query(
            "INSERT INTO accounts (player_id, preferred_name, pronouns, bio, \
                                   prefs_json, created_at, updated_at) \
             VALUES (?1, '', '', '', '{}', ?2, ?2)",
        )
        .bind(&player_id_text)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        insert_identity(&mut tx, &player_id_text, &identity, &now).await?;
        tx.commit().await?;

        load_account(&self.pool, &player_id_text).await
    }

    async fn link_identity(&self, player_id: PlayerId, identity: Identity) -> StoreResult<()> {
        let now = Utc::now().to_rfc3339();
        let player_id_text = player_id_text(&player_id);
        let mut tx = self.pool.begin().await?;
        let result = insert_identity(&mut tx, &player_id_text, &identity, &now).await;
        match result {
            Ok(_) => {
                tx.commit().await?;
                Ok(())
            }
            Err(StoreError::Sqlx(sqlx::Error::Database(db))) if is_unique_violation(&*db) => {
                Err(StoreError::IdentityConflict)
            }
            Err(other) => Err(other),
        }
    }

    async fn unlink_identity(&self, player_id: PlayerId, identity_id: &str) -> StoreResult<()> {
        let player_id_text = player_id_text(&player_id);
        let res = sqlx::query("DELETE FROM identities WHERE id = ?1 AND player_id = ?2")
            .bind(identity_id)
            .bind(&player_id_text)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    async fn update_profile(
        &self,
        player_id: PlayerId,
        preferred_name: Option<String>,
        pronouns: Option<String>,
        bio: Option<String>,
        prefs: Option<Preferences>,
    ) -> StoreResult<Account> {
        let player_id_text = player_id_text(&player_id);
        let now = Utc::now().to_rfc3339();
        let prefs_json = match &prefs {
            Some(p) => Some(serde_json::to_string(p)?),
            None => None,
        };
        sqlx::query(
            "UPDATE accounts \
             SET preferred_name = COALESCE(?1, preferred_name), \
                 pronouns       = COALESCE(?2, pronouns), \
                 bio            = COALESCE(?3, bio), \
                 prefs_json     = COALESCE(?4, prefs_json), \
                 updated_at     = ?5 \
             WHERE player_id = ?6",
        )
        .bind(preferred_name)
        .bind(pronouns)
        .bind(bio)
        .bind(prefs_json)
        .bind(&now)
        .bind(&player_id_text)
        .execute(&self.pool)
        .await?;
        load_account(&self.pool, &player_id_text).await
    }

    async fn delete_account(&self, player_id: PlayerId) -> StoreResult<()> {
        let player_id_text = player_id_text(&player_id);
        // Two-step GDPR cascade in one transaction:
        //   1. Repoint foreign `game_members` rows (where the
        //      deleted player is a member of someone else's game)
        //      to the sentinel row from migration 0005. The host's
        //      roster keeps a placeholder instead of losing the row.
        //   2. DELETE the account. The schema's existing FK
        //      cascades clean up sessions, identities, owned games,
        //      and the deleted player's own game_members rows.
        // OR IGNORE on the UPDATE: if the same foreign game already
        // has a sentinel row from a prior delete, the conflict-row
        // gets cascaded out at step 2 instead.
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE OR IGNORE game_members \
             SET player_id = '0000000000000000' \
             WHERE player_id = ?1 \
               AND game_id NOT IN (\
                   SELECT game_id FROM games WHERE owner_player_id = ?1\
               )",
        )
        .bind(&player_id_text)
        .execute(&mut *tx)
        .await?;
        let res = sqlx::query("DELETE FROM accounts WHERE player_id = ?1")
            .bind(&player_id_text)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_identities_with_ids(
        &self,
        player_id: PlayerId,
    ) -> StoreResult<Vec<(String, Identity)>> {
        let pid = player_id_text(&player_id);
        let rows: Vec<IdentityRow> = sqlx::query_as::<_, IdentityRow>(
            "SELECT id, player_id, kind, primary_key, label, is_primary, verified \
             FROM identities WHERE player_id = ?1",
        )
        .bind(&pid)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| (r.id.clone(), row_to_identity(r)))
            .collect())
    }

    async fn mark_email_verified(&self, address: &str) -> StoreResult<bool> {
        let key = address.to_lowercase();
        let res = sqlx::query(
            "UPDATE identities SET verified = 1 \
             WHERE kind = 'email' AND primary_key = ?1 AND verified = 0",
        )
        .bind(&key)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }
}

// ───────────────────────────── Helpers ────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct IdentityRow {
    id: String,
    player_id: String,
    kind: String,
    primary_key: String,
    label: String,
    is_primary: i64,
    verified: i64,
}

#[derive(sqlx::FromRow)]
struct AccountRow {
    player_id: String,
    preferred_name: String,
    pronouns: String,
    bio: String,
    prefs_json: String,
}

fn identity_key(identity: &Identity) -> (&'static str, String) {
    match identity {
        Identity::Email { address, .. } => ("email", address.to_lowercase()),
        Identity::OpenId {
            issuer, subject, ..
        } => ("oidc", format!("{issuer}|{subject}")),
        Identity::Atproto { did, .. } => ("atproto", did.clone()),
        Identity::PublicKey { ed25519_hex, .. } => ("pubkey", ed25519_hex.to_lowercase()),
    }
}

fn rand_player_id() -> u64 {
    // Derive a u64 from a fresh ULID so the IdGenerator path stays
    // single-source (ULIDs are seeded with rand under the hood).

    Ulid::new().0 as u64
}

fn player_id_text(id: &PlayerId) -> String {
    format!("{:016X}", id.0)
}

fn parse_player_id_text(s: &str) -> Result<PlayerId, StoreError> {
    let raw = u64::from_str_radix(s, 16).map_err(|_| StoreError::Invalid("player_id"))?;
    Ok(PlayerId::new(raw))
}

fn is_unique_violation(err: &(dyn sqlx::error::DatabaseError)) -> bool {
    err.code()
        .map(|c| c == "2067" /* SQLITE_CONSTRAINT_UNIQUE */ || c == "1555" /* SQLITE_CONSTRAINT_PRIMARYKEY */)
        .unwrap_or(false)
        || err.message().contains("UNIQUE")
}

async fn insert_identity(
    tx: &mut sqlx::SqliteConnection,
    player_id_text: &str,
    identity: &Identity,
    now: &str,
) -> StoreResult<()> {
    let id = Ulid::new().to_string();
    let (kind, primary_key) = identity_key(identity);
    let label = identity.label();
    let (is_primary, verified) = match identity {
        Identity::Email {
            primary, verified, ..
        } => (*primary as i64, *verified as i64),
        _ => (0, 1),
    };
    sqlx::query(
        "INSERT INTO identities (id, player_id, kind, primary_key, label, \
                                 is_primary, verified, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(&id)
    .bind(player_id_text)
    .bind(kind)
    .bind(&primary_key)
    .bind(&label)
    .bind(is_primary)
    .bind(verified)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

async fn load_account(pool: &Pool<Sqlite>, player_id_text: &str) -> StoreResult<Account> {
    let row: AccountRow = sqlx::query_as::<_, AccountRow>(
        "SELECT player_id, preferred_name, pronouns, bio, prefs_json \
         FROM accounts WHERE player_id = ?1",
    )
    .bind(player_id_text)
    .fetch_optional(pool)
    .await?
    .ok_or(StoreError::NotFound)?;
    let prefs: Preferences = serde_json::from_str(&row.prefs_json).unwrap_or_default();
    let identities = load_identities(pool, &row.player_id).await?;
    let player_id = parse_player_id_text(&row.player_id)?;
    Ok(Account {
        player_id,
        preferred_name: row.preferred_name,
        pronouns: row.pronouns,
        bio: row.bio,
        identities,
        prefs,
    })
}

async fn load_identities(pool: &Pool<Sqlite>, player_id_text: &str) -> StoreResult<Vec<Identity>> {
    let rows: Vec<IdentityRow> = sqlx::query_as::<_, IdentityRow>(
        "SELECT id, player_id, kind, primary_key, label, is_primary, verified \
         FROM identities WHERE player_id = ?1",
    )
    .bind(player_id_text)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(row_to_identity(&r));
    }
    Ok(out)
}

fn row_to_identity(r: &IdentityRow) -> Identity {
    match r.kind.as_str() {
        "email" => Identity::Email {
            address: r.primary_key.clone(),
            verified: r.verified != 0,
            primary: r.is_primary != 0,
        },
        "oidc" => {
            let (issuer, subject) = r
                .primary_key
                .split_once('|')
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .unwrap_or_else(|| (r.primary_key.clone(), String::new()));
            Identity::OpenId {
                issuer,
                subject,
                label: r.label.clone(),
            }
        }
        "atproto" => Identity::Atproto {
            did: r.primary_key.clone(),
            handle: r.label.clone(),
        },
        "pubkey" => Identity::PublicKey {
            ed25519_hex: r.primary_key.clone(),
            label: r.label.clone(),
        },
        _ => Identity::Email {
            address: r.primary_key.clone(),
            verified: r.verified != 0,
            primary: r.is_primary != 0,
        },
    }
}

// ───────────────────────────── In-memory test impl ────────────────────────────

#[cfg(test)]
mod mem {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    pub struct MemAccountStore {
        accounts: Mutex<HashMap<PlayerId, Account>>,
    }

    impl Default for MemAccountStore {
        fn default() -> Self {
            Self {
                accounts: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl AccountStore for MemAccountStore {
        async fn get_by_player_id(&self, player_id: PlayerId) -> StoreResult<Option<Account>> {
            Ok(self.accounts.lock().unwrap().get(&player_id).cloned())
        }

        async fn lookup_by_identity(&self, identity: &Identity) -> StoreResult<Option<Account>> {
            let key = identity_key(identity);
            let map = self.accounts.lock().unwrap();
            for acct in map.values() {
                for id in &acct.identities {
                    if identity_key(id) == key {
                        return Ok(Some(acct.clone()));
                    }
                }
            }
            Ok(None)
        }

        async fn find_or_create_account_for_identity(
            &self,
            identity: Identity,
        ) -> StoreResult<Account> {
            if let Some(a) = self.lookup_by_identity(&identity).await? {
                return Ok(a);
            }
            let player_id = PlayerId::new(rand_player_id());
            let acct = Account {
                player_id,
                identities: vec![identity],
                ..Account::default()
            };
            self.accounts
                .lock()
                .unwrap()
                .insert(player_id, acct.clone());
            Ok(acct)
        }

        async fn link_identity(&self, player_id: PlayerId, identity: Identity) -> StoreResult<()> {
            // Conflict if another account already has this identity.
            if let Some(other) = self.lookup_by_identity(&identity).await? {
                if other.player_id != player_id {
                    return Err(StoreError::IdentityConflict);
                }
                return Ok(());
            }
            let mut map = self.accounts.lock().unwrap();
            let acct = map.get_mut(&player_id).ok_or(StoreError::NotFound)?;
            acct.identities.push(identity);
            Ok(())
        }

        async fn unlink_identity(
            &self,
            player_id: PlayerId,
            _identity_id: &str,
        ) -> StoreResult<()> {
            // The mem store doesn't carry stable identity IDs; tests use
            // direct field manipulation.
            let mut map = self.accounts.lock().unwrap();
            map.get_mut(&player_id).ok_or(StoreError::NotFound)?;
            Ok(())
        }

        async fn update_profile(
            &self,
            player_id: PlayerId,
            preferred_name: Option<String>,
            pronouns: Option<String>,
            bio: Option<String>,
            prefs: Option<Preferences>,
        ) -> StoreResult<Account> {
            let mut map = self.accounts.lock().unwrap();
            let acct = map.get_mut(&player_id).ok_or(StoreError::NotFound)?;
            if let Some(v) = preferred_name {
                acct.preferred_name = v;
            }
            if let Some(v) = pronouns {
                acct.pronouns = v;
            }
            if let Some(v) = bio {
                acct.bio = v;
            }
            if let Some(v) = prefs {
                acct.prefs = v;
            }
            Ok(acct.clone())
        }

        async fn delete_account(&self, player_id: PlayerId) -> StoreResult<()> {
            self.accounts
                .lock()
                .unwrap()
                .remove(&player_id)
                .ok_or(StoreError::NotFound)
                .map(|_| ())
        }

        async fn list_identities_with_ids(
            &self,
            player_id: PlayerId,
        ) -> StoreResult<Vec<(String, Identity)>> {
            let map = self.accounts.lock().unwrap();
            let acct = map.get(&player_id).ok_or(StoreError::NotFound)?;
            Ok(acct
                .identities
                .iter()
                .enumerate()
                .map(|(i, id)| (format!("mem:{i}"), id.clone()))
                .collect())
        }

        async fn mark_email_verified(&self, address: &str) -> StoreResult<bool> {
            let key = address.to_lowercase();
            let mut map = self.accounts.lock().unwrap();
            for acct in map.values_mut() {
                for ident in acct.identities.iter_mut() {
                    if let Identity::Email {
                        address: a,
                        verified,
                        ..
                    } = ident
                    {
                        if a.to_lowercase() == key && !*verified {
                            *verified = true;
                            return Ok(true);
                        }
                    }
                }
            }
            Ok(false)
        }
    }

    #[tokio::test]
    async fn find_or_create_is_idempotent() {
        let store = MemAccountStore::default();
        let id = Identity::Email {
            address: "alice@example.com".into(),
            verified: false,
            primary: true,
        };
        let a = store
            .find_or_create_account_for_identity(id.clone())
            .await
            .unwrap();
        let b = store.find_or_create_account_for_identity(id).await.unwrap();
        assert_eq!(a.player_id, b.player_id);
    }

    #[tokio::test]
    async fn delete_account_anonymises_foreign_game_membership() {
        use crate::games::{GameStore, NewGame, SqliteGameStore};

        struct TempFile(std::path::PathBuf);
        impl Drop for TempFile {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
                let _ = std::fs::remove_file(self.0.with_extension("sqlite-wal"));
                let _ = std::fs::remove_file(self.0.with_extension("sqlite-shm"));
            }
        }

        // tempfile keeps the Sqlite pool sharing one disk-backed
        // db; `:memory:` would give each pool connection its own
        // empty database.
        let mut p = std::env::temp_dir();
        p.push(format!(
            "open4x_anon_{}_{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let tmp = TempFile(p);
        let path = &tmp.0;

        let store = SqliteAccountStore::connect(&path).await.unwrap();
        let games = SqliteGameStore::from_pool(store.pool.clone());

        // Two real accounts: bob hosts a game, alice joins as a
        // member. (Two separate emails so they get distinct
        // player_ids.)
        let alice = store
            .find_or_create_account_for_identity(Identity::Email {
                address: "alice@example.com".into(),
                verified: true,
                primary: true,
            })
            .await
            .unwrap();
        let bob = store
            .find_or_create_account_for_identity(Identity::Email {
                address: "bob@example.com".into(),
                verified: true,
                primary: true,
            })
            .await
            .unwrap();

        // Bob owns this game — `create_game` seeds an owner row
        // in `game_members`. We then add alice as a foreign
        // member.
        let g = games
            .create_game(NewGame {
                owner_player_id: bob.player_id,
                name: "Bob's Game".into(),
                leader: "Bob".into(),
                civ_id: "ROME".into(),
                difficulty: "prince".into(),
                players_human: 2,
                players_ai: 0,
                map_type: "continents".into(),
                map_size: "tiny".into(),
                seed: "0xABCD".into(),
                server_url: String::new(),
                server_token: String::new(),
            })
            .await
            .unwrap();
        let alice_pid = format!("{:016X}", alice.player_id.0);
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO game_members (game_id, player_id, role, invited_at) \
             VALUES (?1, ?2, 'player', ?3)",
        )
        .bind(&g.game_id)
        .bind(&alice_pid)
        .bind(&now)
        .execute(&store.pool)
        .await
        .unwrap();

        // Sanity: alice's membership row exists.
        let pre: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM game_members WHERE game_id = ?1 AND player_id = ?2",
        )
        .bind(&g.game_id)
        .bind(&alice_pid)
        .fetch_one(&store.pool)
        .await
        .unwrap();
        assert_eq!(pre.0, 1, "precondition: alice is in bob's game");

        // GDPR delete.
        store.delete_account(alice.player_id).await.unwrap();

        // Bob's game still exists.
        let game_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM games WHERE game_id = ?1")
            .bind(&g.game_id)
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(
            game_count.0, 1,
            "host's game must survive a member's deletion"
        );

        // Alice's row is gone, but a sentinel-anonymised row took
        // its place — i.e. the host still has a non-empty roster.
        let alice_left: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM game_members WHERE game_id = ?1 AND player_id = ?2",
        )
        .bind(&g.game_id)
        .bind(&alice_pid)
        .fetch_one(&store.pool)
        .await
        .unwrap();
        assert_eq!(alice_left.0, 0, "alice's member row must be repointed");
        let sentinel_present: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM game_members \
             WHERE game_id = ?1 AND player_id = '0000000000000000'",
        )
        .bind(&g.game_id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
        assert_eq!(sentinel_present.0, 1, "sentinel row must replace alice");
    }

    #[tokio::test]
    async fn link_identity_rejects_cross_account_dup() {
        let store = MemAccountStore::default();
        let alice = Identity::Email {
            address: "alice@example.com".into(),
            verified: false,
            primary: true,
        };
        let bob = Identity::Email {
            address: "bob@example.com".into(),
            verified: false,
            primary: true,
        };
        let _a = store
            .find_or_create_account_for_identity(alice.clone())
            .await
            .unwrap();
        let b = store
            .find_or_create_account_for_identity(bob)
            .await
            .unwrap();
        let err = store.link_identity(b.player_id, alice).await.unwrap_err();
        assert!(matches!(err, StoreError::IdentityConflict));
    }
}
