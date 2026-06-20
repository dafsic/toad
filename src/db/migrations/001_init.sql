-- Initial schema: orders table with partial-fill tracking and multi-exchange support.
--
-- Exchanges: kraken (spot), hyperliquid (perp), mexc_spot (spot).
-- Status flow: pending → open → partially_filled → filled | cancelled | failed
--
-- NOTE: This is a fresh schema (destructive on re-run). Back up data/bot.db before
-- re-applying migrations.

CREATE TABLE IF NOT EXISTS orders (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange         TEXT    NOT NULL CHECK(exchange IN ('kraken', 'hyperliquid', 'mexc_spot')),
    symbol           TEXT    NOT NULL DEFAULT 'XMR/USDC',
    side             TEXT    NOT NULL CHECK(side IN ('buy', 'sell')),
    quantity         REAL    NOT NULL,
    price            REAL    NOT NULL,
    price_change     REAL    NOT NULL,
    -- Leverage: spot fixed at 1; perp user-specified (>=1).
    -- Reverse grid legs inherit the parent order's leverage.
    leverage         INTEGER NOT NULL DEFAULT 1 CHECK(leverage >= 1),
    is_auto          INTEGER NOT NULL DEFAULT 0,
    parent_order_id  INTEGER,
    exchange_order_id TEXT,
    -- Cumulative filled quantity (updated from WebSocket fill events).
    -- Equals `quantity` when fully filled.
    filled_quantity  REAL    NOT NULL DEFAULT 0,
    status           TEXT    NOT NULL DEFAULT 'pending'
                             CHECK(status IN ('pending', 'open', 'partially_filled', 'filled', 'cancelled', 'failed')),
    filled_price     REAL,
    created_at       TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_order_id) REFERENCES orders(id)
);

CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_parent ON orders(parent_order_id);
CREATE INDEX IF NOT EXISTS idx_orders_exchange ON orders(exchange);
