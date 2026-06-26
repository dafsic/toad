# Multi-stage build for toad grid bot

# Stage 0: cargo-chef base (used by planner + builder)
FROM rust:1.85.0-bookworm AS chef
RUN cargo install cargo-chef --version 0.1.77 --locked \
    && rm -rf $CARGO_HOME/registry

# Stage 1: build the React frontend
FROM node:22-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

# Stage 2: generate the dependency recipe (caching key)
FROM chef AS planner
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: cook dependencies (cached until recipe.json changes),
#           then build the real binary
FROM chef AS builder
WORKDIR /app
# sqlx::query! macros use offline query metadata (.sqlx/); no DB connection at build time
ENV SQLX_OFFLINE=true
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY .sqlx ./.sqlx
COPY --from=frontend /app/frontend/dist ./frontend/dist
RUN cargo build --release --locked --bin toad

# Stage 4: minimal runtime image
FROM debian:bookworm-slim AS runtime
LABEL org.opencontainers.image.source="https://github.com/anomalyco/toad" \
      org.opencontainers.image.licenses="MIT"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/toad /usr/local/bin/toad

VOLUME ["/data"]
EXPOSE 3000
ENV DATABASE_URL=/data/bot.db

HEALTHCHECK --interval=30s --timeout=3s --start-period=30s --retries=3 \
    CMD curl -fsS http://127.0.0.1:3000/api/health || exit 1

STOPSIGNAL SIGTERM
ENTRYPOINT ["toad"]
