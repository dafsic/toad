# Agent Instructions — Toad Grid Bot

XMR/USDC 无限链式反向网格交易机器人。Kraken 现货 + Hyperliquid 永续合约（逐仓模式）。

## Build Commands

| Task | Command |
|------|---------|
| Check (fast) | `cargo check` |
| Lint | `cargo clippy` |
| Build | `npm run build --prefix frontend && cargo build --release` |
| Run | `DATABASE_URL=sqlite:data/bot.db cargo run` |

## Commit Attribution

AI commits MUST include:
```
Co-Authored-By: Claude Sonnet 4.6 <claude-sonnet-4-6@anthropic.com>
```

## Pending Modules (priority order)

1. `src/config.rs` — implement `Config::parse()` via clap derive + env
2. `src/main.rs` — wire: `init_pool → adapters → GridEngine → bot → Axum serve`
3. `src/bot/mod.rs` — teloxide; commands `/order` `/orders` `/cancel <id>`; check `allowed_telegram_user_id` in every handler
4. `frontend/` — React 19 + Vite + shadcn/ui; see `skills/backend-api/SKILL.md` for API contract
5. `src/assets.rs` — rust-embed fallback (SPA → index.html)

## Key Conventions

- **Do not modify** completed modules (`exchange/`, `engine/`, `api/`, `sse/`, `db/`) unless fixing a confirmed bug
- Order status flow: `pending → open → partially_filled → filled | cancelled | failed`
- Always **write `pending` to DB first**, then call exchange, then upgrade to `open` or `failed`
- `leverage`: Kraken always 1; Hyperliquid ≥ 1, inherited by counter-orders
- Hyperliquid: use `hypersdk = { git = "..." }` (not crates.io); call `update_leverage(is_cross=false)` before every order
- `sqlx::query!` macros require `DATABASE_URL` env var at compile time — set it or use a `.env` file
- Frontend dev proxy: `vite.config.ts` already routes `/api` → `http://localhost:3000`

## Shutdown & Recovery

### Graceful Shutdown
- Uses `tokio_util::sync::CancellationToken` for coordinated shutdown
- Ctrl+C / SIGTERM triggers token cancellation
- All background tasks (GridEngine, exchange listeners, Telegram Bot) monitor the token
- Main waits up to 10 seconds for all tasks to complete

### Startup Recovery (Downtime Order Sync)
- `GridEngine::run()` starts a polling task (`tokio::time::interval`, 60s) whose first tick fires immediately
- Each tick calls `poll_exchange("kraken")` + `poll_exchange("hyperliquid")`
- `poll_exchange` queries active orders (`open` + `partially_filled`), picks the lowest sell + highest buy, checks exchange status
- **filled** → uses `order.price` as filled_price → triggers `handle_filled_order()` → creates reverse order
- **cancelled** → updates DB to `cancelled`, sends SSE + Telegram notification
- **open** → continues to next tick

## Environment Variables

```
TELEGRAM_BOT_TOKEN, ALLOWED_TELEGRAM_USER_ID
KRAKEN_API_KEY, KRAKEN_API_SECRET
HYPERLIQUID_PRIVATE_KEY, HYPERLIQUID_ACCOUNT_ADDRESS, HYPERLIQUID_TESTNET
JWT_SECRET (default: change-me-in-production)
DATABASE_URL (default: sqlite:data/bot.db), SERVER_ADDR (default: 0.0.0.0:3000)
```

## Authentication

### Flow
1. **Frontend** → POST `/api/auth/request` → get 6-digit code
2. **Frontend** → SSE `/api/auth/wait/:code` (long-polling for token)
3. **User** → sends `/login <code>` to Telegram Bot
4. **Bot** → verifies user_id → generates JWT → sends via oneshot channel to waiting SSE connection
5. **Frontend** → receives token via SSE → sets cookie `auth_token` → redirects to `/`

### JWT Structure
- Claims: `{ sub: user_id (u64), exp: timestamp, iat: timestamp }`
- Expiry: 8 hours from creation
- Signing: HS256 with `JWT_SECRET`

### Middleware
- `src/auth/middleware.rs` — `auth_middleware()` protects `/api/orders`, `/api/sse`
- Extracts token from `Cookie: auth_token=...`
- Validates signature and expiry
- Checks `claims.sub == config.allowed_telegram_user_id`
- Returns 401 Unauthorized or 403 Forbidden on failure

### Session Management
- `AuthStore: Arc<RwLock<HashMap<String, AuthSession>>>`
- Session lifecycle: created when frontend calls `/api/auth/request` → removed after 5 minutes or on successful login
- Cleanup task runs every 60 seconds, removes expired sessions (older than 5 minutes)
- Uses `tokio::sync::oneshot` channel to notify waiting SSE connection when Bot verifies

### Routes
- **Public**: `/api/auth/request`, `/api/auth/wait/:code`
- **Protected** (requires auth middleware): `/api/orders`, `/api/sse`


