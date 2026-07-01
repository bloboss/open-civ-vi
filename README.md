# Open Civ VI

An open-source Rust implementation of a Civilization VI-style 4X strategy game engine. The project provides a complete game simulation -- terrain, cities, units, combat, research, diplomacy, trade, and more -- designed for correctness, extensibility, and eventual use as a reinforcement-learning environment.

## Goals

- **Functional game engine** -- all core Civ VI mechanics implemented in pure Rust with no graphical dependency in the core library
- **RL-ready** -- every state mutation produces a semantic diff (`GameStateDiff`), enabling replay, observation, and reward shaping for AI agents
- **Extensible** -- game content (civilizations, units, terrain, buildings, wonders) is defined by implementing Rust traits; new content links at compile time with no runtime scripting layer
- **Multiplayer** -- an Axum-based WebSocket server and a Leptos/WASM browser client provide real-time multiplayer gameplay

## Crate Overview

| Crate | Role |
|-------|------|
| `libhexgrid` | Pure hex geometry -- coordinates, pathfinding, line of sight |
| `libciv` | All game state and rules -- world, civilizations, rules engine, AI |
| `open4x-protocol` | Versioned wire types (`v1::*`) -- the client/server contract |
| `open4x-sdk` | Typed HTTP client over the protocol (native + wasm transports) |
| `open4x-cli` | CLI binary (`open4x`) for local simulation, interactive play, and remote server mode |
| `open4x-server` | Native Axum server -- REST (`/api/v1/*`) + WebSocket (`/ws`) + game rooms |
| `open4x-client-web` | Leptos/WASM browser client (consumes the SDK) |
| `open4x-accounts` | Identity substrate -- accounts, sessions, magic-link/OIDC auth (sqlite) |
| `open4x-lobby` | Pre-game surface -- landing, login, new-game wizard, profile (Leptos + Axum) |

## Quick Start

```bash
# Build all crates
cargo build --workspace

# Run all tests (200+ integration tests)
cargo test --workspace

# Lint (must pass with zero warnings)
cargo clippy --workspace -- -D warnings

# Interactive play mode
cargo run -p open4x -- play

# AI-vs-AI simulation
cargo run -p open4x -- run --turns 100
```

For the WASM frontend:

```bash
cargo install trunk
cd open4x-client-web && trunk serve
```

## Documentation

Full documentation is available in the [mdBook](./book/):

```bash
mdbook serve book/
```

Topics covered: architecture, all game systems, engine design (IDs, modifiers, diffs, extensibility), multiplayer protocol, and the implementation roadmap.

## Project Status

The engine has **200+ passing integration tests** across 21 test files. Core gameplay is functional end-to-end: map generation, city founding, unit movement, combat, research, production, trade, cultural borders, loyalty, era scoring, tourism, and victory conditions all work.

See the [Implementation Roadmap](./book/src/roadmap/status.md) for detailed status and planned work.

## License & Contributing

Contributions are welcome -- see the roadmap for areas that need work.

### Development setup

Lint/format tooling runs through [pre-commit](https://pre-commit.com);
the Python-side tools are pinned in a [uv](https://docs.astral.sh/uv/)
project (`pyproject.toml` / `uv.lock`). One-time setup:

```bash
uv sync                    # pinned Python tooling (pre-commit, codespell, black)
uv run pre-commit install  # install pre-commit, commit-msg, and pre-push hooks
```

- **commit**: `rustfmt --check` (changed files), `codespell`, `black`
- **commit-msg**: Conventional Commits with a **mandatory scope** —
  `type(scope): subject`, validated against [`TAGS.md`](./TAGS.md) and
  [`SCOPES.md`](./SCOPES.md)
- **push**: `cargo clippy --workspace --all-targets`

See [AGENTS.md](./AGENTS.md) for the full contributor conventions.
