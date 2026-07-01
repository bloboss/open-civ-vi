# open4x-server (API-only build).
#
# Builds the native Axum binary (no feature flags — the crate is
# unconditionally server-only since the crate-split). The `/api/v1/*`
# and `/ws` routes work; static-file fallbacks return 404 when
# OPEN4X_STATIC_DIR points at an empty dir (fine for the API/CLI
# harness).
#
# Build context: workspace root (run via `docker compose build` from
# `dockerfiles/`).

# ── builder ────────────────────────────────────────────────────────────
FROM rust:1-slim-bookworm AS builder

# pkg-config + build essentials cover the few sys-deps that creep in
# transitively (ring's build script, etc).
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config build-essential ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

# open4x-server is unconditionally a native Axum server since the
# crate-split (the old ssr/csr feature gates were dropped), so it needs
# no feature flags.
RUN cargo build --release -p open4x-server

# ── runtime ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /src/target/release/open4x-server /usr/local/bin/open4x-server

# API-only: leave OPEN4X_STATIC_DIR pointing at an empty dir so the
# ServeDir fallback returns 404 instead of trying to mount the SPA.
RUN mkdir -p /app/empty-static
ENV OPEN4X_STATIC_DIR=/app/empty-static
ENV PORT=3001

EXPOSE 3001
ENTRYPOINT ["/usr/local/bin/open4x-server"]
