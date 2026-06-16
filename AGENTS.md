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
- Order status flow: `pending → open → filled | cancelled | failed`
- Always **write `pending` to DB first**, then call exchange, then upgrade to `open` or `failed`
- `leverage`: Kraken always 1; Hyperliquid ≥ 1, inherited by counter-orders
- Hyperliquid: use `hypersdk = { git = "..." }` (not crates.io); call `update_leverage(is_cross=false)` before every order
- `sqlx::query!` macros require `DATABASE_URL` env var at compile time — set it or use a `.env` file
- Frontend dev proxy: `vite.config.ts` already routes `/api` → `http://localhost:3000`

## Environment Variables

```
TELEGRAM_BOT_TOKEN, ALLOWED_TELEGRAM_USER_ID
KRAKEN_API_KEY, KRAKEN_API_SECRET
HYPERLIQUID_PRIVATE_KEY, HYPERLIQUID_ACCOUNT_ADDRESS, HYPERLIQUID_TESTNET
DATABASE_URL (default: sqlite:data/bot.db), SERVER_ADDR (default: 0.0.0.0:3000)
```

