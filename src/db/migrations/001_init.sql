-- Migration 001: 初始化订单表

CREATE TABLE IF NOT EXISTS orders (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    exchange         TEXT    NOT NULL CHECK(exchange IN ('kraken', 'hyperliquid')),
    symbol           TEXT    NOT NULL DEFAULT 'XMR/USDC',
    side             TEXT    NOT NULL CHECK(side IN ('buy', 'sell')),
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
                             CHECK(status IN ('pending', 'open', 'filled', 'cancelled', 'failed')),
    filled_price     REAL,
    created_at       TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_order_id) REFERENCES orders(id)
);

CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_parent ON orders(parent_order_id);
CREATE INDEX IF NOT EXISTS idx_orders_exchange ON orders(exchange);
