# 🐸 Toad Grid Bot

XMR/USDC 无限链式反向网格交易机器人。支持 **Kraken 现货** 和 **Hyperliquid 永续合约**（逐仓模式）。

用户手动下一笔限价单，成交后系统自动以固定价差挂出反向限价单，持续 ping-pong 振荡，直到手动取消为止。支持同时运行多个独立网格。

---

## 功能特性

- **Web 界面** — 下单表单、实时订单列表、状态推送（SSE）
- **Telegram Bot** — `/order` `/orders` `/cancel`，成交实时通知
- **无限链式网格** — 成交 → 自动挂反向单 → 循环，直到取消
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
| `HYPERLIQUID_PRIVATE_KEY` | Hyperliquid API 钱包私钥（hex）| 必填 |
| `HYPERLIQUID_ACCOUNT_ADDRESS` | 主账户地址（API agent wallet 时填写）| 空 |
| `HYPERLIQUID_TESTNET` | 连接测试网 | `false` |
| `SERVER_ADDR` | HTTP 监听地址 | `0.0.0.0:3000` |
| `DATABASE_URL` | SQLite 路径 | `sqlite:data/bot.db` |

---

## Telegram Bot 命令

| 命令 | 说明 |
|------|------|
| `/order <exchange> <side> <qty> <price> <price_change> [leverage]` | 下单 |
| `/orders [open\|filled\|cancelled]` | 查看订单 |
| `/cancel <id>` | 取消指定挂单 |

**示例：**
```
/order kraken buy 2.5 145.80 1.50
/order hyperliquid sell 1.0 150.00 2.00 5
/cancel 42
```

---

## 网格逻辑

```
buy 成交  →  sell 挂单，价格 = filled_price + price_change
sell 成交 →  buy  挂单，价格 = filled_price - price_change
```

对手单继承相同的 `price_change` 和 `leverage`，形成持续振荡的双向网格。

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

数据库迁移文件位于 `src/db/migrations/`，通过 `sqlx::migrate!` 在启动时自动执行。

---

## 技术栈

| 层 | 技术 |
|----|------|
| 后端 | Rust · Tokio · Axum 0.8 |
| 数据库 | SQLite · sqlx 0.8 |
| Telegram | teloxide 0.13 |
| 前端 | React 19 · Vite · Tailwind CSS |
| 静态嵌入 | rust-embed（单二进制） |
| 实时推送 | Server-Sent Events |
| Hyperliquid | [hypersdk](https://github.com/infinitefield/hypersdk)（git 主分支） |

---

## 安全说明

- 所有密钥通过环境变量注入，不写入代码或镜像
- Telegram Bot 强制校验 `ALLOWED_TELEGRAM_USER_ID`，拒绝所有非授权用户
- 建议在反向代理层（Caddy / Nginx）启用 HTTPS + Basic Auth
- 定期备份 `data/bot.db`

---

## License

MIT
