//! Shared `AppState` plumbed into every Axum handler.

#![cfg(feature = "ssr")]

use std::path::PathBuf;
use std::sync::Arc;

use ipnet::IpNet;
use open4x_accounts::audit::SqliteAuditStore;
use open4x_accounts::friends::SqliteFriendsStore;
use open4x_accounts::games::SqliteGameStore;
use open4x_accounts::magic_link::MagicLinkSigner;
use open4x_accounts::mailer::{LogMailer, Mailer, SmtpConfig, SmtpMailer};
use open4x_accounts::presets::SqlitePresetsStore;
use open4x_accounts::store::SqliteAccountStore;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};

use crate::server::client_ip::parse_trusted_proxies;
use crate::server::process::{DeployMode, ProcessConfig, ProcessOrchestrator};
use crate::server::pubkey_challenge::PubkeyChallengeStore;

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<Sqlite>,
    pub store: Arc<SqliteAccountStore>,
    pub games: Arc<SqliteGameStore>,
    pub audit: Arc<SqliteAuditStore>,
    pub friends: Arc<SqliteFriendsStore>,
    pub presets: Arc<SqlitePresetsStore>,
    pub signer: MagicLinkSigner,
    pub mailer: Arc<dyn Mailer>,
    /// Base URL for outgoing magic-link URLs (e.g. `https://lobby.example`).
    /// Empty string means "use the current request's Host header" — fine
    /// for dev / single-machine.
    pub public_base_url: String,
    /// URL of the open4x-server instance the lobby orchestrates new
    /// games against. Defaults to `http://localhost:3001`. Empty
    /// disables the orchestrator (game rows ship with empty
    /// `server_url` / `server_token` and Resume returns 503).
    pub game_server_url: String,
    /// CIDRs whose direct TCP peers are trusted to set
    /// `X-Forwarded-For`. When the request peer matches one of
    /// these, [`crate::server::client_ip`] reads the client address
    /// out of the header instead. Empty (the default) means the
    /// header is always ignored.
    pub trusted_proxies: Vec<IpNet>,
    /// Selected at boot via `OPEN4X_LOBBY_PER_GAME`. When
    /// `PerGame`, [`process_orch`] is `Some` and `games::create`
    /// spawns a fresh `open4x-server` per row.
    pub deploy_mode: DeployMode,
    /// Process registry — only set when [`deploy_mode`] is
    /// `PerGame`. Cloneable; cheap to hand out to handlers.
    pub process_orch: Option<ProcessOrchestrator>,
    /// Where the avatar pipeline writes per-player PNGs. Always
    /// `<data_dir>/avatars/`. Files are named
    /// `<player_id_hex>.png` and served at `/avatars/<file>` by
    /// the binary's `ServeDir`.
    pub avatar_dir: PathBuf,
    /// Process-local, single-use nonce store for the pubkey
    /// challenge-response auth flow. Ephemeral — see
    /// [`crate::server::pubkey_challenge`].
    pub pubkey_challenges: PubkeyChallengeStore,
}

impl AppState {
    /// Boot the AppState: open / migrate the sqlite db at `data_dir/accounts.sqlite`,
    /// load (or generate) the HMAC key at `data_dir/lobby.key`, default to
    /// `LogMailer`. The lobby's `main.rs` calls this once at startup.
    pub async fn boot(data_dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let data_dir: PathBuf = data_dir.into();
        std::fs::create_dir_all(&data_dir)?;

        // Open + migrate the sqlite db. We re-open via SqliteAccountStore
        // for the trait surface, but ALSO keep a Pool around for the
        // session/magic-link helpers (they take a `&Pool<Sqlite>` directly).
        let db_path = data_dir.join("accounts.sqlite");
        let opts = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .pragma("foreign_keys", "ON");
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        sqlx::migrate!("../open4x-accounts/migrations")
            .run(&pool)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let store = SqliteAccountStore::connect(&db_path)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let games = SqliteGameStore::from_pool(pool.clone());
        let audit = SqliteAuditStore::from_pool(pool.clone());
        let friends = SqliteFriendsStore::from_pool(pool.clone());
        let presets = SqlitePresetsStore::from_pool(pool.clone());

        let signer = MagicLinkSigner::from_env_or_path(data_dir.join("lobby.key"))
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        // Prefer SMTP when fully configured; fall back to LogMailer
        // so dev / CI keep printing the magic link to stderr.
        let mailer: Arc<dyn Mailer> = match SmtpConfig::from_env() {
            Some(cfg) => match SmtpMailer::new(cfg) {
                Ok(m) => {
                    eprintln!("[mailer] SMTP configured — magic links go via lettre");
                    Arc::new(m)
                }
                Err(e) => {
                    eprintln!("[mailer] SMTP config rejected ({e}); falling back to LogMailer");
                    Arc::new(LogMailer)
                }
            },
            None => Arc::new(LogMailer),
        };
        let public_base_url = std::env::var("OPEN4X_LOBBY_PUBLIC_URL").unwrap_or_default();
        let game_server_url = std::env::var("OPEN4X_GAME_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());
        let trusted_proxies = parse_trusted_proxies(
            &std::env::var("OPEN4X_LOBBY_TRUSTED_PROXIES").unwrap_or_default(),
        );

        let deploy_mode = DeployMode::from_env();
        let process_orch = match deploy_mode {
            DeployMode::PerGame => {
                let cfg = ProcessConfig::from_env();
                eprintln!(
                    "[orchestrator] per-game mode: binary={} ports={}-{} data_root={} public_url={}",
                    cfg.binary.display(),
                    cfg.port_lo,
                    cfg.port_hi,
                    cfg.data_root.display(),
                    cfg.public_url_template
                        .as_deref()
                        .unwrap_or("<none, loopback only>"),
                );
                Some(ProcessOrchestrator::new(cfg))
            }
            DeployMode::Shared => None,
        };

        let avatar_dir = data_dir.join("avatars");
        std::fs::create_dir_all(&avatar_dir)?;

        Ok(Self {
            pool,
            store: Arc::new(store),
            games: Arc::new(games),
            audit: Arc::new(audit),
            friends: Arc::new(friends),
            presets: Arc::new(presets),
            signer,
            mailer,
            public_base_url,
            game_server_url,
            trusted_proxies,
            deploy_mode,
            process_orch,
            avatar_dir,
            pubkey_challenges: PubkeyChallengeStore::new(),
        })
    }
}
