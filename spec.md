# 加密货币无限链式反向网格交易机器人
## 最终技术方案（单用户独立部署版）

---

## 一、需求规格

### 1. 项目概述

开发一个支持从 **Web 页面** 和 **Telegram Bot** 下单的加密货币自动化交易系统。核心特性为**无限链式反向网格**：

用户手动下达一笔限价单后，系统在该订单成交时自动在**相反方向**以固定价差（`price_change`）挂出新限价单。新订单成交后继续使用相同的 `price_change` 触发下一笔反向订单，形成持续链式振荡网格，直到用户手动取消相关挂单为止。

当前版本仅支持：
- **Kraken 现货** 和 **Hyperliquid 永续合约**
- **XMR/USDC** 交易对

### 2. 支持范围

- **交易所**：
  - Kraken（现货）
  - Hyperliquid（永续合约）
- **交易对**：XMR/USDC（Kraken API symbol 为 `XMRUSDC`，Hyperliquid 为 `XMR`）
- **部署模式**：用户独立部署（单用户实例）

### 3. 功能需求

#### 3.1 下单渠道
- **Web 前端**：提供订单录入表单、当前挂单列表、实时状态展示
- **Telegram Bot**：支持命令式下单与交互式菜单，并接收成交、取消、异常通知

#### 3.2 订单参数
| 参数            | 类型     | 说明                                      | 示例          |
|-----------------|----------|-------------------------------------------|---------------|
| `exchange`      | string   | `kraken` / `hyperliquid`                  | `kraken`      |
| `symbol`        | string   | `XMR/USDC`                                | `XMR/USDC`    |
| `side`          | string   | `buy` / `sell`                            | `buy`         |
| `quantity`      | float    | 下单数量（XMR 数量）                      | `2.5`         |
| `price`         | float    | 限价单价格                                | `145.80`      |
| `price_change`  | float    | 网格价差（正数，链式继承使用）            | `1.50`        |
| `leverage`      | integer  | 杠杆倍数（Kraken 固定为 1；Hyperliquid ≥1）| `5`           |

订单类型：限价单（GTC）。

#### 3.3 无限链式反向网格核心逻辑
- 用户手动下单成功后，系统记录订单并提交到交易所。
- 当订单**完全成交**时：
  - 获取实际成交均价 `filled_price`
  - 计算反向订单价格：
    - 原订单为 `buy` → 反向 `sell`，价格 = `filled_price + price_change`
    - 原订单为 `sell` → 反向 `buy`，价格 = `filled_price - price_change`
  - 自动提交**相反方向、相同数量、相同 `price_change`、相同 `leverage`** 的新限价单
  - 新订单继承 `price_change` 与 `leverage`，继续监听并链式触发后续反向订单
  - Hyperliquid 永续合约订单始终使用**逐仓（isolated）模式**，每次下单前自动设置杠杆
  - Kraken 现货 `leverage` 固定为 1，不涉及合约杠杆
- 形成在两个价格水平附近持续振荡的**双向 ping-pong 网格**
- 用户可同时运行多个不同 `price_change` 的独立网格

#### 3.4 订单管理功能
- 用户可随时查看**当前所有挂单**（手动 + 自动生成）
- 支持按交易所、方向、是否自动生成等条件筛选
- 用户可手动取消任意挂单（单个或批量）
- 取消操作仅中断该订单后续链式触发，已成交的父订单不受影响

### 4. 非功能性需求
- 支持通过环境变量或命令行参数在启动时注入配置（Telegram User ID、交易所 API Key/Secret）
- 仅允许配置的 Telegram 用户操作
- 系统重启后能恢复所有活跃网格链路
- 完整的操作日志与异常告警
- 安全：API Key 仅在内存中使用，用户负责部署环境安全

---

## 二、技术方案

### 1. 技术栈总览

| 层级         | 技术选型                              | 说明 |
|--------------|---------------------------------------|------|
| 后端语言     | Rust + Tokio + Axum                   | 高性能、内存安全 |
| 配置解析     | clap（derive + env）                  | 支持 CLI 参数 + 环境变量 |
| 数据库       | SQLite + sqlx                         | 轻量、零配置 |
| Telegram Bot | teloxide                              | 成熟的 Rust Telegram 框架 |
| 前端         | React 19 + Vite + Tailwind + shadcn/ui | 现代开发体验 |
| 静态资源嵌入 | rust-embed                            | 前端产物编译进二进制，单文件启动 |
| 实时通信     | Server-Sent Events (SSE)              | 简单可靠的订单状态推送 |
| 部署         | Docker（单容器）                       | 一个二进制即可运行，无需 Nginx |
| Hyperliquid SDK | hypersdk（git 主分支）               | Hyperliquid 官方 Rust SDK，含 EIP-712 签名、WebSocket |
| 其他         | reqwest, serde, tokio-tungstenite     | 交易所 API 调用与 WebSocket（Kraken） |

### 2. 配置管理

使用 `clap` 实现启动时配置注入（推荐配合 `.env` 文件）。

**核心配置项**（通过环境变量或命令行参数传入）：
- `TELEGRAM_BOT_TOKEN`
- `ALLOWED_TELEGRAM_USER_ID`（仅此用户可操作）
- `KRAKEN_API_KEY` / `KRAKEN_API_SECRET`
- `HYPERLIQUID_PRIVATE_KEY`（API 钱包私钥，十六进制）
- `HYPERLIQUID_ACCOUNT_ADDRESS`（主账户地址；API agent wallet 模式时必填，普通钱包留空）
- `HYPERLIQUID_TESTNET`（`true` 连接测试网，默认 `false`）
- `DATABASE_URL`（默认 `data/bot.db`）
- `SERVER_ADDR`

启动时所有敏感信息加载到内存中的 `Config` 结构体，后续不再从数据库读取。

### 3. 数据库设计

```sql
CREATE TABLE orders (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange         TEXT    NOT NULL CHECK(exchange IN ('kraken','hyperliquid')),
    symbol           TEXT    NOT NULL DEFAULT 'XMR/USDC',
    side             TEXT    NOT NULL CHECK(side IN ('buy','sell')),
    quantity         REAL    NOT NULL,
    price            REAL    NOT NULL,
    price_change     REAL    NOT NULL,
    -- 杠杆倍数：Kraken 现货固定为 1，Hyperliquid 永续合约由用户指定。
    -- 对手单（链式反向订单）继承父订单的相同杠杆。
    leverage         INTEGER NOT NULL DEFAULT 1 CHECK(leverage >= 1),
    is_auto          INTEGER NOT NULL DEFAULT 0,
    parent_order_id  INTEGER,
    exchange_order_id TEXT,
    status           TEXT    NOT NULL DEFAULT 'pending'
                             CHECK(status IN ('pending','open','filled','cancelled','failed')),
    filled_price     REAL,
    created_at       TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_order_id) REFERENCES orders(id)
);

CREATE INDEX idx_orders_status  ON orders(status);
CREATE INDEX idx_orders_parent  ON orders(parent_order_id);
CREATE INDEX idx_orders_exchange ON orders(exchange);
```

### 4. 后端架构

**核心模块**：
- `config`：clap 配置解析
- `db`：sqlx + SQLite 操作
- `exchange`：`ExchangeAdapter` Trait + Kraken / Hyperliquid 具体实现
- `engine`：Grid Engine（成交监听 + 链式反向订单生成）
- `api`：Axum REST 接口（下单、查询、取消）
- `bot`：teloxide Telegram Bot（带用户 ID 校验）
- `sse`：实时订单状态推送

**AppState** 在启动时初始化，包含 `Config` 和各交易所适配器。

**成交监听**：优先使用交易所 WebSocket，降级为 REST 轮询。

### 5. Telegram Bot 权限控制

在所有 handler 中校验：

```rust
if msg.from.map_or(true, |u| u.id.0 != config.allowed_telegram_user_id) {
    // 拒绝非授权用户
}
```

### 6. 前端

- React + Tailwind + shadcn/ui 构建管理界面
- 订单列表（支持筛选、取消按钮）
- 下单表单
- 通过 SSE 实时更新订单状态

前端构建产物通过 `rust-embed` 在编译期直接嵌入后端二进制：

```rust
#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

// Axum 路由托管嵌入的静态文件
let app = Router::new()
    .route("/api/...", ...)
    .fallback(|uri: Uri| async move { serve_embedded_asset(uri) });
```

用户启动程序后直接通过浏览器访问 `http://localhost:<SERVER_ADDR>` 即可，无需单独部署 Nginx 或 Web 服务器。建议在反向代理层（如 Caddy）配合 HTTPS + Basic Auth 加强保护。

**项目结构**：
```
toad/
├── src/          # Rust 后端
├── frontend/     # React 前端（独立 npm 项目）
│   ├── src/
│   └── dist/     # vite build 输出，由 rust-embed 嵌入
└── Cargo.toml
```

**构建流程**：先执行 `npm run build`（生成 `frontend/dist/`），再执行 `cargo build --release`，rust-embed 在编译期将 `dist/` 打包进二进制。

### 7. Docker 部署

由于前端静态资源已嵌入二进制，**只需单个容器**：

```dockerfile
# 多阶段构建
FROM node:22-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1.87-alpine AS backend
WORKDIR /app
COPY . .
COPY --from=frontend /app/frontend/dist ./frontend/dist
RUN cargo build --release

FROM alpine:3.21
COPY --from=backend /app/target/release/toad /usr/local/bin/toad
VOLUME /data
ENTRYPOINT ["toad"]
```

```yaml
# docker-compose.yml
services:
  toad:
    image: toad
    ports:
      - "3000:3000"
    volumes:
      - ./data:/data
    env_file: .env
```

用户通过 `.env` 文件注入所有配置，一键 `docker compose up -d` 即可运行，无需 Nginx 容器。

### 8. 安全与运维

- 所有密钥通过环境变量注入，**不在代码或镜像中硬编码**
- 建议在外层配合 Caddy 或 Nginx 反向代理，启用 HTTPS + Basic Auth
- 前端静态资源嵌入二进制，无运行时文件依赖
- 定期备份 `data/bot.db`
- 日志使用 `tracing` 输出，便于排查问题

### 9. 开发与迭代建议

**MVP 开发顺序**（推荐 4 周内完成）：
1. 配置模块 + 数据库初始化
2. 交易所适配器基础功能
3. 手动下单 + 订单管理 API
4. 无限链式反向网格核心逻辑
5. Telegram Bot（含权限校验）
6. SSE + React 前端（Vite 独立开发，`vite build` 产出 `frontend/dist/`）
7. rust-embed 集成 + 单二进制验证
8. Dockerfile 多阶段构建 + Docker Compose 打包与部署文档

---

**本文档为最终完整技术方案**，已包含全部需求与实现细节，可直接用于开发启动。

如需配套的代码模板、完整 Dockerfile、`.env.example` 或 rust-embed 集成示例，请随时告知。