# ── 多阶段构建 ────────────────────────────────────────────────────────────────

# Stage 1: 构建前端
FROM node:22-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: 构建 Rust 后端（并嵌入前端产物）
FROM rust:1.87-alpine AS backend
RUN apk add --no-cache musl-dev sqlite-dev
# sqlx::query! 编译时宏使用离线数据（.sqlx/），无需构建期连接数据库
ENV SQLX_OFFLINE=true
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY .sqlx/ ./.sqlx/
# 将前端 dist/ 复制到 rust-embed 期望的位置
COPY --from=frontend /app/frontend/dist ./frontend/dist
RUN cargo build --release

# Stage 3: 最小运行时镜像
FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=backend /app/target/release/toad /usr/local/bin/toad

VOLUME ["/data"]
EXPOSE 3000
ENV DATABASE_URL=/data/bot.db

ENTRYPOINT ["toad"]
