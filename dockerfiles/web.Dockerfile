# open4x-client-web image — builds the Leptos/WASM SPA with trunk and
# serves it via nginx, proxying /api/* and /ws to the server container.
#
# Build context: workspace root.

# ── builder ────────────────────────────────────────────────────────────
FROM rust:1-slim-bookworm AS builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config build-essential ca-certificates \
 && rm -rf /var/lib/apt/lists/*

RUN cargo install trunk --locked \
 && rustup target add wasm32-unknown-unknown

WORKDIR /src
COPY . .

# trunk's entry point lives in open4x-server/ alongside the server binary's
# Cargo.toml; it references open4x-client-web as the WASM crate.
RUN cd open4x-server && trunk build --release index.html

# ── runtime ────────────────────────────────────────────────────────────
FROM nginx:alpine AS runtime

COPY --from=builder /src/open4x-server/dist /usr/share/nginx/html
COPY dockerfiles/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080
