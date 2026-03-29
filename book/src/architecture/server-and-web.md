# Server & Web Client

The multiplayer stack is a single `open4x-server` crate with feature flags: `ssr` for the native Axum server and `csr` for the Leptos/WASM browser client. Wire-protocol types live in the `types` module, shared by both features.

> **History**: Previously three crates (`open4x-api`, `open4x-server`, `open4x-web`) were merged to eliminate duplication.

## Wire Protocol Types (`types` module)

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

## Server (`ssr` feature)

### Architecture

```
HTTP Server (Axum)
+-- GET /ws            -> WebSocket upgrade
+-- GET /health        -> Health check ("ok")
+-- GET /api/demo-game -> JSON demo game result
+-- GET /api/game/*    -> REST API (bearer token auth)
+-- Static files       -> Trunk-built frontend
```

### State Management

```rust
struct AppState {
    games: DashMap<GameId, GameRoom>,       // concurrent map of active games
    players: DashMap<[u8; 32], PlayerRecord>, // persistent player profiles
    api_tokens: DashMap<String, ApiTokenRecord>, // REST API bearer tokens
    templates: Vec<CivTemplate>,            // built-in civ definitions
}
```

Each `GameRoom` holds a full `GameState`, `DefaultRulesEngine`, player slots, AI agents, and a broadcast channel for push updates.

### WebSocket Flow

1. **Authentication**: Server sends `Challenge { nonce }` -> client signs with Ed25519 private key -> server verifies -> `AuthSuccess { session_token, profile }`
2. **Lobby**: `ListGames` -> `GamesList`, `CreateGame` -> `GameCreated`, `JoinGame` -> `GameJoined { view }`
3. **Gameplay**: `Action(GameAction)` -> `ActionResult { ok, error }`, `EndTurn` -> `TurnResolved { new_turn, view }`

### REST API

Bearer-token authenticated endpoints for programmatic access:

| Endpoint | Description |
|----------|-------------|
| `/api/game/view` | Full `GameView` projection (enables custom renderers) |
| `/api/game/cities` | City report for all own cities |
| `/api/game/city/{id}` | Detailed report for a specific city |
| `/api/game/resources` | Resource inventory |
| `/api/game/units` | Unit roster |
| `/api/game/map-stats` | Terrain/feature/resource counts |
| `/api/game/players` | Known civilization info |
| `/api/game/science` | Tech tree progress |
| `/api/game/culture` | Civic tree progress |
| `/api/game/turn` | Turn status (for future webhook support) |

### Fog-of-War Projection

The `projection` module converts internal `GameState` into per-player `GameView`:
- Only explored tiles appear in the board view
- Only visible units are included
- Own cities show full detail; foreign cities show limited info
- The player's own civ shows full research/government/yield detail; others show public summary only

### Deployment

The server is containerized with a multi-stage Dockerfile:
1. Build `open4x-server` binary with `--features ssr` (Rust 1.86)
2. Build WASM frontend with `trunk` from `open4x-server/index.html`
3. Package into Debian slim runtime image

Exposes port `3001`. Persistent game data stored in a Docker volume at `/app/data`.

## Frontend (`csr` feature)

### Tab-Based UI

The game interface uses a tab system instead of a sidebar overlay:

| Tab | Description |
|-----|-------------|
| Map | Hex viewport with tile/unit info sidebar |
| Data Reports | Sub-tabs: Cities, Resources, Units, Map Stats |
| Science | Tech tree grid with status coloring and click-to-research |
| Culture | Civic tree grid with inspiration tracking |
| Governors | Governor management (placeholder) |
| Great People | Great person tracking (placeholder) |
| Climate | Climate monitoring (placeholder) |
| Players | Opponent data with diplomacy status |
| City | Individual city management with production queue |

### WebSocket Client

```rust
struct WsClient {
    socket: Rc<WebSocket>,
}
```

The client connects to the server's `/ws` endpoint, performs Ed25519 authentication, and exchanges `ClientMessage`/`ServerMessage` JSON frames. The `WsClient::connect()` method sets up an `onmessage` callback that feeds server messages into Leptos reactive signals for UI updates.

### Hex Map Renderer

The `hexmap` module renders the game board as SVG:
- Pointy-top hexagons colored by terrain type
- Click handlers for tile and unit selection
- Movement and attack interactions

### Build

The frontend is compiled to `wasm32-unknown-unknown` by Trunk. The `.cargo/config.toml` sets the `getrandom_backend="wasm_js"` rustflag required for `getrandom 0.3` compatibility in WASM.
