# Commit Scopes (SCOPES)

The commit **scope is mandatory** — `type(scope): subject`. It names the
area of the codebase a change touches, so history can be filtered by
subsystem. Prefer the most specific applicable scope. The allowed values
are the machine-readable list below, enforced by `.githooks/commit-msg`.

<!-- SCOPES:BEGIN -->
libciv
protocol
accounts
lobby
server
cli
client-js
client-web
sdk
xtask
auth
deps
docker
docs
book
hooks
ci
repo
infra
release
<!-- SCOPES:END -->

## What each scope means

**Crates** (a change scoped to one workspace member):
| scope | crate |
|-------|-------|
| `libciv` | `libciv/` — the game engine |
| `protocol` | `open4x-protocol/` — wire types |
| `accounts` | `open4x-accounts/` — identity/auth substrate |
| `lobby` | `open4x-lobby/` — pre-game SPA + HTTP |
| `server` | `open4x-server/` — in-game runtime |
| `cli` | `open4x-cli/` |
| `client-js` | `open4x-client-js/` — JS reference client |
| `client-web` | `open4x-client-web/` — in-game Leptos SPA |
| `sdk` | `open4x-sdk/` |
| `xtask` | `xtask/` — build/dev task runner |

**Cross-cutting** (a change that spans crates or lives outside them):
| scope | area |
|-------|------|
| `auth` | authentication/identity spanning crates (e.g. pubkey login) |
| `deps` | dependency manifests (`Cargo.toml` / `Cargo.lock`) |
| `docker` | Dockerfiles / compose / `.dockerignore` |
| `docs` | top-level docs (README, AGENTS) |
| `book` | the mdBook under `book/` |
| `hooks` | git hooks + this commit tooling |
| `ci` | CI pipelines |
| `repo` | repo-wide config (`.gitignore`, workspace layout) |
| `infra` | deployment/ops that isn't Docker-specific |
| `release` | version bumps / changelogs / tags |

If no scope fits, the change is probably too broad — split it, or add a
new scope to this file in the same PR.
