-- +migrate Up
SET LOCAL statement_timeout = '15s';

CREATE TABLE IF NOT EXISTS orders (
    id BIGINT GENERATED ALWAYS AS IDENTITY,
    order_id TEXT,
    exchange TEXT NOT NULL,
    bot TEXT NOT NULL,
    pair TEXT NOT NULL,
    side TEXT NOT NULL,
    price NUMERIC(20, 6) NOT NULL,
    amount NUMERIC(20, 6) NOT NULL,
    multiplier INT NOT NULL,
    order_status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(id)
);

CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_orders_order_id ON public.orders USING btree (order_id);

-- +migrate Down
SET LOCAL statement_timeout = '15s';

DROP UNIQUE INDEX CONCURRENTLY IF EXISTS idx_orders_order_id;

DROP TABLE IF EXISTS orders CASCADE;
