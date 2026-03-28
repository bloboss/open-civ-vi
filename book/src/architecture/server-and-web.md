# Server & Web Client

The multiplayer stack consists of three crates: `open4x-api` (shared types), `open4x-server` (Axum backend), and `open4x-web` (Leptos/WASM frontend).

## open4x-api -- Wire Protocol Types

Serializable mirror types for the game state, shared between server and client. All types derive `Serialize`/`Deserialize`.

### Key Types

| Type | Description |
|------|-------------|
| `ClientMessage` | All messages the client can send (auth, actions, lobby) |
| `ServerMessage` | All messages the server can send (views, results, errors) |
| `GameAction` | Player actions: move, attack, found city, build, research, trade, diplomacy |
| `GameView` | Full game state projection for one player (fog-of-war filtered) |
| `BoardView` | Tile grid with terrain, features, resources, visibility |
| `CivView` | Full detail for the player's own civilization |
| `PublicCivView` | Limited info for other civilizations |
| `CityView` | City detail (full for own cities, limited for foreign) |
| `UnitView` | Unit state (position, health, movement) |
| `TechTreeView` | Tech tree with research status |
| `ProfileView` | Player profile with display name and civ template |
| `CivTemplate` | Civilization definition (name, leader, abilities, uniques) |

### GameAction Variants

```rust
enum GameAction {
    MoveUnit { unit, to },
    Attack { attacker, defender },
    FoundCity { settler, name },
    PlaceImprovement { coord, improvement },
    QueueProduction { city, item },
    CancelProduction { city, index },
    EstablishTradeRoute { trader, destination },
    QueueResearch { tech },
    QueueCivic { civic },
    AssignCitizen { city, tile, lock },
    UnassignCitizen { city, tile },
    DeclareWar { target },
    MakePeace { target },
    AssignPolicy { policy },
}
```

## open4x-server -- Axum Backend

### Architecture

```
HTTP Server (Axum)
+-- GET /ws            -> WebSocket upgrade
+-- GET /health        -> Health check ("ok")
+-- GET /api/demo-game -> JSON demo game result
+-- Static files       -> Leptos/trunk frontend
```

### State Management

```rust
struct AppState {
    games: DashMap<GameId, GameRoom>,       // concurrent map of active games
    players: DashMap<[u8; 32], PlayerRecord>, // persistent player profiles
    templates: Vec<CivTemplate>,            // built-in civ definitions
}
```

Each `GameRoom` holds a full `GameState`, `DefaultRulesEngine`, player slots, AI agents, and a broadcast channel for push updates.

### WebSocket Flow

1. **Authentication**: Server sends `Challenge { nonce }` -> client signs with Ed25519 private key -> server verifies -> `AuthSuccess { session_token, profile }`
2. **Lobby**: `ListGames` -> `GamesList`, `CreateGame` -> `GameCreated`, `JoinGame` -> `GameJoined { view }`
3. **Gameplay**: `Action(GameAction)` -> `ActionResult { ok, error }`, `EndTurn` -> `TurnResolved { new_turn, view }`

### Fog-of-War Projection

The `projection` module converts internal `GameState` into per-player `GameView`:
- Only explored tiles appear in the board view
- Only visible units are included
- Own cities show full detail; foreign cities show limited info
- The player's own civ shows full research/government/yield detail; others show public summary only

### Deployment

The server is containerized with a multi-stage Dockerfile:
1. Build `open4x-server` binary (Rust 1.86)
2. Build WASM frontend with `trunk`
3. Package into Debian slim runtime image

Exposes port `3001`. Persistent game data stored in a Docker volume at `/app/data`.

## open4x-web -- Leptos/WASM Frontend

### Pages

| Page | Description |
|------|-------------|
| `Home` | Main menu with buttons for New Game, Settings, Players, Demo |
| `MapConfig` | Map size and seed configuration -> starts a game |
| `Game` | Main gameplay view with hex canvas, unit controls, action buttons |
| `DemoConfig` | AI-vs-AI demo parameters |
| `Replay` | Animation of demo game results |
| `Settings` | User preferences (deferred) |
| `Players` | Player list (deferred) |

### WebSocket Client

```rust
struct WsClient {
    socket: Rc<WebSocket>,
}
```

The client connects to the server's `/ws` endpoint, performs Ed25519 authentication, and exchanges `ClientMessage`/`ServerMessage` JSON frames. The `WsClient::connect()` method sets up an `onmessage` callback that feeds server messages into Leptos reactive signals for UI updates.

### Hex Map Renderer

The `hexmap` module renders the game board on an HTML5 canvas:
- Hex tiles with terrain coloring
- Click handlers for tile and unit selection
- Movement and attack range visualization

### Build

The frontend is compiled to `wasm32-unknown-unknown` by Trunk. The `.cargo/config.toml` sets the `getrandom_backend="wasm_js"` rustflag required for `getrandom 0.3` compatibility in WASM.
