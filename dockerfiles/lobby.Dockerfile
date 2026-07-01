# open4x-lobby (pre-game surface: landing / login / new-game wizard).
#
# Builds the native Axum binary with the default `ssr` feature, which
# pulls open4x-accounts/persistence (sqlite) + the magic-link mailer.
# Auth, /api/v1/me, /api/v1/games and the orchestrator all work; the
# SPA is served from OPEN4X_LOBBY_STATIC_DIR when set, otherwise from
# the embedded bundle (empty unless `trunk build` ran first — API-only
# deploys leave it unset and 404 the SPA shell, which is fine for the
# curl / auth harness).
#
# Build context: workspace root (run via `docker compose build`).

# ── builder ────────────────────────────────────────────────────────────
FROM rust:1-slim-bookworm AS builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config build-essential ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

RUN cargo build --release -p open4x-lobby

# ── runtime ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /src/target/release/open4x-lobby /usr/local/bin/open4x-lobby

# Persistent sqlite + avatar store lives under the data dir volume.
ENV OPEN4X_LOBBY_DATA_DIR=/app/data
ENV PORT=3002

EXPOSE 3002
ENTRYPOINT ["/usr/local/bin/open4x-lobby"]
