# Civsim CLI Roadmap: Non-REPL Multiplayer Game

## Goal

Rebuild civsim as a stateless, file-backed CLI where each invocation reads game
state from a JSON file, performs one action, writes state back, and emits deltas
as JSON on stdout. Enables multiplayer (humans share a game file) and programmatic
integration.

## Phases

See the plan file for full details. Summary:

- **Phase 0**: libciv foundation (serde on StateDelta, turn_done/player_config on GameState)
- **Phase 1**: civsim infrastructure (state_io, output, player_view)
- **Phase 2**: CLI definition (clap subcommands for all 40+ actions)
- **Phase 3**: Handlers (new_game, action, end_turn, view, status, list) — all parallel
- **Phase 4**: Integration (main.rs dispatch, legacy backward compat)
- **Phase 5**: Tests (single-player, multiplayer, fog-of-war)
