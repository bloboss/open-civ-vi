//! Process-per-game orchestrator (Phase 6).
//!
//! When `OPEN4X_LOBBY_PER_GAME` is set, the lobby spawns a fresh
//! `open4x-server` instance per new game on a free port from a
//! configured range, polls the new instance's `/health`, and then
//! delegates to the existing [`super::orchestrator::bootstrap_game`]
//! to mint a GameRoom on it. The spawned child is held in a
//! per-AppState registry so its `tokio::process::Child` lives as
//! long as the lobby — losing the registry kills the child.
//!
//! Deploy story: per-game OOM / panic doesn't take down everyone
//! else's roster, only the affected game's tab.

#![cfg(feature = "ssr")]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{Instant, sleep};

use super::orchestrator::{self, BootstrappedGame, NewGameRequest, OrchestratorError};

/// Deploy mode selected at boot from `OPEN4X_LOBBY_PER_GAME`. The
/// shared variant matches the original v1 model: one configured
/// `open4x-server` and many GameRooms by id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployMode {
    Shared,
    PerGame,
}

impl DeployMode {
    pub fn from_env() -> Self {
        match std::env::var("OPEN4X_LOBBY_PER_GAME").as_deref() {
            Ok("1") | Ok("true") | Ok("yes") => DeployMode::PerGame,
            _ => DeployMode::Shared,
        }
    }
}

/// One spawned `open4x-server` instance bound to a single game.
#[derive(Debug)]
pub struct GameProcess {
    pub port: u16,
    pub pid: Option<u32>,
    pub url: String,
    /// Held only to keep the child alive. Dropping this `Child`
    /// kills the spawned process (tokio's `Child` defaults to
    /// `kill_on_drop`, but we force it explicitly when spawning).
    child: Child,
}

impl GameProcess {
    /// Best-effort kill — used by [`ProcessOrchestrator::stop_game`]
    /// and on process-registry shutdown.
    pub async fn shutdown(&mut self) {
        let _ = self.child.kill().await;
    }
}

/// Configuration for the per-game orchestrator. Parsed once at
/// boot from env.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Path to the `open4x-server` binary to spawn. Defaults to
    /// looking up `open4x-server` on `PATH` via `Command::new`.
    pub binary: PathBuf,
    /// Inclusive port range to allocate from. Defaults to
    /// `4001..=4100`.
    pub port_lo: u16,
    pub port_hi: u16,
    /// Per-game data dir parent. Each game gets a subdir named by
    /// the lobby's `game_id`. Defaults to `./data/per-game`.
    pub data_root: PathBuf,
    /// How long to wait for the spawned child's `/health` to flip
    /// to 200 before giving up. Defaults to 10 seconds.
    pub health_timeout: Duration,
    /// Public URL template that the *browser* should hit when
    /// resuming a game in this instance. The substring `{port}`
    /// is substituted with the per-game allocated port; everything
    /// else is taken literally. Examples:
    ///   `https://g-{port}.example.com`     — wildcard subdomain
    ///   `https://example.com/play/{port}`  — path-prefix routing
    /// When unset, the lobby writes the in-process URL
    /// `http://127.0.0.1:<port>` directly — fine for localhost
    /// dev, broken for any deploy where the browser isn't on the
    /// lobby host. See `book/src/multiplayer/reverse-proxy.md`.
    pub public_url_template: Option<String>,
}

impl ProcessConfig {
    pub fn from_env() -> Self {
        let binary = std::env::var("OPEN4X_LOBBY_GAME_BINARY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("open4x-server"));
        let (port_lo, port_hi) = parse_port_range(
            std::env::var("OPEN4X_LOBBY_PORT_RANGE")
                .as_deref()
                .unwrap_or("4001-4100"),
        )
        .unwrap_or((4001, 4100));
        let data_root = std::env::var("OPEN4X_LOBBY_PER_GAME_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/per-game"));
        let health_timeout = Duration::from_secs(
            std::env::var("OPEN4X_LOBBY_HEALTH_TIMEOUT_S")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(10),
        );
        let public_url_template = std::env::var("OPEN4X_LOBBY_PUBLIC_GAME_URL_TEMPLATE")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Self {
            binary,
            port_lo,
            port_hi,
            data_root,
            health_timeout,
            public_url_template,
        }
    }
}

/// Substitute `{port}` in `template` with the literal port
/// number. If the template is `None`, returns the local URL
/// (`http://127.0.0.1:<port>`) unchanged.
pub fn render_public_url(template: Option<&str>, port: u16, fallback: &str) -> String {
    match template {
        Some(t) => t.replace("{port}", &port.to_string()),
        None => fallback.to_string(),
    }
}

fn parse_port_range(s: &str) -> Option<(u16, u16)> {
    let (lo, hi) = s.split_once('-')?;
    let lo: u16 = lo.trim().parse().ok()?;
    let hi: u16 = hi.trim().parse().ok()?;
    if lo == 0 || hi < lo {
        return None;
    }
    Some((lo, hi))
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("no free port in range {lo}-{hi}")]
    NoFreePort { lo: u16, hi: u16 },
    #[error("spawn {binary}: {source}")]
    Spawn {
        binary: String,
        #[source]
        source: std::io::Error,
    },
    #[error("data dir {dir}: {source}")]
    DataDir {
        dir: String,
        #[source]
        source: std::io::Error,
    },
    #[error("/health on {url} did not return 200 within {secs}s")]
    HealthTimeout { url: String, secs: u64 },
    #[error(transparent)]
    Bootstrap(#[from] OrchestratorError),
}

/// Owns the registry of spawned children + the port allocator.
/// Cloneable: it's a wrapper around `Arc`s, so AppState can hold
/// one and share it across handlers.
#[derive(Clone)]
pub struct ProcessOrchestrator {
    cfg: ProcessConfig,
    inner: Arc<OrchestratorInner>,
}

struct OrchestratorInner {
    /// Game-id → spawned process. Use the LOBBY game id as key,
    /// not the remote one (we register the entry before bootstrap
    /// returns, so we can map back to it on shutdown).
    procs: Mutex<HashMap<String, GameProcess>>,
    /// Set of ports currently held by procs above. Lets the
    /// allocator skip ports we've handed out without scanning the
    /// registry.
    in_use: Mutex<HashSet<u16>>,
}

impl ProcessOrchestrator {
    pub fn new(cfg: ProcessConfig) -> Self {
        Self {
            cfg,
            inner: Arc::new(OrchestratorInner {
                procs: Mutex::new(HashMap::new()),
                in_use: Mutex::new(HashSet::new()),
            }),
        }
    }

    pub fn config(&self) -> &ProcessConfig {
        &self.cfg
    }

    /// Spawn a fresh `open4x-server`, wait for `/health`, then
    /// delegate to [`bootstrap_game`] to create the GameRoom. The
    /// resulting [`GameProcess`] is parked under the open4x-server's
    /// `remote_game_id` so [`stop_game`] can clean up later.
    pub async fn bootstrap_per_game(
        &self,
        req: &NewGameRequest,
    ) -> Result<BootstrappedGame, ProcessError> {
        let port = self.allocate_port().await?;
        let url = format!("http://127.0.0.1:{port}");
        // Per-game data dir keyed by port at first; renamed to
        // `<remote_game_id>` after bootstrap so leftover dirs are
        // findable by id in the on-disk layout.
        let scratch_dir = self.cfg.data_root.join(format!("port-{port}"));
        std::fs::create_dir_all(&scratch_dir).map_err(|e| ProcessError::DataDir {
            dir: scratch_dir.display().to_string(),
            source: e,
        })?;

        let mut child = match Command::new(&self.cfg.binary)
            .env("PORT", port.to_string())
            .env("OPEN4X_DATA_DIR", &scratch_dir)
            // Don't share the lobby's stdin — tokio defaults to
            // inherit, which can wedge spawned children if the
            // parent's tty is closed. Stdout/stderr stay inherited
            // so per-game logs land in the lobby's journal.
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                self.release_port(port).await;
                return Err(ProcessError::Spawn {
                    binary: self.cfg.binary.display().to_string(),
                    source: e,
                });
            }
        };
        let pid = child.id();

        // Health-poll the new instance. If it never comes up, kill
        // the half-spawned child and free the port before bailing.
        if let Err(e) = wait_for_health(&url, self.cfg.health_timeout).await {
            let _ = child.kill().await;
            self.release_port(port).await;
            return Err(e);
        }

        let mut bootstrapped = match orchestrator::bootstrap_game(&url, req).await {
            Ok(b) => b,
            Err(e) => {
                let _ = child.kill().await;
                self.release_port(port).await;
                return Err(e.into());
            }
        };
        // Rewrite the URL the *games row* (and therefore the
        // browser at Resume time) sees if a public template is
        // configured. The lobby itself keeps talking to the child
        // over the loopback URL stored in `GameProcess.url`.
        bootstrapped.server_url =
            render_public_url(self.cfg.public_url_template.as_deref(), port, &url);

        let proc = GameProcess {
            port,
            pid,
            url: url.clone(),
            child,
        };
        self.inner
            .procs
            .lock()
            .await
            .insert(bootstrapped.remote_game_id.clone(), proc);
        Ok(bootstrapped)
    }

    /// Best-effort: kill the spawned child for `remote_game_id`, if
    /// any, and free its port. Idempotent.
    pub async fn stop_game(&self, remote_game_id: &str) {
        let removed = self.inner.procs.lock().await.remove(remote_game_id);
        if let Some(mut p) = removed {
            p.shutdown().await;
            self.release_port(p.port).await;
        }
    }

    /// For ops / debug: how many children are currently alive.
    pub async fn live_count(&self) -> usize {
        self.inner.procs.lock().await.len()
    }

    async fn allocate_port(&self) -> Result<u16, ProcessError> {
        let mut held = self.inner.in_use.lock().await;
        for p in self.cfg.port_lo..=self.cfg.port_hi {
            if held.contains(&p) {
                continue;
            }
            // OS-level liveness probe: if something else is
            // already bound to the port, skip it. We bind+drop a
            // listener to confirm it's free before claiming. Note
            // there's an inherent TOCTOU race against the eventual
            // child bind — that's fine, the child will fail-fast
            // and the next request will pick a different slot.
            if std::net::TcpListener::bind(("127.0.0.1", p)).is_ok() {
                held.insert(p);
                return Ok(p);
            }
        }
        Err(ProcessError::NoFreePort {
            lo: self.cfg.port_lo,
            hi: self.cfg.port_hi,
        })
    }

    async fn release_port(&self, port: u16) {
        self.inner.in_use.lock().await.remove(&port);
    }
}

/// Poll `<url>/health` until it returns 2xx or we hit `deadline`.
async fn wait_for_health(url: &str, timeout: Duration) -> Result<(), ProcessError> {
    let url = format!("{}/health", url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .map_err(|e| ProcessError::Bootstrap(OrchestratorError::Transport(e.to_string())))?;
    let started = Instant::now();
    let mut backoff = Duration::from_millis(100);
    loop {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        if started.elapsed() >= timeout {
            return Err(ProcessError::HealthTimeout {
                url,
                secs: timeout.as_secs(),
            });
        }
        sleep(backoff).await;
        // Slow gradually so we don't hammer the boot path.
        backoff = (backoff * 2).min(Duration::from_millis(800));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_port_range() {
        assert_eq!(parse_port_range("4001-4100"), Some((4001, 4100)));
        assert_eq!(parse_port_range("4001 - 4100"), Some((4001, 4100)));
        assert_eq!(parse_port_range("4100-4001"), None);
        assert_eq!(parse_port_range("badrange"), None);
        assert_eq!(parse_port_range("0-100"), None);
    }

    #[test]
    fn renders_public_url_with_and_without_template() {
        assert_eq!(
            render_public_url(None, 4501, "http://127.0.0.1:4501"),
            "http://127.0.0.1:4501",
        );
        assert_eq!(
            render_public_url(Some("https://g-{port}.example.com"), 4501, "ignored"),
            "https://g-4501.example.com",
        );
        assert_eq!(
            render_public_url(Some("https://example.com/play/{port}"), 4502, "ignored"),
            "https://example.com/play/4502",
        );
        // Template without {port} substitution is taken literally.
        assert_eq!(
            render_public_url(Some("https://example.com/static"), 4501, "ignored"),
            "https://example.com/static",
        );
    }

    #[test]
    fn deploy_mode_default_shared_when_unset() {
        // SAFETY: edition 2024 — single-threaded test reads env.
        unsafe {
            std::env::remove_var("OPEN4X_LOBBY_PER_GAME");
        }
        assert_eq!(DeployMode::from_env(), DeployMode::Shared);
        unsafe {
            std::env::set_var("OPEN4X_LOBBY_PER_GAME", "1");
        }
        assert_eq!(DeployMode::from_env(), DeployMode::PerGame);
        unsafe {
            std::env::remove_var("OPEN4X_LOBBY_PER_GAME");
        }
    }
}
