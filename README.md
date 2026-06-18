# 🐸 Toad Grid Bot

XMR/USDC 无限链式反向网格交易机器人。支持 **Kraken 现货** 和 **Hyperliquid 永续合约**（逐仓模式）。

用户手动下一笔限价单，成交后系统自动以固定价差挂出反向限价单，持续 ping-pong 振荡，直到手动取消为止。支持同时运行多个独立网格。

---

## 功能特性

- **Web 界面** — 下单表单、实时订单列表、状态推送（SSE）
- **Telegram 认证** — 基于 Telegram Bot 的无密码登录，JWT token（8 小时有效期）
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
| `JWT_SECRET` | JWT 签名密钥（请修改为随机字符串）| `change-me-in-production` |
| `SERVER_ADDR` | HTTP 监听地址 | `0.0.0.0:3000` |
| `DATABASE_URL` | SQLite 路径 | `sqlite:data/bot.db` |

---

## Web 界面认证

首次访问 Web 界面时，系统会显示一个 **6 位验证码**。

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

## Telegram Bot 命令

| 命令 | 说明 |
|------|------|
| `/login <code>` | Web 界面登录验证（6 位验证码）|
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
buy 成交  →  sell 挂单，价格 = 挂单价格 + price_change
sell 成交 →  buy  挂单，价格 = 挂单价格 - price_change
```

对手单继承相同的 `price_change` 和 `leverage`，形成持续振荡的双向网格。

### 订单状态流

```
pending → open → partially_filled → filled | cancelled | failed
```

- `pending`：已写入 DB，尚未提交到交易所
- `open`：交易所已接受挂单
- `partially_filled`：WebSocket 报告部分成交，`filled_quantity` 实时更新
- `filled`：轮询确认完全成交，已自动挂出反向对手单
- `cancelled` / `failed`：终态

### WebSocket 与轮询的分工

- **WebSocket**（成交事件）：仅更新 `filled_quantity` 和状态 → `partially_filled`，**不挂对手单**
- **轮询**（每 60 秒/交易所）：查询最低挂卖单和最高挂买单是否完全成交 → 完全成交则挂对手单

即使 WebSocket 完全不工作，轮询也能保证网格正常运行。

---

## 停机恢复机制

程序重启时，轮询任务的首次 tick 立即执行，**主动检查**所有活跃订单（`open` + `partially_filled`）在停机期间的状态变化：

### 恢复流程

1. **加载活跃订单** — 从数据库查询所有 `status='open'` 或 `'partially_filled'` 的订单
2. **筛选候选订单** — 每个交易所筛出最低价卖单和最高价买单
3. **查询交易所状态** — 调用交易所 API 查询这两个候选订单的最新状态
4. **状态同步**：
   - **已成交** → 使用挂单价格触发链式反向下单（保持网格完整性）
   - **已取消** → 更新数据库并发送 Telegram 通知
   - **仍挂单** → 等待下一次轮询

### 重要说明

⚠️ **成交价格限制**

由于交易所状态 API 限制，轮询检测到完全成交时无法获取精确成交价格，系统会使用**挂单价格**作为成交价。反向订单价格基于挂单价格 ± `price_change` 计算，保证网格层级间距固定一致，不受实际成交价波动影响。

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

// TODO:
3. 所有挂单都是限价单，不用只做maker，可以是taker单，成交后立即挂反向单
4. 增加删除订单功能
5. CI自动push镜像
6. 统计功能：统计每笔订单的盈亏，提供总盈亏统计
