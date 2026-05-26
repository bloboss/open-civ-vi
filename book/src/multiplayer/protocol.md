# Wire Protocol (WebSocket)

The multiplayer surface uses WebSocket frames carrying JSON-encoded
`ClientMessage` / `ServerMessage` values. All types live in
[`open4x-protocol::v1::messages`](https://github.com/bloboss/open-civ-vi/blob/main/open4x-protocol/src/v1/messages.rs)
and are the **single source of truth** — this page mirrors the Rust enums,
so when the enums change the relevant section here must change with them.

The single-player REST surface at `/api/v1/*` is documented separately in
[`rest-api.md`](./rest-api.md). OpenAPI does not model WebSockets; for the
WS surface, the Rust types are the canonical reference.

## Authentication

`/ws` uses Ed25519 public-key authentication:

1. Client connects to `ws://server/ws`.
2. Server sends `Challenge { nonce: Vec<u8> }`.
3. Client signs the nonce with its Ed25519 private key.
4. Client sends `Authenticate { pubkey: Vec<u8>, signature: Vec<u8> }`.
5. Server verifies the signature.
6. Server responds with `AuthSuccess { session_token, profile }` or
   `AuthFailure { reason }`.

The client's public key is its persistent identity. Private keys are
generated client-side and stored in browser `localStorage`.

## Client → Server (`ClientMessage`)

| Variant | Payload | Description |
|---|---|---|
| `Authenticate`    | `{ pubkey, signature }`        | Ed25519 auth response |
| `SetProfile`      | `ProfileUpdate`                | Update display name + civ template |
| `CreateGame`      | `CreateGameRequest`            | Create a new game room |
| `JoinGame`        | `{ game_id }`                  | Join an existing game |
| `ListGames`       | —                              | Request available games |
| `Action`          | `GameAction`                   | Submit a game action (see below) |
| `EndTurn`         | —                              | Signal turn completion |
| `Ping`            | —                              | Keepalive |

`CreateGameRequest`:

```text
{ name, width, height, seed, num_ai, max_players, turn_limit? }
```

`ProfileUpdate`:

```text
{ display_name, selected_template }
```

## Server → Client (`ServerMessage`)

| Variant | Payload | Description |
|---|---|---|
| `Challenge`        | `{ nonce }`                       | Auth challenge |
| `AuthSuccess`      | `{ session_token, profile }`      | Auth succeeded |
| `AuthFailure`      | `{ reason }`                      | Auth failed |
| `GamesList`        | `Vec<GameListEntry>`              | Available games |
| `GameCreated`      | `{ game_id }`                     | Game created |
| `GameJoined`       | `{ game_id, view }`               | Joined game with initial state |
| `GameUpdate`       | `GameView`                        | Full game state update |
| `ActionResult`     | `{ ok, error? }`                  | Action success/failure |
| `TurnResolved`     | `{ new_turn, view }`              | Turn processed |
| `PlayerEndedTurn`  | `{ civ_id }`                      | Peer player ended their turn |
| `GameOver`         | `{ view }`                        | Game ended |
| `ProfileUpdated`   | `ProfileView`                     | Profile change confirmed |
| `TemplatesList`    | `Vec<CivTemplate>`                | Available civ templates |
| `Pong`             | —                                 | Keepalive response |
| `Error`            | `{ message }`                     | Generic error |

`GameListEntry`:

```text
{ game_id, name, players_joined, max_players, turn, status }
```

`GameStatus`: `Lobby` | `InProgress` | `Finished`.

## Game actions (`GameAction`)

Each variant in
[`messages::GameAction`](https://github.com/bloboss/open-civ-vi/blob/main/open4x-protocol/src/v1/messages.rs)
is a player intent. The list below mirrors the enum verbatim:

```text
MoveUnit         { unit, to: HexCoord }
Attack           { attacker, defender }
FoundCity        { settler, name }
PlaceImprovement { coord, improvement }
AssignCitizen    { city, tile, lock }
AssignCityFocus  { city, focus }
RenameCity       { city, name }
UnassignCitizen  { city, tile }
QueueProduction  { city, item: ProductionItemView }
CancelProduction { city, index }
EstablishTradeRoute { trader, destination }
QueueResearch    { tech }
CancelResearch                          (idempotent — drops the active tech)
CancelCivic                             (idempotent — drops the active civic)
ChangeGovernment { name }
QueueCivic       { civic }
DeclareWar       { target }
MakePeace        { target }
AssignPolicy     { policy }
FoundPantheon    { belief }
FoundReligion    { prophet, name, beliefs }
SpreadReligion   { unit }
TheologicalCombat { attacker, defender }
PurchaseWithFaith { city, item }
```

Some of these (notably the religion actions, `EstablishTradeRoute`, and the
treaty flow) are not yet wired through to the REST surface — the WS path is
their only home today. The REST surface exposes the ones that the
single-player HUD needs; see
[`open4x-server/src/server/rest/handlers.rs`](https://github.com/bloboss/open-civ-vi/blob/main/open4x-server/src/server/rest/handlers.rs)
for the current coverage.

## `GameView` (the fog-of-war-filtered projection)

`GameView` is what the server sends to each client; it is a projection of
the authoritative game state filtered for the recipient civ.
Definition lives in
[`open4x-protocol::v1::view`](https://github.com/bloboss/open-civ-vi/blob/main/open4x-protocol/src/v1/view.rs):

```text
GameView {
    turn,
    my_civ_id,
    board: BoardView,                  // only explored tiles
    my_civ: CivView,                   // full detail
    other_civs: Vec<PublicCivView>,    // limited info
    cities: Vec<CityView>,             // full for own, limited for foreign
    units:  Vec<UnitView>,             // only visible units
    tech_tree:  TechTreeView,
    civic_tree: CivicTreeView,
    trade_routes: Vec<TradeRouteView>,
    unit_type_defs, building_defs,
    scores: Vec<(CivId, u32)>,
    religions,
    game_over: Option<GameOverView>,
}
```

### Visibility filtering

- **Tiles**: only tiles in `explored_tiles` are sent; visibility state
  (`Visible` vs `Foggy`) is included.
- **Units**: only units on tiles in `visible_tiles` are sent.
- **Cities**: own cities include full production / population detail;
  foreign cities show only name, owner, and location.
- **Civilization**: own civ includes research queue, gold, policies,
  unlocks; other civs show only name, leader, score, and diplomatic
  status.
