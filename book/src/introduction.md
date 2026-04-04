# Open Civ VI

**Open Civ VI** is an open-source Rust implementation of a Civilization VI-style 4X strategy game engine. The project provides a complete game simulation -- terrain, cities, units, combat, research, diplomacy, trade, and more -- designed from the ground up for correctness, extensibility, and eventual use as a reinforcement-learning environment.

## Goals

- **Functional game engine** -- all core Civ VI mechanics implemented in pure Rust with no graphical dependency in the core library.
- **RL-ready** -- every state mutation produces a semantic diff (`GameStateDiff`), enabling replay, observation, and reward shaping for AI agents.
- **Extensibility** -- game content (civilizations, units, terrain, buildings, wonders) is defined by implementing Rust traits. New content links at compile time; there is no runtime scripting layer.
- **Multiplayer** -- an Axum-based WebSocket server and a Leptos/WASM browser client provide real-time multiplayer gameplay.

## Project Status

The engine is under active development. Core systems -- hex grid, terrain, cities, units, combat, research, trade, cultural borders, map generation, era scoring, loyalty, tourism, victory conditions, and a deterministic AI agent -- are implemented and tested with **200+ integration tests**. See the [Roadmap](./roadmap.md) for detailed implementation status.

## Crate Overview

| Crate | Role |
|-------|------|
| `libhexgrid` | Pure hex geometry -- coordinates, pathfinding, line of sight |
| `libciv` | All game state and rules -- world, civilizations, rules engine, AI |
| `open4x` (open4x-cli) | CLI binary for local simulation and interactive play |
| `open4x-server` | Merged server + frontend (`ssr` for Axum server, `csr` for Leptos/WASM client) |

## License & Contributing

The project is hosted at [github.com/bloboss/open-civ-vi](https://github.com/bloboss/open-civ-vi). Contributions are welcome -- see the roadmap for areas that need work.
