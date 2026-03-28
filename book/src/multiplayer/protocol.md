# Wire Protocol

The multiplayer system uses WebSocket frames carrying JSON-encoded messages. The `open4x-api` crate defines all message types, shared between the server and WASM client.

## Authentication

The protocol uses Ed25519 public-key authentication:

1. Client connects to `ws://server/ws`
2. Server sends `Challenge { nonce: Vec<u8> }`
3. Client signs the nonce with its Ed25519 private key
4. Client sends `Authenticate { pubkey: Vec<u8>, signature: Vec<u8> }`
5. Server verifies the signature
6. Server responds with `AuthSuccess { session_token, profile }` or `AuthFailure { reason }`

The client's public key is its persistent identity. Private keys are generated client-side and stored in browser `localStorage`.

## Message Types

### Client -> Server

| Message | Description |
|---------|-------------|
| `Authenticate { pubkey, signature }` | Ed25519 auth response |
| `SetProfile(ProfileUpdate)` | Update display name and civ template |
| `CreateGame(CreateGameRequest)` | Create a new game room |
| `JoinGame { game_id }` | Join an existing game |
| `ListGames` | Request list of available games |
| `Action(GameAction)` | Submit a game action |
| `EndTurn` | Signal turn completion |
| `Ping` | Keepalive |

### Server -> Client

| Message | Description |
|---------|-------------|
| `Challenge { nonce }` | Auth challenge |
| `AuthSuccess { session_token, profile }` | Auth succeeded |
| `AuthFailure { reason }` | Auth failed |
| `GamesList(Vec<GameListEntry>)` | Available games |
| `GameCreated { game_id }` | Game created successfully |
| `GameJoined { game_id, view }` | Joined game with initial state |
| `GameUpdate(GameView)` | Full game state update |
| `ActionResult { ok, error }` | Action success/failure |
| `TurnResolved { new_turn, view }` | Turn processed, new state |
| `PlayerEndedTurn { civ_id }` | Another player finished their turn |
| `GameOver { view }` | Game ended |
| `ProfileUpdated(ProfileView)` | Profile change confirmed |
| `TemplatesList(Vec<CivTemplate>)` | Available civilization templates |
| `Pong` | Keepalive response |
| `Error { message }` | Generic error |

## Game Actions

The `GameAction` enum covers all player actions:

```
MoveUnit { unit, to }
Attack { attacker, defender }
FoundCity { settler, name }
PlaceImprovement { coord, improvement }
QueueProduction { city, item }
CancelProduction { city, index }
EstablishTradeRoute { trader, destination }
QueueResearch { tech }
QueueCivic { civic }
AssignCitizen { city, tile, lock }
UnassignCitizen { city, tile }
DeclareWar { target }
MakePeace { target }
AssignPolicy { policy }
```

## Game View

The `GameView` is a fog-of-war-filtered projection of the game state for a single player:

```
GameView {
    turn,
    my_civ_id,
    board: BoardView,          // only explored tiles
    my_civ: CivView,           // full detail
    other_civs: Vec<PublicCivView>,  // limited info
    cities: Vec<CityView>,     // full for own, limited for foreign
    units: Vec<UnitView>,      // only visible units
    tech_tree: TechTreeView,
    civic_tree: CivicTreeView,
    trade_routes: Vec<TradeRouteView>,
    unit_type_defs, building_defs,
    scores: Vec<(CivId, u32)>,
    game_over: Option<GameOverView>,
}
```

### Visibility Filtering

- **Tiles**: only tiles in `explored_tiles` are sent; visibility state (`Visible` vs `Foggy`) is included
- **Units**: only units on tiles in `visible_tiles` are sent
- **Cities**: own cities include full production/population detail; foreign cities show only name, owner, and location
- **Civilization**: own civ includes research queue, gold, policies, unlocks; other civs show only name, leader, score, and diplomatic status
