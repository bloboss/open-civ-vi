# open4x-web image — builds the lightweight pure-JS + WebGL2 reference client
# (open4x-client-js) with tsc (no bundler) and serves the static files via
# nginx, proxying /api/ and /ws to the server container.
#
# Build context: workspace root (run via `docker compose build` / `podman-compose`).

# ── builder ────────────────────────────────────────────────────────────
FROM node:22-slim AS builder

WORKDIR /src
# The committed generated types (src/gen/protocol) ship with the source, so a
# plain `tsc` build needs no network beyond the typescript devDependency.
COPY open4x-client-js/ ./
RUN npm install --no-audit --no-fund \
 && npx tsc

# ── runtime ────────────────────────────────────────────────────────────
FROM nginx:alpine AS runtime

# Static client = the HTML shell + stylesheet + the tsc-compiled ES modules.
# (src/gen/*.ts are type-only and erased at compile, so only dist/ is served.)
COPY --from=builder /src/index.html /src/styles.css /usr/share/nginx/html/
COPY --from=builder /src/dist /usr/share/nginx/html/dist
COPY dockerfiles/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080
