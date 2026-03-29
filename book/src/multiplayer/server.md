# Game Server

The `open4x-server` crate provides an Axum-based HTTP/WebSocket server for multiplayer games.

## Architecture

```
Axum Router
  |-- GET /ws           WebSocket upgrade handler
  |-- GET /health       Health check endpoint
  |-- GET /api/demo-game  AI-vs-AI demo (JSON response)
  |-- Static files      Leptos/trunk frontend (from OPEN4X_STATIC_DIR)
```

## State

### AppState

Shared across all connections via `Arc`:

```rust
struct AppState {
    games: DashMap<GameId, GameRoom>,
    players: DashMap<[u8; 32], PlayerRecord>,
    templates: Vec<CivTemplate>,
}
```

- `games`: concurrent hash map of active game rooms
- `players`: persistent player profiles indexed by Ed25519 public key
- `templates`: built-in civilization templates (Rome, Babylon, Greece, etc.)

### GameRoom

Per-game state:

```rust
struct GameRoom {
    game_id: GameId,
    name: String,
    state: GameState,
    rules: DefaultRulesEngine,
    players: Vec<PlayerSlot>,
    ai_agents: Vec<(CivId, HeuristicAgent)>,
    status: GameStatus,  // Lobby | InProgress | Finished
    config: GameRoomConfig,
    tx: broadcast::Sender<ServerMessage>,
}
```

The broadcast channel (`tx`) pushes state updates to all connected players in the room.

### PlayerSlot

```rust
struct PlayerSlot {
    civ_id: CivId,
    pubkey: [u8; 32],
    profile: ProfileView,
    submitted_turn: bool,
}
```

## Game Flow

### Creating a Game

1. Client sends `CreateGame { name, width, height, seed, num_ai, max_players, turn_limit }`
2. Server generates a map using `world::mapgen::generate()`
3. Server creates civilizations for AI players and assigns `HeuristicAgent`s
4. Server returns `GameCreated { game_id }`

### Joining a Game

1. Client sends `JoinGame { game_id }`
2. Server assigns a civilization to the player based on their selected template
3. Server sends `GameJoined { game_id, view }` with the initial game view
4. Other players in the room receive notification

### Gameplay

1. Players send `Action(GameAction)` messages during their turn
2. Server validates and applies each action via `GameRoom::apply_action()`
3. Server broadcasts `ActionResult { ok, error }` to the acting player
4. When a player is done, they send `EndTurn`
5. Server broadcasts `PlayerEndedTurn { civ_id }` to all players

### Turn Resolution

When all human players have ended their turn:
1. Server runs `HeuristicAgent::take_turn()` for each AI civilization
2. Server calls `RulesEngine::advance_turn()` to process the turn
3. Server broadcasts `TurnResolved { new_turn, view }` with updated state
4. Each player receives their own fog-of-war-filtered `GameView`

### Game Over

When a victory condition is met during turn resolution:
1. `game_over` is set on the `GameState`
2. Server broadcasts `GameOver { view }` to all players
3. `GameStatus` changes to `Finished`

## Projection Layer

The `projection` module converts internal game state to player-visible views:
- Converts `libciv` types to wire-protocol types
- Applies fog-of-war filtering per player
- Computes effective yields with all modifiers applied
- Handles coordinate system conversion

## Deployment

### Docker

```dockerfile
# Multi-stage build
FROM rust:1.86 AS build-server    # compile server binary
FROM rust:1.86 AS build-web       # compile WASM frontend with trunk
FROM debian:bookworm-slim         # runtime image
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | 3001 | HTTP listen port |
| `OPEN4X_STATIC_DIR` | `./open4x-server/dist` | Path to frontend static files |
| `OPEN4X_DATA_DIR` | `./data` | Persistent data directory |

### Docker Compose

```bash
docker compose up --build
```

Exposes port 3001 with a health check on `GET /health` (30s interval).
