# Multi-stage build: compile server + frontend, produce a minimal runtime image.
#
# Usage:
#   docker compose up --build
#   # or standalone:
#   docker build -t open4x .
#   docker run -p 3001:3001 -v open4x-data:/app/data open4x

# ============================================================================
# Stage 1: Build the server binary (native Linux)
# ============================================================================
FROM rust:1.86-bookworm AS build-server

WORKDIR /src
COPY . .

RUN cargo build --release -p open4x-server --features ssr --no-default-features

# ============================================================================
# Stage 2: Build the WASM frontend with trunk
# ============================================================================
FROM rust:1.86-bookworm AS build-web

# Install trunk and wasm target.
RUN cargo install trunk --locked \
    && rustup target add wasm32-unknown-unknown

WORKDIR /src
COPY . .

# Trunk builds from the index.html in open4x-server/.
RUN cd open4x-server && trunk build --release index.html

# ============================================================================
# Stage 3: Minimal runtime image
# ============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the server binary.
COPY --from=build-server /src/target/release/open4x-server /app/open4x-server

# Copy the trunk-built frontend static files.
COPY --from=build-web /src/open4x-server/dist /app/static

# Create the data directory for persistence.
RUN mkdir -p /app/data

# Environment configuration.
ENV PORT=3001
ENV OPEN4X_STATIC_DIR=/app/static
ENV OPEN4X_DATA_DIR=/app/data

EXPOSE 3001

# Health check.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3001/health || exit 1

ENTRYPOINT ["/app/open4x-server"]
