# Quick Start

## Prerequisites

- **Rust** (edition 2024, stable toolchain 1.86+)
- **`but`** (GitButler) for version control — see [`agents/skills/gitbutler/SKILL.md`](https://github.com/bloboss/open-civ-vi/blob/main/agents/skills/gitbutler/SKILL.md). `jj` use is suspended during the GitButler-workflow window.
- For WASM frontend: **trunk** (`cargo install trunk`) and the `wasm32-unknown-unknown` target

## Building

```bash
# Build all crates
cargo build --workspace

# Run all tests (200+ integration tests)
cargo test --workspace

# Lint (must pass with zero warnings)
cargo clippy --workspace -- -D warnings
```

## Running the CLI

The CLI binary is the `open4x` package (in the `open4x-cli/` directory):

```bash
# Create a new game and print board dimensions
cargo run -p open4x -- new

# Simulate 100 turns of AI-vs-AI gameplay
cargo run -p open4x -- run --turns 100

# Interactive play mode (stdin-driven warrior movement)
cargo run -p open4x -- play

# AI demo with periodic board visualization
cargo run -p open4x -- ai-demo --turns 50 --board-every 10
```

### CLI Commands

| Command | Description |
|---------|-------------|
| `new` | Generate a fresh map and report board size |
| `run` | Headless simulation for N turns |
| `demo` | Single-turn demo: create game, move unit, advance turn |
| `ai-demo` | Two `HeuristicAgent` AIs play against each other |
| `play` | Interactive mode with keyboard-driven unit movement |

All commands accept `--seed`, `--width`, and `--height` flags for deterministic, reproducible games.

## Running the Multiplayer Server

```bash
# Build and run the server (listens on port 3001)
cargo run -p open4x-server

# Or use Docker
docker compose up --build
```

The server serves the WASM frontend as static files and exposes a WebSocket endpoint at `/ws`.

## Building the WASM Frontend

The frontend is the `open4x-client-web` crate — a Leptos CSR cdylib that
builds for `wasm32-unknown-unknown`. The native server binary
(`open4x-server`) is a separate crate and has no Leptos / wasm-bindgen
dependency.

```bash
# One-time setup
rustup target add wasm32-unknown-unknown
cargo install trunk          # or use a prebuilt release (cargo install fails on some platforms)

# Iterate on the UI with hot-reloading (proxies API calls to localhost:3001)
cd open4x-client-web
trunk serve

# Production build — emits open4x-client-web/dist/ for the server to serve
trunk build --release
```

Then start the native server (which serves the trunk-built artefacts from
`OPEN4X_STATIC_DIR`, defaulting to `./open4x-client-web/dist`):

```bash
cargo run -p open4x-server     # serves /api/*, /ws, and dist/
```

Use an absolute path for `OPEN4X_STATIC_DIR` when running from a different
working directory; the relative default resolves against the binary's CWD.

### REST surface

The HTTP API lives under `/api/v1/*`. The canonical, machine-readable
description is generated from the Rust source via
[`utoipa`](https://docs.rs/utoipa) and shipped at
[`book/src/multiplayer/openapi.json`](./multiplayer/openapi.json). The
human-readable companion is the
[REST API reference](./multiplayer/rest-api.md); the full phased plan and
changelog live in [`web-ui.md`](./roadmap/web-ui.md).

```bash
# Regenerate the OpenAPI document. Runs from anywhere in the workspace;
# default output path is book/src/multiplayer/openapi.json.
cargo run -p open4x-server --features openapi --bin gen-openapi
```

CI keeps the document in sync via the `openapi_paths_match_router` test in
`open4x-server/tests/openapi_contract.rs`. If the live Axum router and the
`#[utoipa::path]` annotations in `open4x-server/src/server/openapi.rs`
ever disagree, the test fails.

The frontend connects to the server's WebSocket endpoint (`/ws`) for the AI
demo and future multiplayer; the single-player loop uses REST instead.

## Running Individual Test Suites

```bash
# Run tests in a specific crate
cargo test -p libciv
cargo test -p libhexgrid

# Run a specific integration test file
cargo test --test gameplay
cargo test --test mapgen
cargo test --test ai_agent

# Run a single test by name
cargo test --workspace test_hills_defender_takes_less_damage
```

## Project Structure

```
open-civ-vi/
+-- libhexgrid/          # Pure hex geometry library
+-- libciv/              # Core game engine
|   +-- src/
|   |   +-- ai/          # AI agents
|   |   +-- civ/         # Civilizations, cities, units, diplomacy
|   |   +-- game/        # GameState, RulesEngine, TurnEngine
|   |   +-- rules/       # Modifiers, tech trees, policies
|   |   +-- world/       # Terrain, features, improvements, mapgen
|   +-- tests/           # Integration tests (20+ test files)
+-- open4x-cli/          # CLI binary (remote mode wraps open4x-sdk)
+-- open4x-protocol/     # Versioned wire types (v1::*)
+-- open4x-sdk/          # Typed HTTP client (native + wasm transports)
+-- open4x-server/       # Native Axum server (REST + WS, no Leptos)
+-- open4x-client-web/   # Leptos CSR frontend (wasm32 cdylib)
+-- book/                # This documentation (mdBook)
+-- CLAUDE.md            # Claude Code configuration
```
