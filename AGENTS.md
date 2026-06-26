# Agent Instructions — Toad Grid Bot

XMR/USDC 无限链式反向网格交易机器人。Kraken 现货 + MEXC 现货 + Hyperliquid 永续合约（逐仓模式）。

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

## Module Status

All modules implemented. Core layers (do not modify unless fixing a confirmed bug):

| Module | Description |
|--------|-------------|
| `src/config.rs` | clap derive + `.env` + env var parsing |
| `src/main.rs` | Startup orchestration: `init_pool → adapters (ExchangeRegistry) → GridEngine → bot → Axum serve` |
| `src/bot/mod.rs` | teloxide 0.13; commands `/order` `/orders` `/cancel` `/login`; checks `allowed_telegram_user_id` in every handler |
| `src/frontend/` | React 19 + Vite + Tailwind CSS + Radix UI (shadcn/ui style) |
| `src/assets.rs` | rust-embed fallback (SPA → index.html) |
| `src/exchange/` | `ExchangeAdapter` trait + `ExchangeKind`/`ExchangeRegistry` + Kraken/Hyperliquid/MEXC adapters |
| `src/engine/` | GridEngine: WebSocket fill updates + polling-driven reverse orders; iterates `ExchangeRegistry` |
| `src/api/` | Axum 0.8 REST handlers + router |
| `src/sse/` | Server-Sent Events broadcast |
| `src/db/` | SQLite + sqlx, `sqlx::migrate!` auto-migration |
| `src/auth/` | JWT auth (HS256, 8h) + Telegram verification flow |

## Key Conventions

- Order status flow: `pending → open → partially_filled → filled | cancelled | failed`
- Always **write `pending` to DB first**, then call exchange, then upgrade to `open` or `failed`
- `leverage`: Spot (kraken/mexc_spot) always 1; Perp (hyperliquid) ≥ 1, inherited by counter-orders. Use `adapter.kind().effective_leverage(req)` instead of string checks
- **Adapter dispatch**: `ExchangeRegistry = HashMap<String, Arc<dyn ExchangeAdapter>>`; all callers (api/bot/engine) look up by exchange name, no hard-coded branches
- **Reverse order pricing**: based on `order.price ± price_change` (not filled_price), ensuring fixed grid spacing
- **Assisted mode**: `price_change == 0` means assisted order only — still submitted & tracked, but no reverse order is placed after fill
- **WebSocket vs Polling**: WebSocket only updates `filled_quantity` → `partially_filled`; polling (60s) detects full fills and places reverse orders. Grid works even if WebSocket is down.
- **Race condition protection**: `update_fill_progress()` and `mark_order_filled()` use `WHERE status IN ('open','partially_filled')` conditional updates
- Hyperliquid: use `hypersdk = { git = "..." }` (not crates.io); call `update_leverage(is_cross=false)` before every order
- `sqlx::query!` macros require `DATABASE_URL` env var at compile time — set it or use a `.env` file
- Frontend dev proxy: `vite.config.ts` already routes `/api` → `http://localhost:3000`

## Shutdown & Recovery

### Graceful Shutdown
- Uses `tokio_util::sync::CancellationToken` for coordinated shutdown
- Ctrl+C / SIGTERM triggers token cancellation
- All background tasks (GridEngine, exchange listeners, Telegram Bot) monitor the token
- Main waits up to 10 seconds for all tasks to complete

### Polling & Recovery
- `GridEngine::run()` starts a polling task (`tokio::time::interval`, 60s) whose first tick fires immediately
- Each tick calls `poll_exchange` for every exchange in the `ExchangeRegistry`
- `poll_exchange` queries active orders (`open` + `partially_filled`), picks the lowest sell + highest buy, checks exchange status
- **filled** → uses `order.price` as filled_price → triggers `handle_filled_order()` → creates reverse order
- **cancelled** → updates DB to `cancelled`, sends SSE + Telegram notification
- **open** → continues to next tick

## Environment Variables

```
TELEGRAM_BOT_TOKEN, ALLOWED_TELEGRAM_USER_ID   (required)
KRAKEN_API_KEY, KRAKEN_API_SECRET               (optional; empty = Kraken disabled)
MEXC_API_KEY, MEXC_API_SECRET                   (optional; empty = MEXC disabled)
HYPERLIQUID_PRIVATE_KEY, HYPERLIQUID_ACCOUNT_ADDRESS, HYPERLIQUID_TESTNET
                                                (optional; empty private key = Hyperliquid disabled)
JWT_SECRET (default: change-me-in-production)
DATABASE_URL (default: sqlite:data/bot.db), SERVER_ADDR (default: 0.0.0.0:3000)
```

### Exchange enablement
An exchange is enabled only when its API credentials are non-empty. Disabled
exchanges are never added to the `ExchangeRegistry`: no WebSocket subscription,
no polling, and `POST /api/orders` / Telegram `/order` for them returns an error.
`GET /api/price/:exchange` still returns quotes for any exchange (uses an
independent public ticker client, no credentials needed). The frontend fetches
`GET /api/exchanges` to render only enabled-exchange panels and filter options.

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
- **Public**: `/api/auth/request`, `/api/auth/wait/:code`, `/api/exchanges`, `/api/price/:exchange`, `/api/health`
- **Protected** (requires auth middleware):
  - `POST /api/orders` — create order
  - `GET /api/orders` — list orders (cursor pagination + filters)
  - `DELETE /api/orders/:id` — cancel open order (calls exchange API)
  - `DELETE /api/orders/:id/hard` — hard-delete terminal orders (filled/cancelled/failed only)
  - `GET /api/sse` — SSE event stream

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

When the user types `/graphify`, invoke the `skill` tool with `skill: "graphify"` before doing anything else.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
