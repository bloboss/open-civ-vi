# Accounts and Login — `open4x-lobby` + `open4x-accounts`

> **Status**: Phase 0 complete (workspace split + paper-theme SPA scaffold +
> screen ports). All subsequent phases pending.
> **Goal**: A self-hostable pre-game surface (landing → login → menu → new
> game wizard → profile) that authenticates real users, manages their
> identities, lists their ongoing games, and orchestrates per-game
> `open4x-server` instances. Distinct from the in-game runtime — see
> §1 for the rationale.

---

## 1. Why this exists

The Open4X stack splits cleanly into two concerns:

| Concern             | Lifetime                  | Persistence              | Crate                |
|---------------------|---------------------------|--------------------------|----------------------|
| In-game runtime     | one game session          | in-memory `GameRoom`     | `open4x-server`      |
| Pre-game surface    | spans many games          | account / identity DB    | `open4x-lobby`       |
| Identity substrate  | shared between the above  | account / identity DB    | `open4x-accounts`    |

`open4x-server` is single-purpose: hold one `GameRoom`, mutate via
`/api/v1/*`, project to clients. Auth there is anonymous bearer tokens
minted by `POST /games/new`. That's correct for the in-game runtime; it's
inadequate for a multi-game lobby where users come back over weeks, link
their email + GitHub + atproto to one identity, manage friends, and
resume games on multiple devices.

The lobby owns: auth (email magic-link, OIDC, atproto), user identity
graph, ongoing-games index, new-game wizard, profile and preferences. It
is the **only** thing the user touches before they're in a game.

Visual reference: `docs/open4x-landing/project/hifi/`. Paper theme, IBM
Plex Mono / Sans / Serif. Distinct from the in-game dark theme by design
— stepping from lobby into a game is a deliberate context switch.

---

## 2. Inventory

### 2.1 Phase 0 — done

- [x] Workspace members `open4x-lobby` (dual ssr/csr, mirrors
      `open4x-server` Cargo layout) and `open4x-accounts` (plain library)
- [x] `open4x-accounts` type skeleton: `PlayerId` (16-hex dot-grouped),
      `Identity` enum (Email · OpenId · Atproto), `Account`, `Preferences`,
      `MagicLinkToken`
- [x] `open4x-lobby` Leptos SPA scaffold: `index.html` with IBM Plex
      Google-Fonts links, `styles.css` copied verbatim from
      `docs/open4x-landing/project/hifi/styles.css`
- [x] Shared primitives ported: `Btn`, `Tag`, `Toggle`, `Segmented`,
      `Panel` / `PanelHead`, `MiniMap`. Popup is stubbed as `Trigger`
      (renders the underlined help-trigger as a `<span class="trigger">`
      with a `title=` fallback)
- [x] Screens — full ports: **Landing**, **Login**, **Profile**
- [x] Screens — partial ports: **MenuShell** (sidebar + secondary-nav
      placeholders), **OngoingGames** (tile grid with sample data baked
      in), **NewGame** (5-step strip + Map step + Review step; Civ /
      Rules / Players steps render placeholder bodies)
- [x] App shell with `RwSignal<Screen>` route switcher (Landing | Login |
      Menu) + nested `RwSignal<MenuTab>` (Ongoing | NewGame | Profile)
- [x] Trunk + Axum boot path — `trunk build --release --features csr
      --no-default-features` produces a 709 KB wasm + 24 KB hashed CSS;
      `cargo run -p open4x-lobby` serves on `:3002`

### 2.2 Pending

Everything else. See §4.

---

## 3. End-state architecture

```
                                       ┌─ DB (sqlite default · pg opt) ─┐
                                       │   accounts · identities ·      │
                                       │   sessions · games_index       │
                                       └────────────▲───────────────────┘
                                                    │
┌─ open4x-accounts (lib) ──────────────┐  reads /   │
│                                      │  writes    │
│   PlayerId · Identity · Account      │◄───────────┘
│   MagicLink minting · OIDC client    │
│   atproto resolver · session tokens  │
│                                      │
└──────────▲────────────────────▲──────┘
           │                    │
           │ uses               │ validates inbound tokens
           │                    │
┌──────────┴───────┐   ┌────────┴──────────────┐
│ open4x-lobby     │   │ open4x-server         │
│ (axum + leptos)  │   │ (axum + leptos)       │
│                  │   │                       │
│ /landing /login  │   │ /api/v1/* + /ws       │
│ /menu /api/v1/me │   │ in-game runtime       │
│ /api/v1/games ──►│   │                       │
│ orchestrates     │──►│ POST /games/new       │
│ per-game server  │   │ (token-validated)     │
└──────────────────┘   └───────────────────────┘
```

The accounts crate is the substrate. The lobby crate consumes it for the
pre-game surface and exposes the orchestration HTTP. The in-game server
trusts tokens minted by the accounts crate (via shared signing key in
the simple case, or HTTP introspection later).

For single-machine dev: lobby and one or more in-game servers run as
separate processes on adjacent ports; the lobby reverse-proxies or
redirects clients to the right `:3001+n` URL per game. For multi-host
deploys: same model with the orchestration layer talking to a process
manager (systemd / podman / k8s) instead of `Command::spawn`.

---

## 4. Phased plan

Each phase ends in a working, mergeable state. None requires more than
one accounts PR + one lobby PR.

### Phase 1 — Visual completeness ✅

Get the SPA pixel-perfect against the design before wiring anything to
the network. No persistence, no auth, no orchestration; just finishing
the JSX → Leptos translation so we can iterate UX without backend
plumbing. **Done.**

- [x] **Popup component** — `open4x-lobby/src/components/popup.rs`
      ports the Gwern-style behaviour: 180 ms hover-show, 140 ms
      hide-grace, click-to-pin, Esc / click-outside dismiss for pinned
      popups, viewport-aware vertical flip + horizontal clamp.
      `PopupProvider` mounts at the app root and owns a single
      `RwSignal<Option<PopupState>, LocalStorage>`; each `Popup`
      wrapper captures its anchor's `DOMRect` and asks the provider
      to show. `view_fn: Arc<dyn Fn() -> AnyView>` so `PopupState` is
      cheap-Clone. Timers live in `Arc<SendWrapper<RefCell<Option<
      Timeout>>>>` so the context is `Send + Sync` while keeping the
      single-threaded gloo `Timeout` alive.
      Trigger-stub call sites still work; migration to real popups
      will happen as each screen wires its `<Popup>` sites in
      subsequent commits.
- [x] **`PopupBody`, `PopupActions`, `PopupList`** — body containers
      live at `open4x-lobby/src/components/popup_body.rs`. `PopupBody`
      / `PopupActions` are pure layout wrappers; `PopupList` takes a
      `Vec<PopupListItem>` (`Row { icon, label, desc? }` or
      `Separator`) so menu definitions read structurally rather than
      as untyped JSX. No interactivity yet — wired up when the Popup
      shell lands.
- [x] **NewGame `StepCiv`** — civ picker grid implemented in
      `open4x-lobby/src/screens/newgame.rs::StepCiv`. Static
      `CIVS: &[CivPick]` table holds leader / civ / trait + unique
      unit / unique building / leader ability / civ ability. Each
      card is a `<Popup>` wrapping a `<div class=panel>` with hover
      to surface the `CivSheet` body, click to select. Selection
      stored on a screen-local `RwSignal<String>` keyed on leader
      name; the active card flips its border + background to
      `var(--accent-soft)` per the JSX. Real catalogue
      handshake with libciv lands in Phase 4 alongside the games
      index.
- [x] **NewGame `StepRules`** — `screens/newgame.rs::StepRules`
      ships the two-panel layout: difficulty / era / game-speed
      Segmenteds, six victory-condition Toggles seeded from a
      `VICTORY_CONDITIONS` table (Diplomacy off by default per the
      JSX), plus the world-dynamics sliders (disasters 0-4 with the
      off/light/std/heavy/apocalyptic categorical formatter,
      barbarians 0-4 off/rare/std/raging/horde, city-states 0-24,
      AI aggression with passive/balanced/warlike thresholds at
      34/66) and the AI-personality Segmented (historic / random /
      scripted). Help triggers are real `<Popup>` wrappers.
- [x] **NewGame `StepPlayers`** — `screens/newgame.rs::StepPlayers`.
      Renders the design's 8-slot baseline (1 human-you, 1 open
      invite slot, 6 AI). Each row picks its `Tag` variant from
      a `SlotKind` (Human → `accent-soft`, Open → default, AI →
      `dim`) and shows either an invite Popup (click-pinned, with
      email/OpenID/atproto/PlayerID input + recent-recipients chip
      list) or a slot-management `PopupList` (Change civ / AI
      personality / Swap with… / sep / Remove slot). Turn-mode
      panel ships timer Segmented (off/5min/10min/30min/24hr),
      simultaneous / private-game / cross-play Toggles with live
      muted captions. Removes the now-redundant `StepPlaceholder`
      helper in newgame.rs.
- [x] **`Slider` primitive** — wraps `<input type="range">` with an
      optional `format: Arc<dyn Fn(i32) -> String>` callback for the
      value display. `RwSignal<i32>`-driven; `min` / `max` default to
      `0` / `100`. Lives at
      `open4x-lobby/src/components/slider.rs`.
- [x] **Tweaks panel port** — `components/tweaks_panel.rs` ships a
      fixed-position card (bottom-right) with a Segmented density
      picker. App owns the `RwSignal<String>` density and binds it
      to the root `.app` element's `data-density` attr (was a
      hard-coded literal). Collapsing into a "⚙ tweaks" pill is
      supported. Sliders / color pickers / postMessage host
      protocol from the JSX original are deliberately omitted —
      out of scope today; extend the module if needed.

**Done when**: every screen in `docs/open4x-landing/project/hifi/`
renders identically to the JSX prototype (ignoring purposeful Leptos
restructures). All popups work. Tweaks panel toggles density live.

### Phase 2 — `open4x-accounts` substrate

The crate today is types-only. This phase puts persistence and token
minting behind those types.

#### 2.1 Persistence

- [x] Pick storage: **sqlx + sqlite by default**, postgres opt-in.
      Migrations live in `open4x-accounts/migrations/0001_initial.sql`.
      Cargo features: `persistence` (default off — pulls sqlx +
      tokio + chrono + thiserror + async-trait); `postgres` opts in
      `sqlx/postgres` on top.
- [x] Schema (lives in `migrations/0001_initial.sql`): four tables —
      `accounts` (player_id text PK, preferred_name, pronouns, bio,
      prefs_json, created_at, updated_at), `identities` (id ULID
      text PK, player_id FK CASCADE, kind, primary_key text,
      label, is_primary, verified, created_at; `UNIQUE(kind,
      primary_key)`), `sessions` (token_hash hex PK, player_id FK,
      created_at, expires_at, revoked_at), and a separate
      `magic_link_nonces` table for the Phase 2.2 single-use nonce
      pool.
- [x] `AccountStore` trait + sqlite impl + in-memory test impl. The
      trait is `async_trait`, lives at
      `open4x-accounts/src/store.rs`, and ships every method named
      in the plan (`lookup_by_identity`, `link_identity`,
      `unlink_identity`, `find_or_create_account_for_identity`,
      `update_profile`, `delete_account`). Sqlite impl runs the
      embedded migrations on `connect()`. Mem impl + 2 mem tests
      cover idempotent find-or-create and cross-account-dup
      rejection.

#### 2.2 Magic-link tokens

- [x] `MagicLinkSigner` mint / verify via HMAC-SHA256 over
      `{email}|{expires_at_unix}|{nonce}` payload. Token shape
      `b64url(payload).b64url(sig)`. Single-use enforcement via the
      `magic_link_nonces` UPDATE-where-consumed_at-IS-NULL idiom (no
      race; double-spend returns `MagicLinkError::Reused`). 15-min
      default TTL via `DEFAULT_TTL`. `MagicLinkSigner::from_env_or_path`
      resolves the per-deployment 32-byte key from
      `OPEN4X_LOBBY_HMAC_KEY` (hex), falls back to
      reading/generating a 0600 file on disk. Lives at
      `open4x-accounts/src/magic_link.rs`. 6 tests cover round-trip
      / reuse-rejection / expired-rejection /
      tampered-signature-rejection / unknown-nonce / env-key-load.
- [x] Email transport: pluggable `Mailer` trait at
      `open4x-accounts/src/mailer.rs`. `Mailer: Send + Sync` async
      trait with `send_magic_link(email, link)` plus a `send_raw`
      escape hatch (default-impl errors `NotConfigured`).
      `LogMailer` impl writes the magic link to stderr in a
      grep-friendly `[magic-link] to=… link=…` shape. `SmtpMailer`
      lives behind the `mailer-smtp` cargo feature and uses
      lettre's `AsyncSmtpTransport<Tokio1Executor>` with rustls.
      `SmtpConfig::from_env` reads `SMTP_HOST` / `SMTP_PORT`
      (default 465) / `SMTP_USER` / `SMTP_PASS` / `SMTP_FROM`.
      AppState picks SMTP when all required env vars are set;
      otherwise falls back to LogMailer with a stderr note so
      operators see the choice on boot.

#### 2.3 OIDC client

- [x] `OidcProvider` enum (Google · GitLab · Microsoft · Custom) +
      `OidcConfig` (provider, client_id, client_secret, redirect_uri,
      scopes) at `open4x-accounts/src/oidc.rs`. Factory constructors
      seed the standard `openid email profile` scope set per
      provider.
- [x] PKCE + state + nonce generation (`Pkce::new` →
      `verifier` 32-bytes-base64url, `challenge` =
      base64url(sha256(verifier))).
- [x] `build_authorization_request(config)` returns the redirect
      URL + ephemeral state for the lobby to stash. Uses each
      provider's authorize endpoint directly today; the
      discovery-driven version lands alongside the exchange flow.
- [ ] OIDC discovery (`/.well-known/openid-configuration`) cached per
      issuer (deferred — needs the network half).
- [ ] Authorization-code exchange + ID-token verification (deferred
      — picks up `openidconnect` crate behind a separate feature).
- [ ] Claims → `Identity::OpenId{issuer, subject, label}` mapping
      (deferred).
- [ ] Custom-issuer discovery on first sign-in (deferred).
- [ ] **GitHub** is OAuth2-only (no ID token) and gets its own
      module under a follow-up Phase 2.3 task.

##### 2.3 part 2 — execution plan (network + verify + wiring)

The deterministic half above is done; this is the actionable plan for
the rest, grounded in the current code. Reuse `email_auth::verify` as
the template for the callback's session-mint + cookie + `Redirect::to
("/")` tail — it is the exact shape the OIDC callback ends in.

**Decision locked in:** use the **`openidconnect`** crate behind an
`oidc` cargo feature. The auth-URL builder was fine hand-rolled;
JWT/JWKS signature verification is not — it is where OIDC bugs become
auth bypasses. The crate handles discovery, JWKS rotation, and the
full claim-validation set.

1. **Substrate (`open4x-accounts`, `oidc` feature):**
   - Discovery: GET `/.well-known/openid-configuration`, cache per
     issuer (yields `authorization_endpoint` / `token_endpoint` /
     `jwks_uri`). Replaces the hard-coded endpoints in
     `build_authorization_request`.
   - Code exchange: POST `code` + PKCE `verifier` + client creds to
     `token_endpoint` → ID token (JWT) + access token.
   - ID-token verification (**security-critical**): fetch JWKS, verify
     RS256 signature, check `iss` / `aud`(== our `client_id`) / `exp`
     / `iat` / `nonce`.
   - Claims → `Identity::OpenId{issuer, subject, label}`. `(issuer,
     sub)` is the unique tuple per §5.

2. **Pending-flow storage:** stash `{state, nonce, pkce_verifier,
   provider, link_target?}` (~5 min TTL) in a **signed cookie**
   (HMAC with the existing `OPEN4X_LOBBY_HMAC_KEY`) — no new table and
   it survives the process-per-game deploy (unlike the in-memory
   `PubkeyChallengeStore`).

3. **Routes (`open4x-lobby/src/server/rest/oidc_auth.rs`, registered
   beside the pubkey routes in `rest/mod.rs`):**
   - `GET /auth/oidc/{provider}/start` → build request, set pending
     cookie, 302 to provider.
   - `GET /auth/oidc/{provider}/callback?code&state` → validate state,
     exchange, verify, find-or-create **or** `link_identity`, mint
     session, set `lobby_session`, redirect `/`.
   - `POST /auth/oidc/custom/start { issuer_url }` → discovery + 302.
   - `POST /me/identities/oidc/{provider}` → start with link intent
     (Profile "+ link another"), via the pending `link_target` field.

4. **AppState + config:** add `oidc_configs: HashMap<OidcProvider,
   OidcConfig>` (+ http client / JWKS cache) populated at boot from
   env (`OIDC_GOOGLE_CLIENT_ID` / `_SECRET`, …); `redirect_uri`
   derived from `public_base_url`. Unconfigured providers render no
   button.

5. **SPA:** Login provider buttons do a **full-page navigation** to
   `/auth/oidc/{provider}/start` (redirect flow, not `fetch`); the
   custom button reveals an issuer-URL input. Profile "+ link another"
   → the `/me/identities/oidc/...` start. **GitHub is OAuth2, not
   OIDC** (no ID token) — separate small handler, or drop its button
   for v1.

6. **Ops + tests:** rate-limit the callback per `state` (reuse the
   `email_auth` audit-log throttle); add OIDC sign-in / link / failure
   `AuditEventKind`s; mock discovery + token + JWKS with `wiremock` and
   cover bad-`aud` / expired-`exp` / wrong-`nonce` / tampered-signature
   / mismatched-`state`.

**Sequencing:** (1) substrate + mocked tests, no lobby changes → (2)
pending-cookie + the three routes + AppState config, smoke against
Google → (3) SPA buttons (Login then Profile) → (4) GitHub OAuth2 as a
separate follow-up. This is multi-commit, backend-heavy, security-
sensitive work — a directed reviewed effort, not autonomous-loop fodder.

#### 2.4 atproto

- [ ] Handle resolution: try `_atproto.<handle>` DNS TXT, fall back to
      `https://<handle>/.well-known/atproto-did`.
- [ ] DID resolution: PLC directory for `did:plc:`, web for `did:web:`.
      Returns the user's PDS endpoint.
- [ ] OAuth flow against the PDS (atproto-oauth / DPoP signed
      requests).
- [ ] Map identity to `Identity::Atproto{did, handle}`.

#### 2.5 Session tokens

- [x] Bearer-token shape: `lobby_<base64url(48 bytes of OS
      randomness)>`. Stored as SHA-256 hex in `sessions.token_hash`
      (DB compromise → no log-in mint).
- [x] `mint_session(pool, player_id, ttl) -> RawToken` — 30-day
      default TTL via `DEFAULT_TTL`. Lives in
      `open4x-accounts/src/session.rs`.
- [x] `validate_session(pool, raw) -> Option<PlayerId>` — single
      indexed SELECT keyed on the SHA-256 hash; rejects revoked or
      expired rows; rejects tokens missing the `lobby_` prefix
      without consulting the DB.
- [x] `revoke_session(pool, raw)` (idempotent) plus a
      `revoke_all_for_player(pool, player_id) -> u64` for the
      sign-out-everywhere path. 5 tests cover round-trip /
      revocation / expiry / unknown-token / revoke-all.

**Done when**: `cargo test -p open4x-accounts` covers the magic-link,
OIDC-claims-mapping, and session-token flows with a fake mailer + a
mocked OIDC discovery server. CLI tool to mint a session for a manually
inserted account, useful for the Phase 3 lobby work.

### Phase 3 — Lobby HTTP surface

Bring the SPA online. Each Leptos screen replaces its baked-in sample
data with a real REST call.

#### 3.1 Auth routes

| Method | Path                                       | Body / Query                          | Effect                                                |
|--------|--------------------------------------------|---------------------------------------|-------------------------------------------------------|
| POST   | `/api/v1/auth/email/start` ✅              | `{email}`                             | Mint magic-link, hand to `Mailer`                     |
| GET    | `/api/v1/auth/email/verify` ✅             | `?token=…`                            | Validate, find-or-create account, set session cookie  |
| POST   | `/api/v1/auth/pubkey/challenge` ✅         | `{pubkey}` (32-byte hex)              | Issue a single-use 32-byte nonce (120s TTL) to sign   |
| POST   | `/api/v1/auth/pubkey/verify` ✅            | `{pubkey, signature, email?}`         | Verify Ed25519 sig over the nonce, find-or-create account keyed on the pubkey identity, link `email` as secondary, set session cookie |
| GET    | `/api/v1/auth/oidc/{provider}/start`       | —                                     | 302 to provider authorize URL (PKCE state in cookie)  |
| GET    | `/api/v1/auth/oidc/{provider}/callback`    | `?code&state`                         | Code exchange, verify ID token, link identity         |
| POST   | `/api/v1/auth/oidc/custom/start`           | `{issuer_url}`                        | Discovery + 302 like above                            |
| POST   | `/api/v1/auth/atproto/start`               | `{handle_or_did}`                     | Resolve PDS, 302 to PDS authorize                     |
| GET    | `/api/v1/auth/atproto/callback`            | `?code&state`                         | DPoP code exchange, link identity                     |
| POST   | `/api/v1/auth/signout` ✅                  | —                                     | Revoke current session                                |

#### 3.2 Account routes

| Method | Path                                      | Returns / Effect                                    |
|--------|-------------------------------------------|-----------------------------------------------------|
| GET    | `/api/v1/me` ✅                           | `MeView` for the authenticated session              |
| PATCH  | `/api/v1/me` ✅                           | Update `preferred_name`, `pronouns`, `bio`, `prefs` |
| POST   | `/api/v1/me/identities/email`             | Start linking-flow for a new email (magic-link)     |
| POST   | `/api/v1/me/identities/oidc/{provider}`   | Start linking-flow for an OIDC provider             |
| DELETE | `/api/v1/me/identities/{id}` ✅           | Unlink — refuse if it would orphan the account      |
| POST   | `/api/v1/me/identities/{id}/primary`      | Mark email identity as primary (one per account)    |
| POST   | `/api/v1/me/avatar`                       | Multipart upload (PNG/JPG, ≤2MB), pipeline TODO     |
| DELETE | `/api/v1/me` ✅                           | Delete account (cascades sessions + identities)     |

#### 3.3 SPA wiring

- [x] `components/api/{auth,me}.rs` bindings — `auth::email_start`
      / `auth::signout`, `me::get` / `me::patch` / `me::delete_me`.
      Single transport helper at `components/api/http.rs` (mirrors
      `open4x-server`'s shape) using `web_sys::Fetch` with
      `RequestCredentials::SameOrigin` so the browser-managed
      `lobby_session` cookie auto-attaches. `identities` bindings
      land alongside their server routes (Phase 2.3 / 2.4
      follow-up).
- [~] `Login` screen: email panel POSTs to `/auth/email/start` ✅
      (with EmailFlow state machine — Idle / Pending / Sent /
      Error — surfacing live "Magic link sent to <addr>. Check
      your inbox." or error text under the button; button
      disables to "Sending…" while the request is in flight).
      OIDC + atproto buttons still inert pending Phase 2.3 part 2
      and Phase 2.4.
- [ ] Magic-link landing page (`/auth/email/verify`) — server-rendered
      redirect to `/menu` (or to the SPA's Menu route via hash).
- [x] `Profile` screen: fetches `/api/v1/me` on mount via a
      `LocalResource` and seeds local `RwSignal`s for every editable
      field + every preference Toggle/Segmented. The "save" button
      diffs back through `me::patch`; SaveState shows
      pending / saved ✓ / error feedback. Avatar initial flips to
      the first character of `preferred_name`. Linked-identities
      list reads from `MeView.identities`; per-row "unlink" button
      opens a confirm-Popup that calls `DELETE /me/identities/{id}`
      and bumps the resource tick on success. Disabled (no popup)
      when the row is the only identity, mirroring the server-side
      "would_orphan_account" guard. "+ link another" still pending.
- [x] Session-cookie middleware on the axum side at
      `open4x-lobby/src/server/auth.rs`. Parses the `lobby_session`
      cookie via a hand-rolled extractor, calls
      `open4x_accounts::session::validate_session`, attaches the
      resulting `PlayerId` (and the raw `AuthCookie`) to request
      extensions when valid. The `RequireSession` extractor pulls
      it back; missing → 401 with structured
      `{"error":"no_session"}` body. Middleware is wired in
      `main.rs` ahead of the static-file fallback so /health and
      the SPA continue to work without auth. Three cookie-parser
      tests cover named-cookie / missing-cookie / empty-value
      cases.

**Done when**: the user can sign in with email or any of the four
pre-configured OIDC providers, see their account, link a second
identity, and sign out. Profile preferences round-trip via `/me`.

### Phase 4 — Games index + orchestration

The Menu currently shows hard-coded sample games. This phase puts a
games index behind `/api/v1/games` and wires the New-Game wizard to
actually create one.

#### 4.1 Schema ✅

`migrations/0002_games.sql` ships the two tables; soft-delete via
`deleted_at`; `status` enum (`your_turn` / `waiting` / `completed` /
`archived`). `GameStore` trait + `SqliteGameStore` impl at
`open4x-accounts/src/games.rs` with `create_game`, `get_game`,
`list_for_player`, `soft_delete`, `touch_last_played`,
`update_runtime_view`. 3 tests cover create→list, soft-delete
hiding, runtime-view round-trip.

```
games(game_id PRIMARY KEY, owner_player_id FK,
      name, leader, civ_id, difficulty, players_human, players_ai,
      map_type, map_size, seed, turn, era, score, status,
      server_url, server_token, last_played_at, created_at, deleted_at)
game_members(game_id FK, player_id FK, role, invited_at, joined_at)
PRIMARY KEY (game_id, player_id)
```

#### 4.2 Routes

| Method | Path                              | Effect                                                 |
|--------|-----------------------------------|--------------------------------------------------------|
| GET    | `/api/v1/games` ✅                | List games visible to the user (own + invited)         |
| POST   | `/api/v1/games` ✅                | New-game wizard submit — bootstrap & return `game_id`  |
| GET    | `/api/v1/games/{id}` ✅           | Single-game preview                                    |
| POST   | `/api/v1/games/{id}/notes` ✅     | Update markdown notes                                  |
| POST   | `/api/v1/games/{id}/invite`       | Invite by email / OpenID / atproto / `PlayerId`        |
| DELETE | `/api/v1/games/{id}` ✅           | Resign / archive / delete                              |
| POST   | `/api/v1/games/{id}/resume` (~)   | Returns server_url + token; 503 until orchestrator     |

#### 4.3 Orchestrator

- [x] Pick a model: **shared-server-multi-room** v1 (single
      configured `open4x-server` instance, `GameRoom`-per-id —
      already keyed that way). Process-per-game is a Phase 6
      enhancement.
- [~] Server-side change: cross-crate auth-key handshake is **v2**;
      v1 leans on `open4x-server`'s anonymous `/games/new` bearer
      tokens (the lobby remembers them per game and hands them
      back at Resume). Tracked as a follow-up Phase 4.3 task.
- [x] Lobby `POST /api/v1/games` translates the wizard's params
      into a call to `open4x-server`'s `POST /api/v1/games/new` via
      the new `server::orchestrator::bootstrap_game`. Stores the
      returned URL + token in `games.server_url` /
      `games.server_token`. Best-effort: orchestrator failure does
      not fail the lobby write; the row is created with empty
      fields and `Resume` returns 503 until a retry path lands.
- [x] `POST /api/v1/games/{id}/resume` returns `{url, token}` once
      both are populated; the browser uses them to authenticate
      against the in-game server. Smoke-tested end-to-end:
      orchestrator created `Rome Capital` in `open4x-server`,
      lobby Resume handed the token back, browser-side `GET
      /api/v1/cities` against the game server returned the city.
      The 302-with-session-bridge-cookie variant from the original
      plan is reserved for a Phase 5 polish pass.

#### 4.4 SPA wiring

- [x] `OngoingGames` reads `/api/v1/games`, renders real tiles with
      MiniMap thumbnails seeded from the world seed (`g.seed.bytes()`
      hashed). Loading + empty-state copy. Crumbs counter
      (`<n> games · <m> awaiting you`) live from the response.
- [x] Filter chips functional (`your_turn` / `waiting` / `completed`
      / `multiplayer` / `all`) — RwSignal-backed; active chip flips
      class. Filtering happens client-side over the loaded list;
      filter / sort / search now round-trip through the URL via
      `history.replaceState`, so back button + reload restore the
      user's view.
- [x] Search box filters client-side over `name` / `leader` /
      `civ_id` (case-insensitive substring match). "Notes" is
      reserved until the per-game notes route ships.
- [x] Sort dropdown — click-trigger Popup with four options
      (recent ↓ / oldest / score ↓ / turn ↓). Active option flagged
      with ✓; chip label reflects the selection. Sorting is
      client-side over the filtered list: Recent uses
      `last_played_at` falling back to `created_at`; Score breaks
      ties with `turn`; Turn / Oldest are straight column sorts.
- [x] "Resume" CTA wired in `screens/ongoing.rs`: clicking the
      tile's resume button posts `/api/v1/games/{id}/resume` via a
      new `components::api::games::resume` binding and navigates
      the browser to `<server_url>/?token=<server_token>`. The
      button is disabled when `server_url` is empty (orchestrator
      down at create-time). The in-game SPA needs a small change
      to read the `?token=` query param instead of bootstrapping
      its own anonymous bearer — tracked as the next commit.
- [x] `+ New game` wizard's "⌬ Generate world" → `POST /api/v1/games`
      with the **user's actual selections**. Phase 4.4 hoist:
      every per-step `RwSignal` lives on a `WizardState` struct
      provided through context. `WizardState::to_create_body()`
      builds the request from current values; the Review summary
      panel renders live from the same struct (`map · seed ·
      world · civilization · difficulty · victory · dynamics ·
      players · turn mode`). Pending / error feedback under the
      button unchanged.
- [x] Notes popup: markdown textarea, persisted via
      `POST /api/v1/games/{id}/notes`. Schema landed in
      `0003_game_notes.sql` (notes TEXT NOT NULL DEFAULT ''),
      `GameStore::set_notes` + `GameView.notes` exposed, owner-only
      route caps at 16 KiB and 403s cross-account. Tile button
      seeds the textarea from the loaded value; Save shows
      pending / saved ✓ / error feedback and bumps the list-tick
      so the next reload reflects the persisted value.
- [~] Per-tile `···` menu: click-trigger Popup with five rows.
      Wired today: Copy game ID (clipboard), View summary
      (flips the popup body to a kv table — name / id / leader+civ
      / map / seed / difficulty / turn / era / score / status /
      players / created / last played, with a `← back` row
      footer), Resign / delete (DELETE + tick refresh). Visible-
      but-inert: Share invite link, Archive (need their underlying
      surfaces — invite mint + status column).

**Done when**: a logged-in user can run the wizard end-to-end and find
themselves in a freshly-created `open4x-server` `GameRoom` they own,
return to the lobby, see it in their list, and resume it on a different
device after signing in there.

### Phase 5 — Polish

User-visible quality once the platform basics are wired.

- [x] **Avatar pipeline** — `POST /api/v1/me/avatar` (multipart).
      4 MiB cap on the upload, image-rs decode (PNG/JPEG only),
      `thumbnail_exact(256, 256)`, atomic write to
      `<data_dir>/avatars/<player_id_hex>.png`. `MeView` gains
      `avatar_url`; main.rs `ServeDir`s `/avatars/` so the SPA's
      `<img src=avatar_url>` reads back without auth. Profile
      gains a hidden file input + the existing "change" Btn now
      triggers it; cache-busted `?t=<now>` query forces refresh
      after a re-upload. Audit row carries `avatar:<bytes>`
      detail.
- [x] **Show invite QR popup** — `components/qr.rs::qr_svg` builds
      a self-contained SVG (one `<rect>` per dark module + a 1-
      module quiet zone) using the `qrcode` crate. Profile's
      "▦ Show invite QR" button is now a click-trigger Popup
      that renders the player_id as a 220 px QR with the hex
      label below. Copy player-ID action wired to
      `navigator.clipboard.write_text` on the same row.
- [x] **Friends screen** — `screens/friends.rs` now wires the
      design's chrome to live `/api/v1/friends` data: list,
      Friends / incoming-requests / outgoing-requests sections,
      Add-friend by email / atproto handle / OpenID URL /
      16-hex PlayerId, accept / decline / cancel / unfriend
      buttons. New schema in `0006_friends.sql` (single
      directed row, status ∈ pending|accepted|blocked, FK
      CASCADE on both sides) + `FriendsStore` in
      `open4x-accounts`. Identity-search via `POST /api/v1/
      friends/search` resolves the broader query shapes server-
      side, gated by the target's `discoverable_by_id`
      preference.
- [x] **Presets screen** — `0007_presets.sql` adds the
      `presets` table (id ULID, player_id FK CASCADE, name,
      body_json, timestamps). `PresetsStore` in
      `open4x-accounts` validates name + body-is-JSON +
      size caps. Lobby exposes GET / POST /api/v1/presets and
      DELETE /api/v1/presets/{id}. SPA screen lists "My
      presets" with per-row Delete; "↑ import JSON…" toggle
      reveals a name + textarea form + Save that persists.
      Load-from-built-in is still a follow-up; the wizard-tab
      "+ Save current" ergonomic shortcut landed in NewGame's
      shared footer (`SavePreset` component — serialises the live
      `WizardState` and POSTs to `/api/v1/presets`).
- [x] **Docs screen** — `screens/docs.rs` ships a quick-links
      panel pointing at `/book/`, the accounts-and-login roadmap,
      and the web-client REST reference. The lobby binary mounts
      `tower_http::ServeDir` at `/book/` (overridable via
      `OPEN4X_LOBBY_BOOK_DIR`, default `./book/book`); the screen's
      status note tells contributors to run `mdbook build book/`
      once to populate the directory.
- [x] **Email verification flow** — `POST /api/v1/me/identities/
      {id}/verify-start` mints + mails a fresh magic link for an
      unverified email row. The existing `/auth/email/verify`
      route now calls `AccountStore::mark_email_verified(&email)`
      after every successful sign-in, so any consumed magic link
      flips its identity's `verified` column to true. Profile
      screen renders a "verify" Btn on email rows where
      `verified=false`. Audit rows for the new mint carry detail
      `verify:<identity_id>` so ops can distinguish verify-start
      mints from sign-in mints.
- [x] **Sign-in feedback states** — `EmailFlow::Error` is now a
      discriminated `EmailFlowError` enum (EmptyEmail /
      RateLimited / ServerBusy / Network / Other) populated
      from `ApiError.status`. Each variant returns category-
      specific copy; retryable variants render a "↻ Try again"
      Btn that re-submits without forcing the user to re-type
      their email. New `AuthEmailStartFailed` audit kind logs
      mint_failed / mail_failed paths so ops can split server-
      side breakage from token-validation failures (the
      `SignInFailed` row now stays scoped to /verify).
- [x] **Real game tile thumbnails** — `GET /api/v1/games/{id}/
      thumbnail` (lobby-side proxy) Bearer-authenticates against
      `<server_url>/api/v1/world/snapshot` server-side and reduces
      the response to a `{width, height, cells:[{q,r,terrain}]}`
      grid the SPA can render. Browser stays cookie-authed only;
      `server_token` never crosses the wire to the client. SPA
      ships an in-session `ThumbnailCache` (provided once at App
      root) keyed by `game_id`; `ensure_fetched` is idempotent so
      filter / sort flips don't re-hit the proxy. `MiniMap` gains
      an optional `cells` prop; when `Some` it renders per-tile
      `<rect>`s in the 100×64 viewBox using a substring-matched
      terrain → CSS-class mapping. Smoke verified end-to-end
      against a real game (60×38 tiny map, 17 explored cells).
- [x] **Tile thumbnails reflect ownership** — the thumbnail
      proxy now passes through `fog`, `owned_by_me` (three-state),
      `city`, and `capital` flags per cell. New `cell_class`
      composer layers them: water > city/capital > ownership
      (mine / foreign / unowned) > fog. New `.svg-map` CSS rules
      (`.land-foreign`, `.city`, `.city-capital`, `.fog`) tint
      accordingly. Captured cities show as accent-filled dots,
      foreign territory dims, fog-of-war tiles render at 0.35
      opacity.
- [x] **i18n hooks** — `components/i18n.rs` ships a `tr(Key)` /
      `tr_in(locale, key)` shim. Match-on-enum keeps the
      compiler-enforced exhaustiveness; no library (wasm size,
      bundle-vs-fetch tradeoffs documented in
      `book/src/multiplayer/i18n.md`). `Locale::En` is the only
      wired locale today; Login is the worked example for call
      sites. Adding a second language is a documented 4-step
      recipe (Locale variant → context provider → `tr_in` arm →
      sibling `de(key)` fn).

### Phase 6 — Self-host + ops

Everything that's not a feature but is required to run this in
production.

- [x] **Single-binary deploy** — `rust-embed` snapshots
      `open4x-lobby/dist/` and `book/book/` into the binary.
      `OPEN4X_LOBBY_STATIC_DIR` (when set) keeps the dev
      `trunk serve` flow working; without it the binary serves
      embedded assets straight off its own `.rodata`. Binary
      grows from ~10 MB to ~13 MB. Migrations were already
      embedded via `sqlx::migrate!`. Deploys: `open4x-lobby` +
      `OPEN4X_LOBBY_DATA_DIR`.
- [ ] **Postgres opt-in** — connection string via env, sqlx feature
      flag.
- [x] **SMTP config** — env-driven (`SMTP_HOST`, `SMTP_PORT`,
      `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM`). LogMailer remains
      the default in dev; SmtpMailer engages when the four
      required vars are all set. See Phase 2.2 mailer entry above.
- [x] **Rate limiting** — per-email (5 mints / 5 min) and per-IP
      (20 mints / 5 min) throttles on
      `POST /api/v1/auth/email/start`, both backed by the audit
      log via `recent_count_by_kind_and_{detail,ip}`. Per-IP cap
      is higher because a household / NAT can legitimately request
      links for several addresses. `OPEN4X_LOBBY_TRUSTED_PROXIES`
      (CIDR list) gates X-Forwarded-For; when the peer falls inside
      a trusted CIDR, `client_ip` returns the rightmost untrusted
      hop in `X-Forwarded-For` instead of the proxy. Returns
      `429 rate_limited` + `Retry-After: 60` with a structured
      body distinguishing the two limits. OIDC-callback-per-state-
      cookie limit lands with the OIDC handler in Phase 2.3 part 2.
- [x] **Audit log** — `0004_audit_events.sql` adds an
      append-only `audit_events` table (id ULID + ts + kind +
      player_id + ip + detail). `open4x-accounts/src/audit.rs`
      ships `AuditEventKind` (8 variants), `NewAuditEvent` input
      shape, `AuditStore` async trait + `SqliteAuditStore` impl
      with `record` and `list_recent(limit)`. Wired from the lobby:
      magic-link mint, sign-in (with player_id), sign-in-failed
      (with `Reused` / `Expired` / `BadSignature` / `Malformed`
      detail), sign-out, account delete, new game created. CLI
      dump subcommand still pending. Smoke-tested:
      `magic_link_mint` → `sign_in` → `sign_out` → `sign_in_failed`
      land in order with the right `player_id` / `detail`.
- [x] **Account deletion (GDPR)** — `DELETE /api/v1/me` and the
      `open4x-accounts delete-account` CLI both invoke the
      schema cascade: sessions + identities + games (via the
      `owner_player_id ON DELETE CASCADE` from `0002_games.sql`)
      all disappear with the account row. Foreign
      `game_members` rows (where the deleted player was a member
      of someone *else's* game) are anonymised via migration
      0005: a sentinel `0000000000000000` account is reserved at
      migrate-time, and `delete_account` runs in a transaction
      that `UPDATE OR IGNORE`s those rows to the sentinel before
      the cascade fires. Host's roster keeps a `(anonymous)`
      placeholder. Unit test (~30 lines) covers the foreign-
      membership path against a real on-disk SqliteAccountStore.
- [x] **Process-per-game orchestrator** — `OPEN4X_LOBBY_PER_GAME=1`
      flips `AppState.deploy_mode` to PerGame. New
      `server::process::ProcessOrchestrator` allocates a free port
      from `OPEN4X_LOBBY_PORT_RANGE` (default 4001-4100),
      `Command::spawn`s the binary set by
      `OPEN4X_LOBBY_GAME_BINARY` with `PORT` + `OPEN4X_DATA_DIR`
      env, polls `/health` with exponential backoff up to
      `OPEN4X_LOBBY_HEALTH_TIMEOUT_S`, then delegates to the
      existing `bootstrap_game` on the new URL. Children are held
      in an Arc registry keyed by `remote_game_id` and use
      tokio's `kill_on_drop(true)` so a lobby kill takes them
      down with it. Smoke verified: 2 games → 2 distinct ports
      4501/4502, both children visible as ppid=lobby; resume
      returns the spawned URL+token; SIGTERM the lobby and both
      children die within 1.5s with no orphans.
- [x] **Reverse-proxy story** — `book/src/multiplayer/reverse-proxy.md`
      ships worked Caddy + nginx configs for both deploy
      topologies (shared-server-multi-room and process-per-game),
      including X-Forwarded-For + trusted-proxies setup, per-game
      subdomain routing, path-prefix routing with a regex
      pinned to the configured `OPEN4X_LOBBY_PORT_RANGE`, and
      WebSocket upgrade support for `/ws`. Paired with a small
      code change: per-game mode now substitutes `{port}` in
      `OPEN4X_LOBBY_PUBLIC_GAME_URL_TEMPLATE` so the row-and-
      browser URL is the public form (e.g.
      `https://g-4501.example.com`) rather than the loopback URL
      it spawns the child on.
- [x] **Backup & restore** — `open4x-accounts db-dump --out
      <path>` runs SQLite's `VACUUM INTO` (online-safe; refuses
      to overwrite). `db-restore --from <path> [--force]`
      validates the source via `PRAGMA integrity_check`, renames
      the live db to `<db>.bak.<unix-ts>` (skip with `--force`),
      then `fs::copy`s the snapshot in place. Smoke verified the
      full roundtrip: PATCH /me → dump → mutate → restore →
      original value reads back. Operators are expected to stop
      the lobby before restoring; the CLI does not enforce that
      because there's no portable way to detect a live process
      holding the file (sqlite locking is advisory on Linux).
- [x] **CLI: `prune-sessions`** — `open4x-accounts prune-sessions
      --days N` (default 30) deletes any session row that is
      revoked OR has expired more than N days ago. Prints the
      affected-row count for cron / scheduled-task wiring.
- [x] **CLI: `delete-account`** — GDPR cascade for a single
      player. Accepts the canonical hex display
      (`0xAAAA·BBBB·CCCC·DDDD`) or a bare 16-char hex string;
      writes an `account_deleted` audit row BEFORE the cascade
      so it survives. Verified: cascades sessions + identities to
      0 in one shot.
- [x] **Health endpoint pings the DB** — `GET /health` runs
      `SELECT 1` against the sqlite pool. Returns 200 `ok` on
      success, 503 `db_unreachable` otherwise so a load balancer
      can pull the instance out of rotation.
- [x] **Graceful shutdown** — `axum::serve(...).with_graceful_
      shutdown(shutdown_signal)` waits for in-flight requests to
      drain on SIGINT / SIGTERM. Logs the signal it received +
      a final 'exited cleanly' line for systemd / podman supervisors.

---

## 5. Cross-cutting decisions to lock in

These don't fit cleanly into one phase but need to be settled early.

- **Identity uniqueness.** `(kind, primary_key)` is the unique tuple.
  An email cannot be linked to two accounts. An OIDC `(issuer, sub)`
  cannot be linked to two accounts. atproto `did` cannot be linked to
  two accounts. **Implication**: signing in with an already-linked
  identity always lands you in the same account — no merging UI in v1.
- **Account merging — out of scope.** If a user has two accounts (one
  from email, one from GitHub) and wants them merged, that's a manual
  operator action in v1.
- **Primary email.** Exactly one `Identity::Email` per account may have
  `primary = true`. Used as the `from`/`to` for system mail and as the
  default Gravatar source. The "Set as primary" button enforces.
- **Cookies vs. bearer tokens.** Lobby uses **httpOnly secure
  cookies** for browser sessions. The in-game server uses **bearer
  tokens** for its REST surface. The Resume flow bridges by minting a
  short-lived bearer signed by the same key.
- **CSRF.** All state-changing routes require a double-submit cookie
  token. Magic-link verify is GET-with-query (intentional — links must
  work from any client) but consumes a single-use server-side nonce.
- **Tile coordinates / wire types.** Reuse `open4x-server`'s wire types
  where possible; do not duplicate `WorldSnapshot` etc. The lobby's
  game-creation flow round-trips the wizard's params straight into the
  server's existing `POST /api/v1/games/new`.

---

## 6. What this plan deliberately does NOT do

- **No multiplayer protocol changes.** The in-game `/ws` stays
  untouched. Real-time multiplayer is out of scope for accounts/login.
- **No payment / premium tier.** Open source, AGPL.
- **No mobile apps.** The SPA is desktop-first, mobile-okay; native
  apps are not on this roadmap.
- **No federation between Open4X instances.** Each lobby is its own
  trust boundary. Cross-instance identity (e.g. atproto-as-bridge) is
  a future thing.
- **No anti-abuse beyond rate limiting.** No CAPTCHA, no fraud
  scoring. Self-hosted instances pick their own posture.

---

## 7. Open questions

- **Tweaks panel scope.** Density is the obvious one to wire. Color
  scheme (paper / ink / auto) needs a CSS-vars dark variant before it's
  meaningful. Defer ink mode to Phase 5?
- **In-game server token format.** HMAC-shared-key (simple, requires
  config sync) vs. HTTP introspection against
  `open4x-accounts` (resilient, but adds a hop). v1 = HMAC; v2 may
  switch.
- **atproto OAuth maturity.** The spec is still moving. We may need to
  ship a "sign in via app password" fallback if OAuth churn breaks the
  flow during Phase 2.

---

## 8. Phasing summary

| Phase | Theme                              | Done when                                                                   | Blocks                       |
|-------|------------------------------------|-----------------------------------------------------------------------------|------------------------------|
| 0     | Scaffolding ✅                     | Workspace split + paper SPA renders all screens (some w/ placeholder data)  | —                            |
| 1     | Visual completeness ✅             | Pixel-perfect against design; popups + slider work; remaining wizard steps  | none — pure UI               |
| 2     | `open4x-accounts` substrate ◐      | Magic-link, OIDC client, atproto resolver, session tokens, sqlite store     | Phase 3                      |
| 3     | Lobby HTTP + auth wiring ✅        | Sign in via 3 methods, link identities, profile round-trip via `/me`        | Phase 4                      |
| 4     | Games index + orchestration ◐      | New-game wizard creates a real `GameRoom`, ongoing list reflects reality    | Phase 5                      |
| 5     | Polish ◐                           | Avatar, QR, friends, presets, real thumbnails, email verify                 | Phase 6                      |
| 6     | Self-host + ops ◐                  | Single-binary deploy, SMTP, rate limit, audit log, process-per-game         | —                            |

---

## 9. Changelog

Running record of work performed against this plan, newest at top.

### Phase 6 — Self-host + ops (2026-05-10, in progress)

- `90841e3c` — feat(open4x-lobby): X-Forwarded-For reader for trusted
  proxies. `OPEN4X_LOBBY_TRUSTED_PROXIES` (CIDR list, ipnet);
  `client_ip` walks XFF right-to-left, returns the first
  untrusted hop when the peer is itself trusted. Per-IP throttle
  + audit log now record the real client IP behind a
  reverse-proxied deploy.
- `375f2657` — feat(open4x-lobby): single-binary deploy via
  rust-embed. SPA + book bundles snapshot into the binary at
  compile time. `OPEN4X_LOBBY_STATIC_DIR` / `_BOOK_DIR` (when
  set) keep the dev `trunk serve` flow; without them the binary
  serves embedded assets straight off `.rodata`. ~10 → ~13 MB.
- `d3393750` — feat(open4x-{accounts,lobby}): SMTP Mailer impl
  with lettre. SmtpConfig::from_env reads SMTP_HOST/PORT/USER/
  PASS/FROM; AppState prefers SMTP when configured, falls back
  to LogMailer otherwise. Lettre AsyncSmtpTransport with rustls
  (no native-tls dep).
- `988578f8` — feat(open4x-accounts): CLI prune-sessions +
  delete-account subcommands. prune-sessions --days N kills
  revoked or expired-N+ days session rows for cron wiring.
  delete-account --player-id <hex> runs the GDPR cascade
  (accepts dot-grouped or bare 16-char hex), writes the
  audit row BEFORE the delete so it survives. Smoke
  cascades sessions + identities to 0.
- `2e63034e` — feat(open4x-lobby): process-per-game
  orchestrator. `OPEN4X_LOBBY_PER_GAME=1` flips boot into a
  mode that `Command::spawn`s a fresh `open4x-server` per game
  on a port allocated from `OPEN4X_LOBBY_PORT_RANGE`. Children
  are held in an Arc registry with `kill_on_drop(true)` so a
  lobby exit takes them down. Smoke verified end-to-end: two
  concurrent games landed on 4501 + 4502, kill -TERM cleaned
  both up with no orphans. Default deploy mode unchanged.
- `fcd59217` — feat(open4x-accounts): db-dump + db-restore CLI
  subcommands. db-dump runs `VACUUM INTO` (refuses to overwrite
  an existing target; online-safe). db-restore validates the
  source via `PRAGMA integrity_check`, rotates the live db to
  `<db>.bak.<unix-ts>` (skip with --force), and `fs::copy`s the
  snapshot in place. Smoke roundtrip verified (mutate→dump→
  mutate→restore→original reads back) plus three validation
  paths (existing target / garbage source / missing source).
- `486a1f18` — feat(open4x-lobby): public-URL template for the
  per-game orchestrator. New
  `OPEN4X_LOBBY_PUBLIC_GAME_URL_TEMPLATE` substitutes `{port}`
  with the per-game allocated port at spawn time; row /
  browser see the public URL while the lobby keeps talking to
  the child over loopback. Boot banner surfaces the template.
  Smoke verified server_url=https://g-4601.example.com on a
  fresh game. 1 unit test covers None / subdomain / path /
  literal templates.
- `37942a34` — feat(open4x-accounts): anonymise foreign
  game-membership on delete. New migration `0005_anon_sentinel
  .sql` reserves a `0000000000000000` "(anonymous)" account
  row. `SqliteAccountStore::delete_account` now runs in a
  transaction that `UPDATE OR IGNORE`s `game_members.player_id`
  to the sentinel for rows where the player was a member of
  *someone else's* game, before the FK cascade tears down
  the deleted player's own data. Host rosters survive an
  invitee's GDPR delete. Bumps Phase 6 GDPR row from `~` to
  `✅`. New unit test covers the round-trip end-to-end against
  real Sqlite.
- `25ea95a8` — feat(open4x-{accounts,lobby}): email-identity
  verify flow. AccountStore::mark_email_verified flips the row
  to verified=true; the existing /auth/email/verify endpoint
  calls it after every successful sign-in (any consumed magic
  link verifies the address). New
  POST /api/v1/me/identities/{id}/verify-start mints + mails
  a fresh link for an unverified row. Profile renders a
  "verify" button on unverified email rows. Audit detail
  `verify:<identity_id>` distinguishes verify-start mints
  from sign-in mints. Smoke verified the full roundtrip.
- `77072665` — feat(open4x-{accounts,lobby}): hardened
  sign-in feedback states. `EmailFlow::Error` is now a
  discriminated `EmailFlowError` enum populated from
  `ApiError.status`; each variant has category-specific
  user-facing copy and retryable variants render a Try-again
  Btn that re-submits without forcing the user to re-type.
  New `AuthEmailStartFailed` audit kind logs mint_failed /
  mail_failed with prefixed detail; smoke against an
  unreachable SMTP host wrote the row with
  `mail_failed:transport error: Connection refused`.
- `ede8fb62` — feat(open4x-lobby): real game tile thumbnails.
  `GET /api/v1/games/{id}/thumbnail` proxies the bearer'd hop
  to `<server_url>/api/v1/world/snapshot` server-side, reducing
  the response to `{width, height, cells:[{q,r,terrain}]}` so
  `server_token` stays lobby-only. SPA caches per-game grids in
  an in-session RwSignal HashMap; MiniMap renders real per-tile
  rects when ready, falls back to the seeded blob layout while
  pending. End-to-end smoke verified against a 60×38 game.
- `06c8fc7c` — feat(open4x-lobby): thumbnail tiles reflect
  ownership + city + fog. ThumbCell gains `fog`, `owned_by_me`
  (3-state), `city`, `capital`. New `cell_class` composer
  layers water > city/capital > ownership > fog. CSS adds
  `.land-foreign` / `.city` / `.city-capital` / `.fog` rules
  built on the existing design vars. Smoke against a fresh
  game showed 1 capital + 7 foreign-owned + 10 unowned cells
  on a 17-cell explored ring.
- `60838da1` — feat(open4x-lobby): avatar upload pipeline.
  Multipart POST /api/v1/me/avatar with image-rs PNG/JPEG
  decode, `thumbnail_exact(256, 256)`, atomic write to
  `<data_dir>/avatars/<player_id_hex>.png`. ServeDir at
  `/avatars/` exposes the read path. MeView gains
  `avatar_url`; Profile renders `<img>` when present, falls
  back to the initial-letter circle. Cache-busted SPA-side
  with `?t=<Date.now()>`. Smoke verified the round-trip:
  uploaded PNG → 256x256 downscale → readback via /avatars/.
- `e71de7f1` — feat(open4x-lobby): i18n shim + Login as
  worked example. `tr(Key)` / `tr_in(locale, key)` routes
  every English literal through a match-on-enum lookup.
  `Locale::En` only today; `Key` enum's compile-time
  exhaustiveness flags missing arms when a second language
  ships. Login screen ports its 17+ literals to `Key::Login*`
  variants. New `book/src/multiplayer/i18n.md` documents the
  add-a-locale recipe. No runtime library — wasm budget +
  compile-time guarantees both win.
- `8c7f1fcd` — feat(open4x-{accounts,lobby}): friends schema
  + routes + screen. New `0006_friends.sql` (single directed
  row, status pending|accepted|blocked, FK CASCADE), new
  `FriendsStore` in open4x-accounts with a unit test covering
  the request → accept → unfriend round-trip + rejection paths
  (self-friend, duplicate). Lobby gains GET /friends, POST
  /friends/request, POST /friends/{id}/accept, DELETE
  /friends/{id}; Friends screen wires Add-friend, accept,
  decline, cancel, unfriend. Smoke verified the full flow
  end-to-end against two real signed-in accounts.
- `1b2623c3` — feat(open4x-{accounts,lobby}): friends
  identity-search route. `SqliteAccountStore::search_for_friend`
  resolves either a 16-hex PlayerId or an identity primary_key
  (email/handle/OpenID URL) to one or more `SearchHit`s,
  gated by `Preferences.discoverable_by_id` for ALL paths.
  Lobby exposes `POST /api/v1/friends/search`; Friends screen
  resolves non-hex inputs through it before posting
  /friends/request. Smoke verified email + hex resolution and
  the opt-out gating both paths.
- `38c1e50e` — feat(open4x-{accounts,lobby}): presets schema
  + routes + screen. New `0007_presets.sql` table; new
  `PresetsStore` with create-list-delete + a unit test
  covering all three rejection paths (empty name, oversize,
  non-JSON body). Lobby gains GET/POST /api/v1/presets +
  DELETE /api/v1/presets/{id}. Presets screen lists user
  rows with delete + an Import-JSON drawer that saves a
  name+body pair. Smoke verified create → list → invalid
  rejections → delete → empty round-trip end-to-end.

- `3e7bf7d4` — feat(open4x-{accounts,lobby}): per-email magic-link
  rate limit. New `AuditStore::recent_count_by_kind_and_detail`
  backs a 5-mints-per-5-min cap on `POST /auth/email/start`.
  Returns 429 + `Retry-After: 60` header + structured body when
  over.
- `596af021` — feat(open4x-{accounts,lobby}): per-IP throttle on
  /auth/email/start. Mirror of the per-email cap at 20 mints / 5
  min via `recent_count_by_kind_and_ip`. ConnectInfo<SocketAddr>
  reads the peer; magic_link_mint audit rows now carry the IP.
  X-Forwarded-For reader for production-behind-proxy is a
  follow-up.
- `89d4fdfb` — feat(open4x-{accounts,lobby}): DELETE
  /api/v1/me/identities/{id}. AccountStore gains
  `list_identities_with_ids` so the wire surfaces stable row
  ids; the /me handler refuses unlinks that would orphan the
  account; audit log records `identity_unlinked`.
- `b01e98fb` — feat(open4x-lobby): wire Profile per-row identity
  unlink button. Confirm-popup posts the new
  `components::api::me::unlink_identity` binding and bumps the
  resource tick. Disabled (no popup) for the only-identity case
  so it mirrors the server-side guard.
- `573b9b55` — feat(open4x-lobby): real /health + graceful
  shutdown. /health pings the DB (200 ok / 503 db_unreachable);
  axum graceful_shutdown drains in-flight requests on
  SIGINT / SIGTERM and logs the signal received.
- `a08a1179` — feat(open4x-{accounts,lobby}): DELETE /me/identities/{id}.
  AccountStore gains `list_identities_with_ids` so the wire surfaces
  stable row ids; the /me handler refuses unlinks that would
  orphan the account; audit log records `identity_unlinked`.

- `0da19288` — feat(open4x-{accounts,lobby}): append-only audit log.
  `0004_audit_events.sql` adds the table; `audit.rs` ships
  `AuditEventKind` (8 variants) + `AuditStore` + `SqliteAuditStore`.
  Lobby handlers write `magic_link_mint` / `sign_in` /
  `sign_in_failed` (with MagicLinkError variant in detail) /
  `sign_out` / `account_deleted` / `new_game_created`. Best-effort
  writes — never fail user flows.
- `4daa2fd1` — feat(open4x-accounts): CLI binary `open4x-accounts`
  with `dump-audit --limit <n>` subcommand. clap-driven, TSV
  output with hex PlayerId display, sanitised tabs/newlines.
- `adf79a0a` — feat(open4x-lobby): static mdBook serve under
  `/book/`. ServeDir mount via `OPEN4X_LOBBY_BOOK_DIR` (default
  `./book/book`); Docs screen status note updated.

### Phase 5 — Polish (2026-05-10, in progress)

- `4628be93` — feat(open4x-lobby): Show invite QR popup. New
  `components/qr.rs::qr_svg` builds a self-contained SVG (single
  rect per dark module + 1-module quiet zone) using the qrcode
  crate (no default features). Profile's "▦ Show invite QR"
  button is now a click-trigger Popup; "⎘ Copy player ID"
  wires `navigator.clipboard`.
- `91c1acd1` — feat(open4x-lobby): wire MenuTab Friends / Presets /
  Docs. The MORE-group sidebar items now actually navigate; three
  new screen modules (`friends.rs` / `presets.rs` / `docs.rs`)
  render the design's chrome with empty-state copy. Real
  persistence + identity-search + mdbook-static-serve are deferred.
- `58d653ef` — feat(open4x-lobby): user-card popup with sign-out.
  The sidebar's user-card is now a click-trigger Popup with three
  rows: Profile & settings (jumps tabs), Copy player ID (via
  Profile), Sign out (fires the on_signout callback). The
  Profile-quick-actions sign-out path stays as the alternate
  surface.

### Phase 4 — Games index polish (2026-05-10, in progress)

- `bf726b35` — feat(open4x-{accounts,lobby}): per-game notes column +
  popup. `0003_game_notes.sql` adds `notes TEXT NOT NULL DEFAULT ''`,
  `GameStore::set_notes` lands, `POST /api/v1/games/{id}/notes`
  enforces ownership + 16 KiB cap, tile "📝 Notes" button is now
  a click-trigger Popup with a textarea + Save button + saved/error
  feedback that bumps the list-tick.
- `f14a94b4` — feat(open4x-lobby): URL-backed filter / sort / search
  state on OngoingGames. `Filter::slug/parse` + `Sort::slug/parse`,
  `read_query_state` seeds at mount, an Effect pushes
  `history.replaceState` on every change, defaults are omitted to
  keep the URL short, percent-codec helpers handle the search
  string.
- `ad8ff73b` — feat(open4x-lobby): View summary on the tile · · ·
  menu. `tile_menu_popup` now drives the popup body through a
  `SummaryMode {Menu, Summary}` signal: View summary flips to a kv
  table of every GameView field; a `← back` row in the popup
  footer returns. Renderer split into render_menu_rows +
  render_summary_kv helpers for borrow-checker sanity.
- `3a9abd9b` — feat(open4x-lobby): hoist NewGame wizard state into
  a `WizardState` struct provided through context. Generate world
  POST + Review summary now both read live values; replaces the
  static REVIEW_ROWS table.
- `a659eb08` — feat(open4x-lobby): per-tile menu (Copy game ID +
  Resign). Click-trigger Popup with five rows; Copy uses
  `navigator.clipboard`, Resign hits `DELETE /games/{id}` + bumps
  a list-refresh tick. PopupList stays for inert call sites.
- `c9696e0a` — feat(open4x-lobby): sort dropdown (Recent / Oldest /
  Score / Turn) on OngoingGames. Click-trigger Popup; active
  option flagged with ✓; sort applied client-side over the
  filtered list.

### Phase 4 — Games index + orchestration (2026-05-10, in progress)

- `13a6440f` — feat(open4x-lobby): orchestrator bootstrap (Phase 4.3
  v1). New `server::orchestrator::bootstrap_game` calls
  `open4x-server`'s `POST /api/v1/games/new` with the wizard
  params, captures the URL + token, stores them on the lobby
  row. Best-effort: orchestrator failure does not fail the lobby
  write. `OPEN4X_GAME_SERVER_URL` env (default
  `http://localhost:3001`) selects the target server. Cargo
  gains `reqwest` (rustls-tls + json, default features off) +
  `thiserror`. Smoke-tested end-to-end: lobby creates a real
  `GameRoom` in the in-game server, Resume returns the bearer,
  the bearer authenticates against `/api/v1/cities`.
- `2f7ba0bd` — feat(open4x-lobby): wire OngoingGames Resume CTA.
  New `components::api::games::resume` binding; the tile button
  navigates `window.location` to `<server_url>/?token=<token>`.
  Disabled when `server_url` is empty.
- `7ff2bd72` — feat(open4x-server): RestGamePage honors `?token=`
  query (lobby Resume). Bootstrap Effect prefers the URL bearer
  when present; anonymous `/games/new` fallback stays for guest
  play. `read_token_from_query` + `decode_uri_component` helpers
  added.
- `72b5f3d5` — feat(open4x-lobby): wire filter chips + search.
  `Filter` enum (All / YourTurn / Waiting / Completed /
  Multiplayer) RwSignal-bound; chip CSS class flips active. Search
  input filters case-insensitively over name / leader / civ_id.
  Empty-set messaging for both 'no games yet' and 'no games match
  the current filter'.

### Phase 4 — Games index + SPA wiring (2026-05-10, in progress)

- `dfd30b85` — feat(open4x-accounts): games index schema + GameStore
  (Phase 4.1). `0002_games.sql` adds `games` + `game_members`. New
  `GameStore` async trait + `SqliteGameStore` impl over a shared
  pool. `GameStatus` enum + `NewGame` input type + soft-delete
  semantics (`Forbidden` if the row exists but isn't yours,
  `NotFound` otherwise). 3 tests cover create→list, soft-delete,
  runtime-view round-trip.
- `b4172e78` — feat(open4x-lobby): games HTTP routes (Phase 4.2).
  `AppState.games: Arc<SqliteGameStore>` shares the existing pool.
  `GET /api/v1/games`, `POST /api/v1/games`, `GET /games/{id}`,
  `DELETE /games/{id}`, `POST /games/{id}/resume` (returns 503
  `orchestrator_not_ready` until Phase 4.3 fills `server_url` +
  `server_token`). `GameView` wire shape strips `server_token`.
- `4b4c1d82` — feat(open4x-lobby): wire OngoingGames + NewGame SPA
  to `/api/v1/games` (Phase 4.4 partial). `components::api::games`
  bindings; `screens/ongoing.rs` renders live rows with MiniMap
  seeded from `g.seed`; wizard "⌬ Generate world" CTA POSTs to
  `/api/v1/games` and routes back to Ongoing on success. Filter
  chips / search / sort / notes / per-tile menu and Resume CTA
  remain unwired pending Phase 4.3 + further wizard hoisting.

### Phase 3 — Lobby HTTP (2026-05-10, complete)

Driven by the `dfdcd4f5` cron tick. Conventional-commit chain:

- `lvlvptxx` — docs(roadmap): pivot to Phase 3 before completing the
  network half of OIDC and the atproto resolver. The session
  validator already exists, so wiring email → cookie → `/me`
  through the lobby gives the design a working sign-in loop while
  the deeper auth providers ship under their own ticks.
- `mkorpkkt` — feat(open4x-lobby): session-cookie middleware +
  `RequireSession` extractor. `AppState::boot` opens / migrates
  the sqlite db at `$OPEN4X_LOBBY_DATA_DIR/accounts.sqlite`, loads
  the magic-link signer key, and defaults to `LogMailer`.
  `session_layer` always runs; auth-required handlers pull the
  resolved `PlayerId` via the extractor (missing →
  `401 {error:"no_session"}`). Hand-rolled cookie parser with 3
  unit tests.
- `lnprrwmn` — feat(open4x-lobby): email magic-link auth route
  pair + signout. `POST /api/v1/auth/email/start { email }`
  validates, mints, records, mails. `GET /api/v1/auth/email/verify`
  consumes the nonce, finds-or-creates the account, mints a
  30-day session, sets the `lobby_session` cookie, redirects to
  `/`. `POST /api/v1/auth/signout` revokes + clears. Smoke-tested
  end-to-end.
- `zyvslvzk` — feat(open4x-{accounts,lobby}): `/api/v1/me` reads,
  writes, and delete. `AccountStore::get_by_player_id` added;
  `MeView` wire shape exposes the hex `PlayerId` (never the raw
  u64) plus the linked-identity list. `DELETE /me` cascades
  sessions + identities via the FK. Smoke-tested.
- `826e27c4` — feat(open4x-lobby): client API bindings + wire Login
  email panel. New `components/api/{http,auth,me}.rs` modules using
  `web_sys::Fetch` with `RequestCredentials::SameOrigin`. Login's
  `Send magic link →` button now posts to `/auth/email/start` with
  EmailFlow {Idle, Pending, Sent(addr), Error(msg)} feedback under
  the button.
- `f1f3b637` — feat(open4x-lobby): wire Profile screen to
  `GET / PATCH /me`. LocalResource on mount seeds eight RwSignals;
  Save button posts a full PatchMeBody with all fields + Preferences;
  identities list is data-driven; SaveState shows pending/saved/
  error feedback.
- `1b827809` — feat(open4x-lobby): bootstrap-on-mount + sign-out
  plumbing. App fires `me::get` once on mount and skips Landing
  when the session cookie is fresh. `on_signout: Callback<()>`
  wired through Profile + MenuShell calls `auth::signout` and
  resets to Landing.

### Phase 2 — `open4x-accounts` substrate (2026-05-10, partial)

- `kwtktvux` — feat(open4x-accounts): persistence layer (sqlite
  store + migrations). `AccountStore` trait + `SqliteAccountStore`
  + `MemAccountStore` test double. Four-table schema in
  `0001_initial.sql` (accounts · identities · sessions ·
  magic_link_nonces). `persistence` cargo feature +
  `postgres` opt-in.
- `lqpoutsu` — feat(open4x-accounts): `MagicLinkSigner` mint /
  verify (Phase 2.2). HMAC-SHA256 envelope, single-use nonce
  enforcement via `UPDATE-where-consumed_at-IS-NULL`,
  `from_env_or_path` key resolver. 6 tests.
- `ozlwxlus` — feat(open4x-accounts): pluggable `Mailer` trait +
  `LogMailer`. `mailer-smtp` cargo feature stub. 2 tests.
- `orplmxzv` — feat(open4x-accounts): session token mint /
  validate / revoke (Phase 2.5). Bearer `lobby_<base64url(48)>`
  hashed to SHA-256 hex in storage. `revoke_all_for_player` for
  sign-out-everywhere. 5 tests.
- `qsnnmnvs` — feat(open4x-accounts): OIDC config + PKCE + auth-URL
  builder (Phase 2.3 part 1). Deterministic, network-free half
  of the code-flow. Built-in factories for Google · GitLab ·
  Microsoft + Custom. 7 tests. Discovery + exchange + ID-token
  verify deferred to a follow-up commit.

### Phase 1 — Visual completeness (2026-05-10, complete)

Driven by the `dfdcd4f5` cron tick. Conventional-commit chain:

- `kpnxvpkm` — feat(open4x-lobby): port Slider primitive. Wraps
  `<input type=range>` with optional `Arc<dyn Fn(i32) -> String>`
  format callback; `RwSignal<i32>`-driven; `min` / `max` default to
  `0` / `100`. Source:
  `docs/open4x-landing/project/primitives.jsx`.
- `nrostsvx` — feat(open4x-lobby): PopupBody / PopupActions /
  PopupList. Pure layout wrappers matching the design's
  `.popup-body` / `.popup-actions[.right]` / `.popup-list` CSS.
  `PopupList` items modelled as a typed `PopupListItem` enum
  (`Row { icon, label, desc? }` | `Separator`) so menu definitions
  are checked at compile time.
- `quzzmluo` — feat(open4x-lobby): Gwern-style Popup component.
  `PopupProvider` mounts at the app root and owns one
  `RwSignal<Option<PopupState>, LocalStorage>`; each `Popup`
  wrapper captures its anchor's `DOMRect` and asks the provider to
  show. 180 ms hover-show / 140 ms hide-grace timers
  (`gloo-timers`). Click-trigger pins. Esc + click-outside dismiss
  pinned popups. Smart positioning prefers below the anchor with a
  viewport-clipping flip. `PopupState` carries an
  `Arc<dyn Fn() -> AnyView + 'static>` view-fn (Clone-cheap;
  `LocalStorage` signal because `AnyView: !Send`). Timers wrapped in
  `SendWrapper` so the context is `Send + Sync` for
  `provide_context`.
- `wnpynloo` — feat(open4x-lobby): migrate Landing + Login Trigger
  stubs to real Popups. Six call sites swept (3 landing footer +
  player-ID + 3 login panel headers). Trigger stub remains for the
  unmigrated screens (menu / newgame / profile).
- `stwyqnyu` — feat(open4x-lobby): port NewGame StepCiv. 8-card
  picker grid backed by a static `CIVS` table, hover popup with
  CivSheet body + footer actions, click-to-select with the active
  card flipping border + background to the accent.
- `pslzypqv` — feat(open4x-lobby): port NewGame StepRules. Two
  panels: difficulty / era / game-speed Segmenteds + six victory
  toggles seeded from a `VICTORY_CONDITIONS` table; world-dynamics
  Sliders for disasters / barbarians / city-states / AI aggression
  with categorical formatters; AI-personality Segmented. Help
  triggers are real `<Popup>` wrappers.
- `vmzkmour` — feat(open4x-lobby): port NewGame StepPlayers. 8-slot
  baseline (1 human-you, 1 open invite slot, 6 AI), invite Popup
  with email/OpenID/atproto/PlayerID input + recent-recipients
  chips, slot-management `PopupList` (Change civ / AI personality
  / Swap / Remove). Turn-mode panel with timer Segmented and
  simultaneous / private / cross-play Toggles. Drops the now-unused
  `StepPlaceholder`. `PanelHead`'s `right` slot was removed (its
  optional `Children` shape was double-wrapping under leptos's
  `#[prop(optional)]` macro); the one head that needed it inlines
  the head DOM.
- `kszpskql` — feat(open4x-lobby): port runtime Tweaks panel.
  Fixed-position card with a Segmented density picker bound to a
  parent-owned `RwSignal<String>`. App now reads density into the
  root `.app` element's `data-density` attr reactively (was a
  hard-coded literal). Sliders / color pickers / postMessage host
  protocol from the JSX original deliberately omitted.
- `vmtvmnwv` — refactor(open4x-lobby): final Trigger sweep.
  Migrates the bare `<span class="trigger" title="…">` sites in
  `screens/menu.rs` (server-status indicator) and
  `screens/newgame.rs::StepMap` (map type, map size) into real
  `<Popup>` wrappers; deletes the now-unused
  `components/popup_stub.rs` placeholder and its public re-export
  from `components/mod.rs`. Phase 1 done.

### Phase 0 — Scaffolding (2026-05-10)

- Workspace `[workspace.members]` updated to add `open4x-accounts` and
  `open4x-lobby`. Both build clean alongside the existing `libciv` /
  `open4x-server` / `open4x-cli` triumvirate.
- `open4x-accounts/src/lib.rs` ships the type skeleton: `PlayerId(u64)`
  with the dot-grouped hex `Display`, `Identity` enum (Email · OpenId ·
  Atproto), `Account` (preferred_name / pronouns / bio / identities /
  prefs), `Preferences` (density · color_scheme · keyboard_nav ·
  turn_notifications · discoverable_by_id), `MagicLinkToken` newtype.
  Two unit tests cover `PlayerId::display` and identity labels.
- `open4x-lobby/Cargo.toml` mirrors `open4x-server`'s ssr/csr feature
  gate. `[lib]` is `cdylib + rlib`, `[[bin]]` is gated on `ssr`.
- `open4x-lobby/index.html` pulls in IBM Plex Mono / Sans / Serif via
  Google Fonts and links `styles.css` through Trunk's `data-trunk`
  asset pipeline (emits a `styles-<hash>.css` on build).
- `open4x-lobby/styles.css` is the design's CSS copied verbatim from
  `docs/open4x-landing/project/hifi/styles.css` (798 lines, no
  modifications).
- `open4x-lobby/src/components/` ships Leptos ports of the JSX
  primitives: `Btn` (primary / accent / ghost / bare; xs / sm / lg /
  block), `Tag`, `Toggle` (`RwSignal<bool>`-driven), `Segmented`
  (with a `Segment {value, label}` row type), `Panel` + `PanelHead`,
  `MiniMap` (deterministic LCG matching the JSX), and `Trigger`
  (placeholder for the not-yet-ported Gwern popup).
- `open4x-lobby/src/screens/` ships Landing (full port — ASCII banner,
  serif headline, two CTAs, EMAIL · OPENID · ATPROTO triggers,
  footer), Login (full port — three stacked auth panels), MenuShell
  (sidebar + secondary nav), OngoingGames (tile grid with the design's
  sample data baked in), NewGame (5-step strip; Map step + Review step
  ported, Civ / Rules / Players steps render placeholder bodies),
  Profile (full port — avatar / quick actions / fields / linked
  identities / preferences).
- `open4x-lobby/src/app.rs` ships the `App` component: `app-bar`
  (brand · HI-FI pill · Landing/Login/Menu nav · online dot · `?` kbd
  hint) plus a `RwSignal<Screen>` driving which screen mounts below.
  `MenuShell` nests a `RwSignal<MenuTab>` for the in-menu sub-nav.
- `open4x-lobby/src/main.rs` is a small Axum binary that serves
  `dist/` via tower-http `ServeDir` and exposes `/health`. Defaults to
  port 3002; static dir comes from `OPEN4X_LOBBY_STATIC_DIR`.
- Smoke test: `trunk build --release --features csr
  --no-default-features` produces 709 KB wasm + 24 KB hashed CSS.
  `target/release/open4x-lobby` boots, serves the SPA at `/`, the wasm
  + css at their fingerprinted URLs, and `/health` returns `ok`.
- Known limitations carried into Phase 1: `Trigger` stubs the popup;
  Wizard's middle three steps are placeholders; OngoingGames + Profile
  use baked-in sample data; no auth, no orchestration.
