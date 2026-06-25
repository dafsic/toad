# 🐸 Toad Grid Bot

XMR/USDC infinite reverse grid trading bot. Supports **Kraken spot**, **MEXC spot**, and **Hyperliquid perpetual** (isolated mode).

The user places a manual limit order. On fill, the system automatically places the reverse limit order at a fixed price delta, ping-ponging until manually cancelled. Supports running multiple independent grids simultaneously.

---

## 功能特性

- **Web 界面** — 下单表单、实时订单列表、状态推送（SSE）、订单删除
- **Telegram 认证** — 基于 Telegram Bot 的无密码登录，JWT token（8 小时有效期）
- **Telegram Bot** — `/order` `/orders` `/cancel` `/login`，成交实时通知
- **无限链式网格** — 成交 → 自动挂反向单 → 循环，直到取消
- **部分成交跟踪** — WebSocket 实时更新已成交数量
- **轮询驱动** — 每 60 秒轮询确认完全成交并挂对手单，WebSocket 断线不影响网格运行
- **单二进制部署** — 前端静态资源编译进 Rust 二进制，无需 Nginx
- **Docker 一键启动**

---

## 快速开始

### 方式一：Docker Compose（推荐）

```bash
cp .env.example .env
# 编辑 .env 填入真实密钥
docker compose up -d
# 浏览器访问 http://localhost:3000
```

### 方式二：本地编译运行

**前置要求：** Rust 1.85+、Node.js 22+

```bash
# 1. 配置环境变量
cp .env.example .env && vi .env

# 2. 构建前端
cd frontend && npm install && npm run build && cd ..

# 3. 运行（sqlx 宏需要 DATABASE_URL）
DATABASE_URL=sqlite:data/bot.db cargo run
```

---

## 环境变量

复制 `.env.example` 并填入真实值：

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `TELEGRAM_BOT_TOKEN` | Telegram Bot Token | 必填 |
| `ALLOWED_TELEGRAM_USER_ID` | 允许操作的 Telegram User ID | 必填 |
| `KRAKEN_API_KEY` | Kraken API Key | 必填 |
| `KRAKEN_API_SECRET` | Kraken API Secret（Base64）| 必填 |
| `MEXC_API_KEY` | MEXC API Key | 必填 |
| `MEXC_API_SECRET` | MEXC API Secret | 必填 |
| `HYPERLIQUID_PRIVATE_KEY` | Hyperliquid API 钱包私钥（hex）| 必填 |
| `HYPERLIQUID_ACCOUNT_ADDRESS` | 主账户地址（API agent wallet 时填写）| 空 |
| `HYPERLIQUID_TESTNET` | 连接测试网 | `false` |
| `JWT_SECRET` | JWT 签名密钥（请修改为随机字符串）| `change-me-in-production` |
| `SERVER_ADDR` | HTTP 监听地址 | `0.0.0.0:3000` |
| `DATABASE_URL` | SQLite 路径 | `sqlite:data/bot.db` |

---

## Web UI Authentication

On first visit to the web UI, the system presents a **6-digit code**.

### 登录流程

1. **打开浏览器** — 访问 `http://localhost:3000`
2. **获取验证码** — 页面自动生成 6 位验证码（如 `123456`）
3. **Telegram 验证** — 在 Telegram 中向 Bot 发送：
   ```
   /login 123456
   ```
4. **自动登录** — 验证成功后浏览器自动跳转到主页面

### Token 有效期

- JWT token 有效期为 **8 小时**
- 过期后需重新验证
- Token 直接编码过期时间，后端无需存储或定期清理

---

## Telegram Bot Commands

| Command | Description |
|---------|-------------|
| `/login <code>` | Web UI login verification (6-digit code) |
| `/order <exchange> <side> <qty> <price> <price_change> [leverage]` | Place order |
| `/orders [status]` | List orders (status optional: open/partially_filled/filled/cancelled/failed, default open) |
| `/cancel <id>` | Cancel the specified order |

**Examples:**
```
/order kraken buy 2.5 145.80 1.50
/order hyperliquid sell 1.0 150.00 2.00 5
/order mexc_spot buy 2.5 145.80 1.50
/cancel 42
```

---

## 网格逻辑

```
buy 成交  →  sell 挂单，价格 = 挂单价格 + price_change
sell 成交 →  buy  挂单，价格 = 挂单价格 - price_change
```

对手单继承相同的 `price_change` 和 `leverage`，形成持续振荡的双向网格。

### Assisted Mode

`price_change = 0` means **assisted order**: the order is submitted and tracked normally, but no reverse leg is placed after fill. Useful when you only want to place orders across platforms without grid oscillation.

### Order Status Flow

```
pending → open → partially_filled → filled | cancelled | failed
```

- `pending`: Written to DB, not yet submitted to exchange
- `open`: Exchange accepted the order
- `partially_filled`: WebSocket reported partial fill, `filled_quantity` updated in realtime
- `filled`: Polling confirmed full fill and placed the reverse leg
- `cancelled` / `failed`: Terminal states

### WebSocket vs Polling Division of Labor

- **WebSocket** (fill events): only updates `filled_quantity` and status → `partially_filled`, **does not place reverse orders**
- **Polling** (every 60s per exchange): checks whether the current lowest sell / highest buy are fully filled → places reverse if so

The grid continues to work even if WebSocket is completely unavailable thanks to polling.

---

## Polling & Recovery on Restart

The polling task runs continuously (every 60s per exchange). It is used both for restart recovery and as the normal driver for placing reverse legs. On restart the first tick fires immediately and **actively inspects** the status of all active orders (`open` + `partially_filled`) that may have changed while stopped.

### Polling Flow

1. **Load active orders** — query all orders with `status='open'` or `'partially_filled'`
2. **Pick candidates** — for each exchange select the current lowest-price sell and highest-price buy
3. **Query exchange** — call the exchange status API for those two candidate orders
4. **Sync state**:
   - **filled** → trigger reverse using the original order price (preserves grid spacing)
   - **cancelled** → update DB and send Telegram notification
   - **still open** → wait for next poll

### Important Note

⚠️ **Fill Price Limitation**

Due to exchange status API limitations, when polling detects a full fill it cannot obtain the exact fill price. The system uses the **original order price** as `filled_price`. Reverse orders are priced from `order.price ± price_change`. This guarantees consistent grid spacing independent of actual execution price.

### 日志示例

```
2026-06-17T12:00:00Z INFO toad::engine: poll: order fully filled, triggering reverse grid leg id=42
2026-06-17T12:00:01Z INFO toad::engine: reverse grid leg placed new_id=43
```

---

## 开发

```bash
# 后端检查
DATABASE_URL=sqlite:data/bot.db cargo check
cargo clippy

# 前端开发（热更新，代理到 localhost:3000）
cd frontend && npm run dev

# 完整构建
npm run build --prefix frontend && cargo build --release
```

Database migrations live in `src/db/migrations/` and run automatically via `sqlx::migrate!` on startup.

NOTE: Current migrations use destructive DROP+CREATE (data loss on re-apply). This is accepted for the project. Always back up `data/bot.db` before upgrading or re-running migrations.

### Compile-time database requirement

`sqlx::query!` macros validate SQL at compile time, so a database with the correct schema must exist when running `cargo check` / `cargo build`. If you get `unable to open database file` errors, create it first:

```bash
mkdir -p data && touch data/bot.db
DATABASE_URL=sqlite:data/bot.db sqlx migrate run --source src/db/migrations
```

If migrations were previously applied but a migration file was modified (checksum mismatch), delete the dev database and re-run the command above:

```bash
rm -f data/bot.db data/bot.db-wal data/bot.db-shm
```

---

## 技术栈

| 层 | 技术 |
|----|------|
| 后端 | Rust · Tokio · Axum 0.8 |
| 数据库 | SQLite · sqlx 0.8 |
| Telegram | teloxide 0.13 |
| 前端 | React 19 · Vite · Tailwind CSS · Radix UI |
| 静态嵌入 | rust-embed（单二进制） |
| 实时推送 | Server-Sent Events |
| 认证 | JWT (HS256, jsonwebtoken) |
| Hyperliquid | [hypersdk](https://github.com/infinitefield/hypersdk)（git 主分支） |
| MEXC 现货 | reqwest + tokio-tungstenite（HMAC-SHA256 + listenKey 用户数据流）|

---

## 安全说明

- 所有密钥通过环境变量注入，不写入代码或镜像
- Telegram Bot 强制校验 `ALLOWED_TELEGRAM_USER_ID`，拒绝所有非授权用户
- 建议在反向代理层（Caddy / Nginx）启用 HTTPS + Basic Auth
- 定期备份 `data/bot.db`

---

