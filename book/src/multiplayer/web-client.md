# Web Client

The `open4x-server` crate (with the `csr` feature) provides a browser-based game client built with Leptos and compiled to WebAssembly.

## Technology Stack

- **Leptos** (v0.7, `csr` feature) -- reactive UI framework
- **wasm-bindgen** -- Rust/JavaScript interop
- **web-sys** -- Web API bindings (WebSocket, Canvas, Storage)
- **ed25519-dalek** -- Client-side key generation and signing
- **Trunk** -- WASM build tool and dev server

## Pages

| Page | Component | Description |
|------|-----------|-------------|
| Home | `HomePage` | Main menu with navigation buttons |
| Map Config | `MapConfigPage` | Set map size, seed, AI count before starting |
| Game | `GamePage` | Main gameplay: hex map, units, actions |
| Demo Config | `DemoConfigPage` | Configure AI-vs-AI demo parameters |
| Replay | `ReplayPage` | Animate and visualize demo game results |
| Settings | `SettingsPage` | User preferences (deferred) |
| Players | `PlayersPage` | Player list (deferred) |

## WebSocket Client

```rust
struct WsClient {
    socket: Rc<WebSocket>,
}
```

### Connection Flow

1. `WsClient::connect(url, on_msg)` creates a WebSocket connection
2. Sets up `onmessage` callback that deserializes `ServerMessage` JSON and calls `on_msg`
3. The closure is leaked to keep it alive for the connection lifetime
4. `send(msg)` serializes `ClientMessage` to JSON and transmits

### Authentication

On connection:
1. Server sends `Challenge { nonce }`
2. Client generates (or retrieves from `localStorage`) an Ed25519 keypair
3. Client signs the nonce and sends `Authenticate { pubkey, signature }`
4. On `AuthSuccess`, the client stores the session token and proceeds to the lobby

## Hex Map Renderer

The `hexmap` module renders the game board on an HTML5 `<canvas>` element:

- Hex tiles colored by terrain type
- Feature overlays (forests, mountains, etc.)
- Unit icons at their positions
- City markers with names
- Territory borders colored by owning civilization
- Click handlers for tile selection and unit commands
- Movement range and attack range highlighting

## Reactive State

Leptos signals drive the UI:
- `GameView` from the server updates reactive signals
- UI components re-render automatically when their signals change
- Page navigation uses a `Page` enum signal to switch between views

## Build

```bash
cd open4x-server
trunk serve          # dev server with hot reload
trunk build --release  # production build to dist/
```

The WASM target requires the `getrandom_backend="wasm_js"` rustflag, configured in `.cargo/config.toml` for the `wasm32-unknown-unknown` target. This ensures all transitive dependencies agree on the WASM random backend.

## Configuration

The client connects to the server WebSocket at the same origin by default. The `index.html` entry point loads the WASM module and initializes the Leptos app.
