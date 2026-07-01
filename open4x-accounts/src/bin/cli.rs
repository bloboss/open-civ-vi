//! `open4x-accounts` — CLI for ops tasks against the lobby's
//! accounts database.
//!
//! Subcommands:
//! - `dump-audit` — print the most recent audit_events rows.
//! - `prune-sessions` — drop revoked + expired sessions older than
//!   N days (default 30) so the table doesn't grow unbounded.
//! - `delete-account` — GDPR cascade for a single player.
//! - `db-dump` — VACUUM INTO an out-file. Online-safe.
//! - `db-restore` — validate + copy a snapshot back over the
//!   live db file (after saving the live one as `.bak.<ts>`).

use std::path::PathBuf;

use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use open4x_accounts::audit::{AuditEventKind, AuditStore, NewAuditEvent, SqliteAuditStore};
use open4x_accounts::store::{AccountStore, SqliteAccountStore};
use open4x_accounts::PlayerId;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[derive(Parser)]
#[command(
    name = "open4x-accounts",
    version,
    about = "Ops CLI for the open4x-lobby accounts database"
)]
struct Cli {
    /// Path to the sqlite db. Defaults to ./data/lobby/accounts.sqlite
    /// (matches AppState::boot's default location).
    #[arg(long, default_value = "./data/lobby/accounts.sqlite")]
    db: PathBuf,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the most recent audit_events rows, newest first.
    DumpAudit {
        /// How many rows to print.
        #[arg(short, long, default_value_t = 100)]
        limit: u32,
    },

    /// Delete sessions that are revoked OR expired more than `--days`
    /// ago. Prints the number of rows removed.
    PruneSessions {
        /// Threshold in days; rows whose `expires_at` (or
        /// `revoked_at`) is older than this are eligible. Default 30.
        #[arg(short, long, default_value_t = 30)]
        days: i64,
    },

    /// Hard-delete an account. Cascades sessions + identities via the
    /// schema FK. Use for GDPR / right-to-be-forgotten requests.
    DeleteAccount {
        /// Hex player ID. Accepts the canonical `0xAAAA·BBBB·CCCC·DDDD`
        /// dot-grouped form *or* a 16-char raw hex string.
        #[arg(long)]
        player_id: String,
    },

    /// Online consistent snapshot via SQLite's `VACUUM INTO`.
    /// Refuses to overwrite an existing file.
    DbDump {
        /// Where to write the snapshot. Parent dir is created.
        #[arg(long)]
        out: PathBuf,
    },

    /// Replace the live db with a previously-dumped snapshot.
    /// Validates the source via `PRAGMA integrity_check` first;
    /// renames the live file to `<db>.bak.<unix-ts>` before
    /// copying (skip with `--force`). The lobby should be stopped
    /// before running this — running lobbies hold a connection,
    /// see stale rows until restart, and may corrupt new writes
    /// against the replaced WAL.
    DbRestore {
        /// Source snapshot to restore from.
        #[arg(long)]
        from: PathBuf,
        /// Skip the rename-as-backup step. Live db is overwritten
        /// directly. Caller-of-last-resort.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let opts = SqliteConnectOptions::new().filename(&cli.db);
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(opts)
        .await
        .unwrap_or_else(|e| {
            eprintln!("open: {e}");
            std::process::exit(2);
        });
    // Sqlite needs PRAGMA foreign_keys = ON per connection for
    // ON DELETE CASCADE to actually fire.
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .ok();

    match cli.cmd {
        Command::DumpAudit { limit } => {
            let store = SqliteAuditStore::from_pool(pool);
            let rows = store.list_recent(limit).await.unwrap_or_else(|e| {
                eprintln!("query: {e}");
                std::process::exit(2);
            });
            println!("ts\tkind\tplayer_id\tip\tdetail");
            for r in rows {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    r.ts,
                    r.kind.as_str(),
                    r.player_id.map(|p| p.display()).unwrap_or_else(|| "-".into()),
                    r.ip.unwrap_or_else(|| "-".into()),
                    r.detail.replace(['\t', '\n'], " "),
                );
            }
        }

        Command::PruneSessions { days } => {
            let cutoff = (Utc::now() - Duration::days(days)).to_rfc3339();
            let res = sqlx::query(
                "DELETE FROM sessions \
                 WHERE revoked_at IS NOT NULL OR expires_at < ?1",
            )
            .bind(&cutoff)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| {
                eprintln!("query: {e}");
                std::process::exit(2);
            });
            println!(
                "pruned {} session row(s) (revoked or expires_at < {})",
                res.rows_affected(),
                cutoff,
            );
        }

        Command::DbDump { out } => {
            if out.exists() {
                eprintln!("dump target already exists: {}", out.display());
                std::process::exit(2);
            }
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            let path_str = out.to_string_lossy().into_owned();
            // VACUUM INTO is the right primitive: it's online-safe,
            // takes a consistent snapshot of every page, and
            // refuses to overwrite — we already pre-checked above
            // for a clearer error message.
            if let Err(e) = sqlx::query(&format!("VACUUM INTO '{}'", path_str.replace('\'', "''")))
                .execute(&pool)
                .await
            {
                eprintln!("dump: {e}");
                std::process::exit(1);
            }
            let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
            println!("dumped {} bytes -> {}", bytes, out.display());
        }

        Command::DbRestore { from, force } => {
            if !from.is_file() {
                eprintln!("restore source not a file: {}", from.display());
                std::process::exit(2);
            }
            // 1. Validate the source: open RO + integrity_check.
            //    Catches truncated copies and malformed databases
            //    before we replace anything live.
            let src_opts = SqliteConnectOptions::new()
                .filename(&from)
                .read_only(true);
            let src_pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(src_opts)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("open source: {e}");
                    std::process::exit(2);
                });
            let check: (String,) = sqlx::query_as("PRAGMA integrity_check")
                .fetch_one(&src_pool)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("source integrity check: {e}");
                    std::process::exit(1);
                });
            if check.0 != "ok" {
                eprintln!("source integrity check failed: {}", check.0);
                std::process::exit(1);
            }
            src_pool.close().await;

            // 2. Release our live handle so the file is not held
            //    open during the copy. (Linux lets us copy over an
            //    open file, but releasing first matches operator
            //    intuition.)
            pool.close().await;

            // 3. Optional .bak rotation.
            if cli.db.exists() && !force {
                let ts = Utc::now().timestamp();
                let bak = {
                    let mut p = cli.db.clone();
                    let stem = p
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "accounts.sqlite".into());
                    p.set_file_name(format!("{stem}.bak.{ts}"));
                    p
                };
                if let Err(e) = std::fs::rename(&cli.db, &bak) {
                    eprintln!("backup target: {e}");
                    std::process::exit(2);
                }
                eprintln!("[backup] saved old db -> {}", bak.display());
            }

            // 4. Copy the snapshot in place.
            if let Some(parent) = cli.db.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            if let Err(e) = std::fs::copy(&from, &cli.db) {
                eprintln!("copy: {e}");
                std::process::exit(1);
            }
            let bytes = std::fs::metadata(&cli.db).map(|m| m.len()).unwrap_or(0);
            println!("restored {} -> {} ({} bytes)", from.display(), cli.db.display(), bytes);
        }

        Command::DeleteAccount { player_id } => {
            let player_id = parse_player_id_arg(&player_id).unwrap_or_else(|e| {
                eprintln!("parse player_id: {e}");
                std::process::exit(2);
            });
            let store = SqliteAccountStore::connect(&cli.db)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("open store: {e}");
                    std::process::exit(2);
                });
            // Audit BEFORE delete so the row survives the cascade.
            let audit = SqliteAuditStore::from_pool(pool);
            let _ = audit
                .record(NewAuditEvent {
                    kind: AuditEventKind::AccountDeleted,
                    player_id: Some(player_id),
                    ip: None,
                    detail: "cli".into(),
                })
                .await;
            match store.delete_account(player_id).await {
                Ok(()) => println!("deleted {}", player_id.display()),
                Err(e) => {
                    eprintln!("delete: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Accept either the canonical dot-grouped display form
/// (`0xAAAA·BBBB·CCCC·DDDD`) or a bare 16-char hex string.
fn parse_player_id_arg(s: &str) -> Result<PlayerId, &'static str> {
    // Strip a leading `0x` / `0X` prefix BEFORE filtering, otherwise
    // the leading `0` survives and we end up with 17 hex digits.
    let trimmed = s.trim();
    let body = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let stripped: String = body
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();
    if stripped.len() != 16 {
        return Err("expected 16 hex digits");
    }
    let raw = u64::from_str_radix(&stripped, 16).map_err(|_| "invalid hex")?;
    Ok(PlayerId::new(raw))
}
