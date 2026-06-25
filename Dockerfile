# ── 多阶段构建 ────────────────────────────────────────────────────────────────

# Stage 1: 构建前端
FROM node:22-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: 构建 Rust 后端（并嵌入前端产物）
# 使用 Debian (glibc) 而非 Alpine (musl)：rust:1-bookworm 基于 buildpack-deps，
# 已内置 libssl-dev / pkg-config，避免 openssl-sys 在 musl 下的编译问题。
FROM rust:1-bookworm AS backend
# sqlx::query! 编译时宏使用离线数据（.sqlx/），无需构建期连接数据库
ENV SQLX_OFFLINE=true
WORKDIR /app

# 先缓存依赖：用 dummy src 编译依赖（失败无妨，依赖已缓存）
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release || true \
    && rm -rf src

# 编译真实源码
COPY src/ ./src/
COPY .sqlx/ ./.sqlx/
# 将前端 dist/ 复制到 rust-embed 期望的位置
COPY --from=frontend /app/frontend/dist ./frontend/dist
RUN cargo build --release

# Stage 3: 最小运行时镜像
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=backend /app/target/release/toad /usr/local/bin/toad

VOLUME ["/data"]
EXPOSE 3000
ENV DATABASE_URL=/data/bot.db

ENTRYPOINT ["toad"]
