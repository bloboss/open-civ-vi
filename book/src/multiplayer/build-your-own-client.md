# Build Your Own Client

The open4x **API is the product**; the renderer is replaceable. The game server
(`open4x-server`) exposes a fully typed REST + WebSocket surface, and the
official browser client (`open4x-client-js`) is just one consumer of it — a
lightweight reference you can fork, replace, or ignore. This page is the
contract for writing your own client in any language.

## The contract surface

| Concern | Source of truth |
|---|---|
| REST endpoints, params, bodies, responses | [`openapi.json`](./openapi.json) (OpenAPI 3) + [REST API Reference](./rest-api.md) |
| WebSocket messages (`/ws`) | [Wire Protocol](./protocol.md) — `open4x_protocol::v1::messages` is canonical |
| Wire data shapes (every type) | `open4x-protocol` crate (`v1::*`), generated to TypeScript (below) |
| Auth handshake | [Game Server](./server.md) + this page |

All three are generated/derived from the same Rust crate, so they cannot drift:

- `cargo xtask gen-openapi` regenerates `openapi.json`; `cargo xtask` `OpenapiCheck`
  fails CI if it's stale.
- `cargo xtask gen-ts` regenerates the TypeScript bindings; the TS-check leg of
  `cargo xtask check` fails CI on drift **or** on a type-name collision (the
  completeness guard).

## Generated TypeScript types

Run:

```bash
cargo xtask gen-ts   # writes open4x-client-js/src/gen/protocol/*.ts + a protocol.ts barrel
```

Every wire type (`PlayerState`, `WorldSnapshot`, `TileView`, `CityRow`,
`TechTreeView`, `MutationResponse<T>`, the `ClientMessage`/`ServerMessage`
WebSocket unions, …) is emitted as a `.ts` type alias via
[`ts-rs`](https://github.com/Aleph-Alpha/ts-rs). ID newtypes are `string`;
enums are externally-tagged unions that match the JSON exactly. Non-TypeScript
clients can read these as a schema reference, or consume `openapi.json` directly.

## Two-layer architecture (and the renderer boundary)

The reference client is deliberately split so the network layer is reusable:

```
open4x-client-js/src/
  gen/protocol/    ← generated wire types (do not edit)
  api.ts           ← the ONLY network layer: typed fetch over /api/v1 + auth
  render/          ← WebGL2 hex renderer (gl.ts, hexgeom.ts, hexmap.ts)
  ui/              ← HUD, tabs (tech/civics/city/units/diplomacy/empire/victory/
                     government), drawers (notifications/turn-queue/overlays)
  main.ts          ← wires Api → GameUI (renderer + screens)
```

`render/` and `ui/` depend on `api.ts`; **`api.ts` never depends on them**. To
bring your own renderer, keep `api.ts` (or reimplement the same calls in your
language) and replace everything above it. `api.ts` is plain ES modules with no
runtime dependencies — it is the canonical worked example of the REST contract.

## Auth handshake

Every `/api/v1/*` call (except `/health` and `POST /api/v1/games/new`) needs a
bearer token:

```
Authorization: Bearer <token>
```

Two ways to obtain one:

1. **Standalone / dev** — `POST /api/v1/games/new` bootstraps a single-player
   session and returns `{ game_id, civ_id, token, turn }`. No auth required.
2. **Via the lobby** — `open4x-lobby` authenticates the user (email magic-link
   or Ed25519 pubkey challenge-response), creates the game, and hands the
   browser back to the game server at `…/?token=<token>`. The client reads the
   `?token=` query param. The *same* Ed25519 keypair can authenticate against
   the lobby and the in-game server.

`api.ts` implements both: `Api.fromLocation()` (reads `?token=`) and
`Api.bootstrap()` (calls `games/new`).

## Reads, mutations, and the loop

- Reads are `GET /api/v1/<resource>` returning the mapped type
  (e.g. `GET /player-state` → `PlayerState`).
- Mutations are `POST/PATCH/DELETE` returning a `MutationResponse<T>`
  (`{ ok, view, turn_status }`); refetch the affected read slices after.
- `POST /api/v1/turn/end` returns **400 `unresolved_required_actions`** when
  required turn-queue items remain — surface them and let the user resolve.

That's the whole contract. Point a renderer at it and you have a client.
