//! `cargo xtask <recipe>` — workspace recipe runner.
//!
//! Pure Rust, no extra installs. Every recipe is a thin orchestrator around
//! `cargo`, `trunk`, or `mdbook`; nothing here knows about the game itself.
//! Add a recipe by extending the [`Recipe`] enum and matching on it in
//! [`run`].

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "cargo xtask",
    about = "Workspace recipe runner. See `cargo xtask <recipe> --help`.",
    bin_name = "cargo xtask"
)]
struct Cli {
    #[command(subcommand)]
    recipe: Recipe,
}

#[derive(Subcommand, Debug)]
enum Recipe {
    /// Regenerate `book/src/multiplayer/openapi.json` from the Rust source.
    GenOpenapi {
        /// Write to this path instead of the default
        /// `book/src/multiplayer/openapi.json`.
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },

    /// Fail if the committed `openapi.json` differs from a freshly-generated
    /// one. Intended for CI; locally, just run `cargo xtask gen-openapi`.
    OpenapiCheck,

    /// Regenerate the TypeScript wire-type bindings from `open4x-protocol`
    /// (via ts-rs) into `open4x-client-js/src/gen/protocol/`, plus a
    /// `protocol.ts` barrel.
    GenTs {
        /// Write into this directory instead of the default
        /// `open4x-client-js/src/gen/protocol`.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
    },

    /// Fail if the committed TS bundle differs from a freshly-generated one.
    /// Intended for CI; locally, just run `cargo xtask gen-ts`.
    TsCheck,

    /// Run every check we want to keep green: build + test + clippy, both
    /// without and with the `openapi` feature, plus a wasm32 client build.
    Check {
        /// Skip the wasm32 build (saves a target install on a fresh machine).
        #[arg(long)]
        no_wasm: bool,
    },

    /// `cargo build --workspace` (sanity build, default features).
    Build,

    /// `cargo test --workspace` plus the server's openapi-feature tests.
    Test,

    /// `cargo clippy --workspace -- -D warnings` plus the openapi-feature
    /// clippy pass.
    Lint,

    /// `cargo build -p open4x-client-web --target wasm32-unknown-unknown`.
    /// Requires the wasm32 target installed (`rustup target add ...`).
    Wasm {
        /// Build in release mode.
        #[arg(long)]
        release: bool,
    },

    /// Build the mdbook under `book/`. Requires `mdbook` on `$PATH`.
    Book {
        /// Run `mdbook serve` with live reload instead of a one-shot build.
        #[arg(long)]
        serve: bool,
    },

    /// Run the native server (`open4x-server`). Pass `--release` to use the
    /// release profile.
    Server {
        #[arg(long)]
        release: bool,
        /// Extra arguments forwarded to the server binary.
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Run the trunk dev server for the wasm client. Requires `trunk` on
    /// `$PATH`. Conventionally proxies REST/WS to `localhost:3001`.
    TrunkServe,

    /// Run an AI-vs-AI demo via the CLI binary.
    Demo {
        /// Number of turns to simulate.
        #[arg(long, default_value_t = 50)]
        turns: u32,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.recipe) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(recipe: Recipe) -> Result<(), String> {
    match recipe {
        Recipe::GenOpenapi { out } => gen_openapi(out.as_deref()),
        Recipe::OpenapiCheck => openapi_check(),
        Recipe::GenTs { out } => gen_ts(out.as_deref()),
        Recipe::TsCheck => ts_check(),
        Recipe::Check { no_wasm } => check_all(no_wasm),
        Recipe::Build => cargo(&["build", "--workspace"]),
        Recipe::Test => {
            cargo(&["test", "--workspace"])?;
            cargo(&["test", "-p", "open4x-server", "--features", "openapi"])
        }
        Recipe::Lint => {
            cargo(&["clippy", "--workspace", "--", "-D", "warnings"])?;
            cargo(&[
                "clippy",
                "-p",
                "open4x-server",
                "--features",
                "openapi",
                "--",
                "-D",
                "warnings",
            ])
        }
        Recipe::Wasm { release } => {
            let mut args: Vec<&str> = vec![
                "build",
                "-p",
                "open4x-client-web",
                "--target",
                "wasm32-unknown-unknown",
            ];
            if release {
                args.push("--release");
            }
            cargo(&args)
        }
        Recipe::Book { serve } => {
            let sub = if serve { "serve" } else { "build" };
            tool(
                "mdbook",
                &[sub],
                Some(workspace_root().join("book").as_path()),
            )
        }
        Recipe::Server { release, args } => {
            let mut cargo_args: Vec<String> =
                vec!["run".into(), "-p".into(), "open4x-server".into()];
            if release {
                cargo_args.push("--release".into());
            }
            if !args.is_empty() {
                cargo_args.push("--".into());
                cargo_args.extend(args);
            }
            let refs: Vec<&str> = cargo_args.iter().map(String::as_str).collect();
            cargo(&refs)
        }
        Recipe::TrunkServe => tool(
            "trunk",
            &["serve"],
            Some(workspace_root().join("open4x-client-web").as_path()),
        ),
        Recipe::Demo { turns } => {
            let turns = turns.to_string();
            cargo(&["run", "-p", "open4x", "--", "ai-demo", "--turns", &turns])
        }
    }
}

// ── recipes ──────────────────────────────────────────────────────────────────

fn gen_openapi(out: Option<&Path>) -> Result<(), String> {
    let mut args: Vec<String> = vec![
        "run".into(),
        "-p".into(),
        "open4x-server".into(),
        "--features".into(),
        "openapi".into(),
        "--bin".into(),
        "gen-openapi".into(),
    ];
    if let Some(path) = out {
        args.push("--".into());
        args.push(path.display().to_string());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    cargo(&refs)
}

/// CI-style check: regenerate the spec to a tempfile and compare it with the
/// committed copy. Exits non-zero if they differ.
fn openapi_check() -> Result<(), String> {
    let committed = workspace_root().join("book/src/multiplayer/openapi.json");
    let tmp = std::env::temp_dir().join(format!("openapi-{}.json", std::process::id()));

    gen_openapi(Some(&tmp))?;

    let a = std::fs::read(&committed).map_err(|e| {
        format!(
            "couldn't read committed openapi.json at {}: {e}",
            committed.display()
        )
    })?;
    let b = std::fs::read(&tmp).map_err(|e| {
        format!(
            "couldn't read freshly-generated openapi.json at {}: {e}",
            tmp.display()
        )
    })?;
    let _ = std::fs::remove_file(&tmp);

    if a == b {
        println!("openapi.json matches generated output ({} bytes).", a.len());
        Ok(())
    } else {
        Err(format!(
            "openapi.json is stale.\n\
             Run `cargo xtask gen-openapi` and commit the result.\n\
             Committed: {} bytes\n\
             Generated: {} bytes",
            a.len(),
            b.len(),
        ))
    }
}

/// Regenerate the TypeScript wire-type bindings from `open4x-protocol`.
/// ts-rs writes one `.ts` per type when the `--features ts` export tests
/// run with `TS_RS_EXPORT_DIR` pointing at the target dir; we then emit a
/// `protocol.ts` barrel re-exporting them.
fn gen_ts(out: Option<&Path>) -> Result<(), String> {
    let dir = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workspace_root().join("open4x-client-js/src/gen/protocol"));
    // Clean stale output so types removed from the protocol don't linger.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

    println!(
        "$ TS_RS_EXPORT_DIR={} {} test -p open4x-protocol --features ts",
        dir.display(),
        cargo_bin(),
    );
    let output = Command::new(cargo_bin())
        .args(["test", "-p", "open4x-protocol", "--features", "ts"])
        .env("TS_RS_EXPORT_DIR", &dir)
        .output()
        .map_err(|e| format!("failed to spawn cargo: {e}"))?;
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err("ts-rs export tests failed".into());
    }

    // Completeness guard: ts-rs runs one `export_bindings_*` test per exported
    // type. If two types share a name they write the same file (last wins), so
    // the file count drops below the test count — a silent type-name collision.
    let expected = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| l.contains("export_bindings_") && l.contains("... ok"))
        .count();

    write_ts_barrel(&dir)?;

    let written = count_ts_files(&dir)?;
    if expected != written {
        return Err(format!(
            "TS type-name collision: {expected} ts-rs export tests but {written} files \
             written — {} type(s) silently overwritten. Disambiguate the duplicate \
             type name(s) with #[ts(rename = \"...\")].",
            expected.saturating_sub(written),
        ));
    }
    Ok(())
}

/// Count generated `.ts` files in `dir`, excluding the `protocol.ts` barrel.
fn count_ts_files(dir: &Path) -> Result<usize, String> {
    Ok(std::fs::read_dir(dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.ends_with(".ts") && n != "protocol.ts"
        })
        .count())
}

/// Emit a `protocol.ts` barrel re-exporting every generated module so a
/// client can `import type { GameView } from "../gen/protocol/protocol"`.
fn write_ts_barrel(dir: &Path) -> Result<(), String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.strip_suffix(".ts")
                .filter(|s| *s != "protocol")
                .map(str::to_string)
        })
        .collect();
    names.sort();
    let mut out = String::from(
        "// Generated by `cargo xtask gen-ts`. Do not edit.\n\
         // Barrel re-export of every open4x-protocol wire type.\n",
    );
    for n in &names {
        out.push_str(&format!("export type {{ {n} }} from \"./{n}\";\n"));
    }
    let barrel = dir.join("protocol.ts");
    std::fs::write(&barrel, out).map_err(|e| format!("write {}: {e}", barrel.display()))?;
    println!("wrote {} ({} types)", barrel.display(), names.len());
    Ok(())
}

/// CI-style drift check: regenerate the TS bundle into a temp dir and compare
/// it against the committed `open4x-client-js/src/gen/protocol/`. Exits
/// non-zero on any added/removed/changed file.
fn ts_check() -> Result<(), String> {
    let committed = workspace_root().join("open4x-client-js/src/gen/protocol");
    if !committed.is_dir() {
        return Err(format!(
            "no committed TS bundle at {} — run `cargo xtask gen-ts` and commit it first.",
            committed.display()
        ));
    }
    let tmp = std::env::temp_dir().join(format!("open4x-ts-check-{}", std::process::id()));
    let result = gen_ts(Some(&tmp)).and_then(|()| diff_ts_dirs(&committed, &tmp));
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// Compare two directories of `.ts` files for an identical filename set and
/// identical bytes per file.
fn diff_ts_dirs(committed: &Path, fresh: &Path) -> Result<(), String> {
    use std::collections::BTreeMap;
    let list = |d: &Path| -> Result<BTreeMap<String, Vec<u8>>, String> {
        let mut map = BTreeMap::new();
        for entry in std::fs::read_dir(d).map_err(|e| format!("read {}: {e}", d.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".ts") {
                map.insert(
                    name,
                    std::fs::read(entry.path()).map_err(|e| e.to_string())?,
                );
            }
        }
        Ok(map)
    };
    let a = list(committed)?;
    let b = list(fresh)?;

    let mut problems: Vec<String> = Vec::new();
    for name in a.keys() {
        if !b.contains_key(name) {
            problems.push(format!("  removed: {name}"));
        }
    }
    for (name, fresh_bytes) in &b {
        match a.get(name) {
            None => problems.push(format!("  added:   {name}")),
            Some(committed_bytes) if committed_bytes != fresh_bytes => {
                problems.push(format!("  changed: {name}"))
            }
            _ => {}
        }
    }

    if problems.is_empty() {
        println!("TS bindings match generated output ({} files).", a.len());
        Ok(())
    } else {
        problems.sort();
        Err(format!(
            "TS bindings are stale.\n\
             Run `cargo xtask gen-ts` and commit the result.\n{}",
            problems.join("\n"),
        ))
    }
}

fn check_all(no_wasm: bool) -> Result<(), String> {
    // Build/test/lint without the feature, then with it.
    cargo(&["build", "--workspace"])?;
    cargo(&["test", "--workspace"])?;
    cargo(&["clippy", "--workspace", "--", "-D", "warnings"])?;

    cargo(&["test", "-p", "open4x-server", "--features", "openapi"])?;
    cargo(&[
        "clippy",
        "-p",
        "open4x-server",
        "--features",
        "openapi",
        "--",
        "-D",
        "warnings",
    ])?;

    openapi_check()?;
    ts_check()?;

    if !no_wasm {
        cargo(&[
            "build",
            "-p",
            "open4x-client-web",
            "--target",
            "wasm32-unknown-unknown",
        ])?;
    }
    Ok(())
}

// ── command helpers ──────────────────────────────────────────────────────────

fn cargo(args: &[&str]) -> Result<(), String> {
    tool(cargo_bin(), args, None)
}

fn cargo_bin() -> String {
    // Honour the cargo that invoked us — matters for rustup overrides and
    // for `cargo xtask` chained from another cargo invocation.
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

fn tool<S: AsRef<OsStr>>(bin: S, args: &[&str], cwd: Option<&Path>) -> Result<(), String> {
    let bin = bin.as_ref();
    let pretty_args = args.join(" ");
    let pretty_bin = bin.to_string_lossy();
    if let Some(d) = cwd {
        println!("$ (cd {}) {pretty_bin} {pretty_args}", d.display());
    } else {
        println!("$ {pretty_bin} {pretty_args}");
    }

    let mut cmd = Command::new(bin);
    cmd.args(args);
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("failed to spawn {pretty_bin}: {e}"))?;
    if !status.success() {
        return Err(format!(
            "{pretty_bin} {pretty_args} exited with {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into()),
        ));
    }
    Ok(())
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at xtask/; the workspace root is one up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask manifest dir has a parent")
        .to_path_buf()
}
