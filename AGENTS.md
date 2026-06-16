# AGENTS.md — Toad Grid Bot

开发指南，供 AI agent 参与本项目开发时参考。

---

## 项目一句话概述

XMR/USDC 无限链式反向网格交易机器人：用户手动下一笔限价单，成交后系统自动以固定价差挂出反向限价单，持续 ping-pong 振荡，直到用户取消为止。支持 Kraken 现货 和 Hyperliquid 永续合约（逐仓模式）。

---

## 仓库结构

```
toad/
├── src/
│   ├── main.rs              # 启动入口（待实现）
│   ├── config.rs            # clap + env 配置解析（待实现）
│   ├── assets.rs            # rust-embed 静态文件托管
│   ├── db/
│   │   ├── mod.rs           # init_pool：连接池 + migrations
│   │   ├── order.rs         # CRUD：insert/get/list/update
│   │   └── migrations/001_init.sql
│   ├── exchange/
│   │   ├── mod.rs           # ExchangeAdapter trait + 公共类型
│   │   ├── kraken.rs        # Kraken REST + WebSocket（已实现）
│   │   └── hyperliquid.rs   # hypersdk git 主分支（已实现）
│   ├── engine/mod.rs        # GridEngine：成交监听 + 链式下单（已实现）
│   ├── api/
│   │   ├── mod.rs           # Axum 路由 + AppState（已实现）
│   │   └── handlers.rs      # create_order / list_orders / cancel_order（已实现）
│   ├── sse/mod.rs           # SSE broadcast channel + handler（已实现）
│   └── bot/mod.rs           # Telegram Bot（待实现）
├── frontend/                # React 19 + Vite + Tailwind + shadcn/ui（待实现）
├── skills/backend-api/SKILL.md  # 后端 API 契约，前端开发必读
├── Cargo.toml
├── Dockerfile               # 多阶段构建
├── docker-compose.yml
└── .env.example
```

---

## 技术栈

| 层 | 选型 |
|----|------|
| 后端 | Rust + Tokio + Axum 0.8 |
| 数据库 | SQLite + sqlx 0.8（WAL 模式，外键启用） |
| Telegram | teloxide 0.13 |
| 前端 | React 19 + Vite + Tailwind + shadcn/ui |
| 前端嵌入 | rust-embed（编译进二进制，单文件启动） |
| 实时推送 | Server-Sent Events（broadcast channel） |
| Hyperliquid | hypersdk git 主分支（非 crates.io 0.2） |
| Kraken | 纯 REST + WebSocket v2，无第三方 SDK |

---

## 关键约定

### 订单模型
- `leverage`：Kraken 固定为 1，Hyperliquid 用户指定（≥1），链式对手单**继承**父订单值
- `is_auto = true`：网格引擎自动生成的对手单
- `parent_order_id`：指向触发本订单的已成交父订单
- 状态流：`pending → open → filled | cancelled | failed`
- 下单流程：**先写 pending 入库** → 提交交易所 → 成功后升级 `open`，失败标记 `failed`

### 游标分页
`GET /api/orders` 用 `id DESC` 游标分页，参数为 `before_id` + `limit`，响应含 `next_cursor`。详见 [skills/backend-api/SKILL.md](skills/backend-api/SKILL.md)。

### Exchange Adapter
实现 `src/exchange/mod.rs` 中的 `ExchangeAdapter` trait：
```rust
place_limit_order(req: &OrderRequest) -> anyhow::Result<OrderConfirmation>
cancel_order(exchange_order_id: &str, symbol: &str) -> anyhow::Result<()>
get_order_status(...) -> anyhow::Result<String>  // "open"|"filled"|"cancelled"|"unknown"
subscribe_fills(tx: mpsc::Sender<FillEvent>) -> anyhow::Result<()>  // 内部自动重连
```

### Hyperliquid 特殊要求
- 使用 `hypersdk = { git = "https://github.com/infinitefield/hypersdk" }`（非 crates.io）
- 每次下单前调用 `update_leverage(is_cross=false)` 确保逐仓模式
- 私钥通过 `HYPERLIQUID_PRIVATE_KEY` 注入；`HYPERLIQUID_ACCOUNT_ADDRESS` 为 agent wallet 的主账户地址

### 配置注入
所有敏感信息通过环境变量或 CLI 参数传入，启动时加载进 `Config` 结构体，**不读写数据库**：
```
TELEGRAM_BOT_TOKEN, ALLOWED_TELEGRAM_USER_ID
KRAKEN_API_KEY, KRAKEN_API_SECRET
HYPERLIQUID_PRIVATE_KEY, HYPERLIQUID_ACCOUNT_ADDRESS, HYPERLIQUID_TESTNET
DATABASE_URL, SERVER_ADDR
```

---

## 待实现模块（优先级顺序）

1. **`src/config.rs`** — clap derive + env，实现 `Config::parse()`
2. **`src/main.rs`** — 串联所有模块：init_pool → build adapters → start engine → start bot → start HTTP server
3. **`src/bot/mod.rs`** — Telegram Bot，命令：`/order` `/orders` `/cancel <id>`；所有 handler 校验 `allowed_telegram_user_id`
4. **`frontend/`** — 参考 [skills/backend-api/SKILL.md](skills/backend-api/SKILL.md) 对接后端
5. **`src/assets.rs`** — rust-embed fallback handler（SPA 路由回退 index.html）

---

## 开发注意事项

- **不要修改已完成的模块**（exchange、engine、api、sse、db）除非有明确 bug
- `sqlx::query!` 宏需要 `DATABASE_URL` 环境变量或 `.env` 文件，编译前确保存在
- Hyperliquid WebSocket 使用 `hypersdk` 内置的 yawc；Kraken WebSocket 使用 `tokio-tungstenite`
- SSE 事件格式：`{"type":"order_created","order_id":42}` / `{"type":"order_updated","order_id":42,"status":"filled"}`
- 前端开发时 `vite.config.ts` 已配置 `/api` 代理到 `http://localhost:3000`
- 构建顺序：`npm run build`（frontend/）→ `cargo build --release`（rust-embed 嵌入 dist/）
