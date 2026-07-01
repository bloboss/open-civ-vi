//! Phase 3 of the CLI server-mode plan: a black-box integration test
//! that boots a real `open4x-server` child process on an ephemeral
//! port, drives a fixed action sequence through `open4x --server
//! ...`, and asserts each response has the expected wire shape.
//!
//! This locks down end-to-end remote dispatch: bootstrap, every
//! Phase 1 read endpoint we surface, every Phase 2 mutation we
//! support, and the structured-error path on `end-turn` when
//! required pending actions remain.
//!
//! Plan: `book/src/roadmap/cli-server-mode.md`.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

// ── server child guard ──────────────────────────────────────────────────────

/// RAII wrapper: spawns the server in `new()`, kills it on `drop()`.
/// Keeps the child's stdout/stderr piped to `/dev/null` so test output
/// stays readable.
struct ServerHandle {
    child: Child,
    port: u16,
}

impl ServerHandle {
    fn new() -> Self {
        let server_bin = locate_server_binary().expect(
            "cannot locate open4x-server binary — \
             run `cargo build -p open4x-server` first",
        );

        // Bind+drop trick to grab a free high port. Race window is
        // tiny and acceptable for a serial-ish test rig.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        // Empty dir for OPEN4X_STATIC_DIR so the ServeDir fallback
        // doesn't try to read the project's `dist/`.
        let empty_dir = std::env::temp_dir().join(format!("open4x_empty_{port}"));
        std::fs::create_dir_all(&empty_dir).expect("mkdir empty static dir");

        let child = Command::new(&server_bin)
            .env("PORT", port.to_string())
            .env("OPEN4X_STATIC_DIR", &empty_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn open4x-server");

        let handle = Self { child, port };
        handle.wait_ready();
        handle
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Block until `GET /health` returns 200, or panic after 15s.
    fn wait_ready(&self) {
        let url = format!("{}/health", self.base_url());
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if let Ok(resp) = reqwest::blocking::get(&url)
                && resp.status().is_success()
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("open4x-server on port {} did not become healthy", self.port);
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ── path helpers ────────────────────────────────────────────────────────────

/// Find the open4x-server binary alongside `open4x` in the same target
/// dir. Returns `None` if it isn't there (typically because the test
/// rig forgot to build it).
fn locate_server_binary() -> Option<PathBuf> {
    let cli_bin = PathBuf::from(env!("CARGO_BIN_EXE_open4x"));
    let target_dir = cli_bin.parent()?;
    let candidate = target_dir.join(if cfg!(windows) {
        "open4x-server.exe"
    } else {
        "open4x-server"
    });
    if candidate.exists() {
        return Some(candidate);
    }

    // Fall back to building it on demand. First-run cost only — cargo
    // caches afterwards.
    let workspace_root = find_workspace_root(&cli_bin)?;
    let status = Command::new("cargo")
        .args(["build", "-p", "open4x-server", "--bin", "open4x-server"])
        .current_dir(&workspace_root)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    candidate.exists().then_some(candidate)
}

fn find_workspace_root(from: &Path) -> Option<PathBuf> {
    for ancestor in from.ancestors() {
        let toml = ancestor.join("Cargo.toml");
        if toml.exists()
            && let Ok(s) = std::fs::read_to_string(&toml)
            && s.contains("[workspace]")
        {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

// ── CLI helpers ─────────────────────────────────────────────────────────────

fn cli_path() -> &'static str {
    env!("CARGO_BIN_EXE_open4x")
}

/// Run the CLI in `--server` mode and return `(success, stdout_json)`.
/// Panics on a body that doesn't parse as JSON so test failures point
/// at the offending command directly.
fn run_remote(server: &str, token_file: &Path, args: &[&str]) -> (bool, Value) {
    let mut cmd = Command::new(cli_path());
    cmd.args(["--server", server, "--token-file"]);
    cmd.arg(token_file);
    cmd.args(args);
    let output = cmd.output().expect("run open4x");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if stdout.is_empty() {
        return (
            output.status.success(),
            Value::String(format!("(empty stdout, stderr={stderr})")),
        );
    }
    let json = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("CLI {args:?} produced non-JSON stdout (err={e}): {stdout}\nstderr={stderr}")
    });
    (output.status.success(), json)
}

fn temp_session_file(label: &str) -> PathBuf {
    let pid = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("open4x_remote_parity_{label}_{pid}_{ts}.json"))
}

// ── the test ────────────────────────────────────────────────────────────────

#[test]
fn remote_parity_full_loop() {
    let server = ServerHandle::new();
    let url = server.base_url();
    let session_path = temp_session_file("loop");
    // Be defensive — earlier failed runs may have left stale files.
    let _ = std::fs::remove_file(&session_path);

    // 1. Bootstrap
    let (ok, body) = run_remote(
        &url,
        &session_path,
        &[
            "new-game", "--width", "20", "--height", "12", "--seed", "7", "--player", "Rome",
            "--ai", "Babylon",
        ],
    );
    assert!(ok, "new-game failed: {body}");
    assert_eq!(body["success"], Value::Bool(true));
    assert_eq!(body["mode"], Value::String("remote".into()));
    assert!(body["game_id"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(body["civ_id"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(body["turn"], Value::from(0u32));
    assert!(session_path.exists(), "session file should be written");

    // 2. Read endpoints
    let (ok, yields) = run_remote(&url, &session_path, &["status", "yields"]);
    assert!(ok && yields["resources"]["science"]["per_turn"].is_number());

    let (ok, pending) = run_remote(&url, &session_path, &["status", "pending"]);
    assert!(
        ok && pending["items"].is_array(),
        "pending items array missing: {pending}"
    );
    // Before any research/civic is queued, both choose_* items are
    // required — that's the gate end-turn enforces below.
    let items = pending["items"].as_array().unwrap();
    let kinds: Vec<&str> = items
        .iter()
        .filter(|i| i["required"] == Value::Bool(true))
        .filter_map(|i| i["id"].as_str())
        .collect();
    assert!(
        kinds.contains(&"choose_research") && kinds.contains(&"choose_civic"),
        "expected choose_research + choose_civic required: {kinds:?}",
    );

    let (ok, units) = run_remote(&url, &session_path, &["list", "units"]);
    assert!(ok && units["units"].is_array(), "list units shape: {units}");
    let warrior_id = units["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["is_own"] == Value::Bool(true) && u["kind"] == Value::String("Warrior".into()))
        .and_then(|u| u["id"].as_str())
        .expect("own warrior in list units")
        .to_string();

    let (ok, cities) = run_remote(&url, &session_path, &["list", "cities"]);
    assert!(ok && cities["cities"].is_array() && !cities["cities"].as_array().unwrap().is_empty());

    // 3. Structured-error path: end-turn must reject while choose_*
    // items are still required.
    let (ok, body) = run_remote(&url, &session_path, &["end-turn"]);
    assert!(
        !ok,
        "end-turn should fail before required actions are cleared: {body}"
    );

    // 4. Mutations
    let (ok, _) = run_remote(
        &url,
        &session_path,
        &["action", "research", "--tech", "Pottery"],
    );
    assert!(ok, "queue research");
    let (ok, _) = run_remote(
        &url,
        &session_path,
        &["action", "study-civic", "--civic", "Code of Laws"],
    );
    assert!(ok, "queue civic");

    let (ok, mv) = run_remote(
        &url,
        &session_path,
        &[
            "action",
            "move",
            "--unit",
            &warrior_id,
            "--to-q",
            "0",
            "--to-r",
            "0",
        ],
    );
    // Move may legitimately fail (occupied tile, out of range) — we
    // only assert the wire shape parses, not the rule outcome.
    assert!(
        mv["ok"].is_boolean() || mv.get("error").is_some(),
        "move shape: {mv}"
    );
    // Regardless of outcome, the CLI exited cleanly (non-rule errors
    // would have produced non-JSON stderr already).
    let _ = ok;

    let (ok, end) = run_remote(&url, &session_path, &["end-turn"]);
    assert!(ok, "end-turn after queueing research+civic: {end}");
    assert_eq!(end["ok"], Value::Bool(true));
    assert_eq!(end["turn_status"]["turn"], Value::from(1u32));

    // 5. Read again to confirm the session token still resolves after
    // the turn flip.
    let (ok, yields_t1) = run_remote(&url, &session_path, &["status", "yields"]);
    assert!(ok);
    assert_eq!(yields_t1["turn"], Value::from(1u32));

    // Cleanup
    let _ = std::fs::remove_file(&session_path);
}

// ── unsupported-action sanity check ────────────────────────────────────────

#[test]
fn remote_action_unsupported_errors_cleanly() {
    let server = ServerHandle::new();
    let url = server.base_url();
    let session_path = temp_session_file("unsupp");
    let _ = std::fs::remove_file(&session_path);

    let (ok, _) = run_remote(
        &url,
        &session_path,
        &[
            "new-game", "--width", "15", "--height", "10", "--seed", "11", "--player", "Rome",
        ],
    );
    assert!(ok);

    // `assign-policy` has no REST mutation today — the remote
    // dispatcher should refuse with a clean stderr message and exit
    // non-zero, not panic or produce garbage on stdout.
    let output = Command::new(cli_path())
        .args([
            "--server",
            &url,
            "--token-file",
            session_path.to_str().unwrap(),
            "action",
            "assign-policy",
            "--policy",
            "Discipline",
        ])
        .output()
        .expect("run open4x");
    assert!(
        !output.status.success(),
        "unsupported action should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("server mode does not yet support"),
        "stderr should explain the gap: {stderr}",
    );

    let _ = std::fs::remove_file(&session_path);
}

// ── baseline-transcript test ───────────────────────────────────────────────

/// Walks a fixed happy-path sequence and captures each step's JSON
/// output (normalized to strip per-run variability: ULIDs, bearer
/// tokens, ephemeral ports, temp session paths) into a transcript
/// checked into the repo. The diff catches projector regressions —
/// if a field rename slips through the shape-only asserts above,
/// the transcript drift surfaces it.
///
/// To regenerate after an intentional projector change:
///
/// ```sh
/// OPEN4X_UPDATE_BASELINE=1 cargo test -p open4x-cli \
///     --test remote_parity remote_parity_baseline
/// ```
#[test]
fn remote_parity_baseline() {
    let server = ServerHandle::new();
    let url = server.base_url();
    let session_path = temp_session_file("baseline");
    let _ = std::fs::remove_file(&session_path);

    // Fixed sequence. Avoid actions whose outcome depends on
    // randomised starting positions (e.g. `move` to a specific
    // tile) — the baseline must be fully deterministic for the
    // given seed.
    let steps: &[(&str, &[&str])] = &[
        (
            "new-game",
            &[
                "new-game", "--width", "20", "--height", "12", "--seed", "7", "--player", "Rome",
                "--ai", "Babylon",
            ],
        ),
        ("status yields", &["status", "yields"]),
        ("status pending", &["status", "pending"]),
        ("list cities", &["list", "cities"]),
        (
            "action research Pottery",
            &["action", "research", "--tech", "Pottery"],
        ),
        (
            "action study-civic Code of Laws",
            &["action", "study-civic", "--civic", "Code of Laws"],
        ),
        ("end-turn", &["end-turn"]),
        ("status yields (turn 1)", &["status", "yields"]),
    ];

    let mut transcript = String::new();
    for (label, args) in steps {
        let (ok, mut body) = run_remote(&url, &session_path, args);
        assert!(ok, "baseline step {label:?} failed: {body}");
        normalize(&mut body);
        transcript.push_str(&format!("=== {label} ===\n"));
        transcript.push_str(&serde_json::to_string_pretty(&body).unwrap());
        transcript.push_str("\n\n");
    }

    let baseline_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/remote_parity_baseline.txt");

    if std::env::var("OPEN4X_UPDATE_BASELINE").is_ok() {
        std::fs::create_dir_all(baseline_path.parent().unwrap()).unwrap();
        std::fs::write(&baseline_path, &transcript).expect("write baseline");
        eprintln!("Updated baseline: {}", baseline_path.display());
    } else {
        let expected = std::fs::read_to_string(&baseline_path).unwrap_or_else(|_| {
            panic!(
                "no baseline at {} — run `OPEN4X_UPDATE_BASELINE=1 cargo test \
                 -p open4x-cli --test remote_parity remote_parity_baseline` to create one",
                baseline_path.display(),
            );
        });
        if expected != transcript {
            let diff = first_diff(&expected, &transcript);
            panic!(
                "transcript drift vs. baseline. If intentional (projector change), \
                 regenerate with OPEN4X_UPDATE_BASELINE=1.\n\n{diff}"
            );
        }
    }

    let _ = std::fs::remove_file(&session_path);
}

/// Walk a `Value` and replace per-run-variable fields. Keeps wire
/// shape and stable content; drops random tokens / IDs / ports /
/// paths. Also sorts arrays of objects by their `id` field (after
/// the ID itself has been masked to `<ULID>`, we sort by the next
/// most-stable key — `name` — when present). This is necessary
/// because libciv's tech / civic / policy registries are HashMap-
/// backed, so the server projector emits them in hash-randomized
/// order. Sorting on the client side keeps the baseline stable
/// while still catching set-level regressions (drops, additions,
/// content changes).
fn normalize(v: &mut Value) {
    match v {
        Value::String(s) => {
            if is_ulid(s) {
                *s = "<ULID>".into();
            }
        }
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                let val = map.get_mut(&k).unwrap();
                match k.as_str() {
                    "token" => *val = Value::String("<TOKEN>".into()),
                    "session_file" => *val = Value::String("<SESSION_PATH>".into()),
                    "server" => *val = Value::String("<SERVER_URL>".into()),
                    _ => normalize(val),
                }
            }
        }
        Value::Array(a) => {
            for elem in a.iter_mut() {
                normalize(elem);
            }
            // After children are normalized, sort if every element is
            // an object — by `name` first, then by the rendered
            // string of the entry as a tiebreaker.
            if !a.is_empty() && a.iter().all(Value::is_object) {
                a.sort_by(|x, y| sort_key(x).cmp(&sort_key(y)));
            }
        }
        _ => {}
    }
}

fn sort_key(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("");
    if !name.is_empty() {
        return name.to_string();
    }
    // No name — fall back to the serialized representation. This is
    // stable across runs because every child has already been
    // normalized (IDs are <ULID>, tokens are <TOKEN>, etc).
    v.to_string()
}

fn is_ulid(s: &str) -> bool {
    s.len() == 26
        && s.chars().all(|c| {
            matches!(
                c,
                '0'..='9'
                | 'A'..='H'
                | 'J' | 'K' | 'M' | 'N'
                | 'P'..='T'
                | 'V'..='Z'
            )
        })
}

/// Short diff hint: byte position + ~200 bytes of context on both
/// sides. Cheap regression signal without pulling in `pretty_assertions`.
fn first_diff(expected: &str, actual: &str) -> String {
    let mismatch = expected
        .as_bytes()
        .iter()
        .zip(actual.as_bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(expected.len().min(actual.len()));
    let start = mismatch.saturating_sub(80);
    let exp_end = (mismatch + 200).min(expected.len());
    let act_end = (mismatch + 200).min(actual.len());
    format!(
        "first diff at byte {mismatch}\n\
         expected: ...{}\n\
         actual:   ...{}",
        &expected[start..exp_end],
        &actual[start..act_end],
    )
}
