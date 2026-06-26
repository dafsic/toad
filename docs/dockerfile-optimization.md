# Dockerfile 优化计划

## 目标

审查并重写当前 `Dockerfile`，解决以下问题：
1. 引入 `cargo chef` 替代脆弱的 dummy-build hack
2. 锁定 Rust 基础镜像版本以保证可复现性
3. 添加 `HEALTHCHECK`（需新增 `/api/health` 端点）
4. 注释改为英文，删除误导性注释
5. 完善 `.dockerignore`

---

## 当前 Dockerfile 状态

**位置**：`/Users/dafsic/development/toad/Dockerfile`

**结构**：三阶段（frontend → backend → runtime），功能正确但存在以下问题：

| # | 问题 | 严重度 |
|---|------|--------|
| 1 | `node:24-alpine` 标签可用性需验证（可能不存在） | 中 |
| 2 | `rust:1-bookworm` 未锁定 rustc 版本 | 中 |
| 3 | Dummy build + 末尾 `grep` 验证 hack 维护负担高 | 高 |
| 4 | 注释中"避免 openssl-sys 在 musl 下问题"具有误导性（项目无 openssl-sys 依赖） | 低 |
| 5 | `cargo build` 未加 `--locked` | 低 |
| 6 | 缺少 `HEALTHCHECK` | 中 |
| 7 | 容器以 root 运行（本次不处理） | 低 |
| 8 | 缺少 `STOPSIGNAL` 显式声明 | 低 |
| 9 | 缺少 OCI `LABEL` | 低 |
| 10 | `.dockerignore` 不全 | 低 |

---

## 文件变更清单

### 新增

1. `.github/workflows/docker-build-pr.yml` — PR 阶段只构建不推送的 workflow

### 修改

1. `Dockerfile` — 完全重写（见下方）
2. `src/api/handlers.rs` — 新增 `health()` handler
3. `src/api/mod.rs` — 在 `public_api` 中注册 `/api/health` 路由
4. `.dockerignore` — 扩展忽略规则

---

## 详细变更

### 1. `src/api/handlers.rs`

在文件末尾添加：

```rust
/// 公开健康检查端点（用于 Docker HEALTHCHECK / 负载均衡探针）
pub async fn health() -> &'static str {
    "ok"
}
```

### 2. `src/api/mod.rs`

将：

```rust
let public_api = Router::new()
    .route("/{exchange}", get(handlers::get_price));
```

改为：

```rust
let public_api = Router::new()
    .route("/health", get(handlers::health))
    .route("/{exchange}", get(handlers::get_price));
```

`/api/health` 必须在 `/{exchange}` 之前注册，否则会被泛型路由吞掉（axum 按注册顺序匹配）。

### 3. `Dockerfile`（完整重写）

```dockerfile
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
```

#### 关键决策说明

| 变更 | 原因 |
|------|------|
| `rust:1.85.0-bookworm` | 锁定 rustc 版本，避免"今天能 build 明天不能" |
| `node:22-alpine` | 当前 LTS（截至 2026-06），`node:24-alpine` 标签可用性需另行验证 |
| `cargo-chef --version 0.1.77` | 锁定 cargo-chef 版本；否则 GHA 缓存可能长期复用旧版 |
| 引入 `cargo chef` | 取代 dummy-build hack；依赖编译独立缓存层，源码变更不触发 deps 重编译 |
| planner 仅 COPY `Cargo.toml`/`Cargo.lock` | `cargo chef prepare` 不需要源码和 `.sqlx/`，减少无效缓存失效 |
| `ENV SQLX_OFFLINE=true` | `sqlx::query!` 编译期使用 `.sqlx/` 离线元数据，无需连接数据库 |
| `cargo build --release --locked` | 强制尊重 `Cargo.lock` |
| `--no-audit --no-fund` | 加快 `npm ci` |
| `apt-get install curl` | `HEALTHCHECK` 需要 |
| `HEALTHCHECK` | 让 `docker ps` 显示健康状态，供编排器使用 |
| `--start-period=30s` | Toad 启动包含交易所连接、Telegram bot、引擎初始化，预留更充裕时间 |
| `STOPSIGNAL SIGTERM` | 显式声明，匹配代码中的 `CancellationToken`（Ctrl+C / SIGTERM 触发优雅关闭） |
| `LABEL` | OCI 标准注解 |
| 保留 root 运行 | 简化 `/data` 卷权限处理；如未来需要非 root，可加 `RUN groupadd -r toad && useradd -r -g toad toad && USER toad` |

#### 为什么不做的变更

- **非 root 用户**：保留 root 简化部署
- **`rust:1-slim-bookworm`**：节省 ~1GB 但工具链调试成本高，不划算
- **多平台镜像（multi-arch）**：需修改 CI workflow，超出本次范围

---

## CI Workflow 变更

### 新增：`.github/workflows/docker-build-pr.yml`

在 PR 阶段执行**只构建、不推送**的 Docker build，避免 Dockerfile 错误在合入 main 后才暴露。

```yaml
name: Docker Build (PR)

on:
  pull_request:
    branches: [main]

jobs:
  docker:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ secrets.DOCKER_USERNAME }}/toad

      - name: Build (no push)
        uses: docker/build-push-action@v6
        with:
          context: .
          file: Dockerfile
          push: false
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
```

### 现有 `.github/workflows/docker-publish.yml`

保持现有触发条件（`push.main` / `push.tags`）和 `cache-from/cache-to: type=gha,mode=max` 不变。新 Dockerfile 对它完全兼容，无需改动。

---

### 4. `.dockerignore`（扩展）

当前内容基础上新增以下规则：

```gitignore
# Build artifacts (扩展)
target/
**/target/
**/node_modules/
**/dist/

# Runtime data
data/

# Sensitive config
.env
.env.local
.env.*.local

# Version control
.git/
.gitignore

# IDE / system files
.DS_Store
.vscode/
.idea/
*.swp
*.swo

# CI / docs / graphs
.github/
.gitlab-ci.yml
.opencode/

# Documentation (not needed in image)
README.md
DESIGN.md
docs/
LICENSE

# Logs / coverage
*.log
coverage/

# Test artifacts
target-*/
```

---

## 缓存命中行为

### `cargo chef` 缓存策略

```
┌──────────────────────────────────────────────────────────────┐
│ Stage 3: builder                                              │
│                                                               │
│  recipe.json 变更?                                            │
│      ├── 否 → 复用 `cargo chef cook` 缓存（依赖已编译）       │
│      └── 是 → 重新 `cargo chef cook`（deps 重编译）           │
│                                                               │
│  src / .sqlx 变更?                                            │
│      └── 不影响 cook 缓存层；只触发最后 `cargo build`         │
└──────────────────────────────────────────────────────────────┘
```

### 典型场景耗时估算

| 场景 | 当前 Dockerfile | 优化后 |
|------|----------------|--------|
| 首次构建（无缓存） | ~10 min | ~10 min（持平） |
| 改 `src/main.rs` 一行 | ~3-5 min（deps 增量编译） | **~10s**（cook 命中） |
| 改 `Cargo.toml` 加新依赖 | ~5 min | ~5 min（recipe.json 变更触发 cook） |
| 改 frontend 一行 | ~30s（仅 npm rebuild） | ~30s（持平） |

---

## 验证步骤

### 1. 本地构建

```sh
docker build -t toad:test .
```

预期：构建成功，无报错。`cargo chef cook` 步骤首次较慢，后续缓存命中。

### 2. 检查镜像大小

```sh
docker images toad:test
```

预期：运行时镜像 < 100MB（vs 当前的 `debian:bookworm-slim` 基础 + 二进制 ≈ 80MB）。

### 3. 本地运行 + 健康检查

`docker-compose.yml` 默认拉取 DockerHub 镜像，因此本地验证需直接用 `docker run`：

```sh
docker run -d \
  --name toad-test \
  -p 3000:3000 \
  -v ./data:/data \
  --env-file .env \
  toad:test

docker inspect --format='{{json .State.Health}}' toad-test
```

预期：30 秒后 `Status: healthy`。

> 若坚持用 `docker compose`，需先给本地镜像打上对应 tag：`docker tag toad:test ${DOCKER_USERNAME}/toad:latest`。

### 4. 健康端点直接验证

```sh
curl -fsS http://localhost:3000/api/health
# 预期输出: ok
```

### 5. CI 验证

#### 已发布镜像

推送到 `main` 分支或打 `v*` tag 触发 `.github/workflows/docker-publish.yml`。预期：
- `cargo chef prepare` 在 planner 阶段成功
- `cargo chef cook` 命中缓存（或首次完整编译）
- 最终镜像 push 到 DockerHub

#### PR 阶段构建

本次新增 `.github/workflows/docker-build-pr.yml`。任何针对 `main` 的 PR 都会触发**只构建、不推送**的 workflow，确保 Dockerfile 改动在合入前就能暴露问题。

### 6. 回归测试

```sh
cargo test
```

预期：所有测试通过。`/api/health` 是新代码，不影响现有逻辑。

---

## 回滚计划

如新 Dockerfile 导致问题，回滚步骤：

```sh
git checkout HEAD~1 -- Dockerfile .dockerignore src/api/handlers.rs src/api/mod.rs
docker build -t toad:rollback .
```

---

## 实施顺序

1. 先改 `src/api/handlers.rs` 和 `src/api/mod.rs`（加 health 端点）
2. 运行 `cargo test` 确保 health 端点不破坏现有功能
3. 重写 `Dockerfile`
4. 更新 `.dockerignore`
5. 新增 `.github/workflows/docker-build-pr.yml`
6. 本地 `docker build` 验证
7. 推送到分支跑 CI（PR 阶段会触发新的 build-only workflow）
8. CI 通过后合入 main

---

## 未处理项（未来可能需要）

- **镜像体积进一步缩小**：换 `gcr.io/distroless/cc-debian12`（无 shell、无 curl，需调整 HEALTHCHECK 为外部脚本）
- **多架构镜像**：在 GHA workflow 中加 `platforms: linux/amd64,linux/arm64`
- **SBOM / provenance**：CI 加 `--sbom=true --provenance=true`
- **非 root 用户**：配合 distroless 实现彻底最小化
- **镜像签名**：`cosign sign` 推送到 DockerHub 后
