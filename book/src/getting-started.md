# Quick Start

## Prerequisites

- **Rust** (edition 2024, stable toolchain 1.86+)
- **jj** (Jujutsu) for version control (the project uses jj, not git)
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

The CLI binary is the `open4x` package (in the `civsim/` directory):

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

```bash
# Install trunk if needed
cargo install trunk

# Build and serve the frontend (hot-reloading)
cd open4x-server
trunk serve
```

The frontend connects to the server's WebSocket endpoint for multiplayer games.

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
+-- civsim/              # CLI binary
+-- open4x-server/       # Merged server + frontend (ssr/csr features)
+-- book/                # This documentation (mdBook)
+-- ARCHITECTURE.md      # Detailed system architecture spec
```
