//! Emit the OpenAPI 3 spec for the `/api/v1/*` surface.
//!
//! Usage:
//!     cargo run -p open4x-server --features openapi --bin gen-openapi
//!     cargo run -p open4x-server --features openapi --bin gen-openapi -- <path>
//!
//! Default output path is `book/src/multiplayer/openapi.json` relative to the
//! workspace root (auto-detected by walking up until a `Cargo.toml` with
//! `[workspace]` is found). The generated file is checked in; CI regenerates
//! and diffs to catch drift.

use std::path::{Path, PathBuf};

fn main() {
    let arg_path = std::env::args().nth(1);
    let out_path = arg_path
        .map(PathBuf::from)
        .unwrap_or_else(default_out_path);

    let doc = open4x_server::server::openapi::document();
    let json = doc
        .to_pretty_json()
        .expect("OpenAPI document must serialize");

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("create output dir");
    }
    // Trailing newline so the file diffs cleanly in git.
    let mut bytes = json.into_bytes();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    std::fs::write(&out_path, &bytes).expect("write openapi.json");

    println!("Wrote {} bytes to {}", bytes.len(), out_path.display());
}

fn default_out_path() -> PathBuf {
    workspace_root()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("book/src/multiplayer/openapi.json")
}

fn workspace_root() -> Option<PathBuf> {
    let mut dir: PathBuf = std::env::current_dir().ok()?;
    loop {
        if has_workspace_marker(&dir) {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn has_workspace_marker(dir: &Path) -> bool {
    let cargo = dir.join("Cargo.toml");
    if !cargo.exists() {
        return false;
    }
    std::fs::read_to_string(&cargo)
        .ok()
        .is_some_and(|s| s.contains("[workspace]"))
}
