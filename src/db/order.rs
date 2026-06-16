use anyhow::Context;
use sqlx::SqlitePool;

// ── 数据模型 ──────────────────────────────────────────────────────────────────

/// 数据库 orders 表的完整行映射。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Order {
    pub id: i64,
    pub exchange: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub price_change: f64,
    pub leverage: i64,
    pub is_auto: i64,               // SQLite 用 INTEGER 存 bool
    pub parent_order_id: Option<i64>,
    pub exchange_order_id: Option<String>,
    pub status: String,
    pub filled_price: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

// ── 写操作参数 ────────────────────────────────────────────────────────────────

/// 插入新订单的参数。
pub struct CreateOrder<'a> {
    pub exchange: &'a str,
    pub symbol: &'a str,
    pub side: &'a str,
    pub quantity: f64,
    pub price: f64,
    pub price_change: f64,
    pub leverage: u32,
    pub is_auto: bool,
    pub parent_order_id: Option<i64>,
    pub exchange_order_id: Option<&'a str>,
    /// 初始状态，通常为 `"pending"` 或 `"open"`
    pub status: &'a str,
}

/// 标记订单完全成交的参数。
pub struct UpdateOrderFilled {
    pub id: i64,
    pub filled_price: f64,
}

/// 通用状态更新（取消、失败等）。
pub struct UpdateOrderStatus<'a> {
    pub id: i64,
    pub status: &'a str,
}

// ── 查询过滤 ──────────────────────────────────────────────────────────────────

/// `list_orders` 的可选过滤条件，所有字段均为 None 时返回全部。
#[derive(Debug, Default)]
pub struct OrderFilter<'a> {
    pub exchange: Option<&'a str>,
    pub side: Option<&'a str>,
    pub status: Option<&'a str>,
    pub is_auto: Option<bool>,
}

// ── CRUD 函数 ─────────────────────────────────────────────────────────────────

/// 插入一条新订单，返回自增 `id`。
pub async fn insert_order(pool: &SqlitePool, p: &CreateOrder<'_>) -> anyhow::Result<i64> {
    let is_auto = p.is_auto as i64;
    let leverage = p.leverage as i64;
    let id = sqlx::query!(
        r#"
        INSERT INTO orders
            (exchange, symbol, side, quantity, price, price_change, leverage,
             is_auto, parent_order_id, exchange_order_id, status)
        VALUES
            (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        p.exchange,
        p.symbol,
        p.side,
        p.quantity,
        p.price,
        p.price_change,
        leverage,
        is_auto,
        p.parent_order_id,
        p.exchange_order_id,
        p.status,
    )
    .execute(pool)
    .await
    .context("insert_order")?
    .last_insert_rowid();

    Ok(id)
}

/// 按 `id` 查询单条订单，不存在时返回 `None`。
pub async fn get_order(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<Order>> {
    let row = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await
    .context("get_order")?;
    Ok(row)
}

/// 按 `exchange_order_id` 查询订单（成交事件匹配用）。
/// 返回状态为 `open` 的第一条匹配，通常唯一。
pub async fn get_order_by_exchange_id(
    pool: &SqlitePool,
    exchange_order_id: &str,
) -> anyhow::Result<Option<Order>> {
    let row = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE exchange_order_id = ? AND status = 'open' LIMIT 1",
        exchange_order_id
    )
    .fetch_optional(pool)
    .await
    .context("get_order_by_exchange_id")?;
    Ok(row)
}

/// 查询所有 status = 'open' 的订单（引擎重启恢复用）。
pub async fn list_open_orders(pool: &SqlitePool) -> anyhow::Result<Vec<Order>> {
    let rows = sqlx::query_as!(Order, "SELECT * FROM orders WHERE status = 'open'")
        .fetch_all(pool)
        .await
        .context("list_open_orders")?;
    Ok(rows)
}

/// 带过滤条件的订单列表查询（API 用）。
///
/// sqlx 的 `query_as!` 不支持动态 WHERE，这里使用 `QueryBuilder` 拼接。
pub async fn list_orders(pool: &SqlitePool, filter: &OrderFilter<'_>) -> anyhow::Result<Vec<Order>> {
    let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT * FROM orders WHERE 1=1",
    );

    if let Some(exchange) = filter.exchange {
        qb.push(" AND exchange = ").push_bind(exchange);
    }
    if let Some(side) = filter.side {
        qb.push(" AND side = ").push_bind(side);
    }
    if let Some(status) = filter.status {
        qb.push(" AND status = ").push_bind(status);
    }
    if let Some(is_auto) = filter.is_auto {
        let v = is_auto as i64;
        qb.push(" AND is_auto = ").push_bind(v);
    }

    qb.push(" ORDER BY id DESC");

    let rows = qb
        .build_query_as::<Order>()
        .fetch_all(pool)
        .await
        .context("list_orders")?;
    Ok(rows)
}

/// 将订单标记为已成交（filled），更新 filled_price 与 updated_at。
pub async fn mark_order_filled(pool: &SqlitePool, p: &UpdateOrderFilled) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        UPDATE orders
        SET status = 'filled', filled_price = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
        p.filled_price,
        p.id,
    )
    .execute(pool)
    .await
    .context("mark_order_filled")?;
    Ok(())
}

/// 更新订单状态（取消、失败等通用变更）。
pub async fn set_order_status(pool: &SqlitePool, p: &UpdateOrderStatus<'_>) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE orders SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        p.status,
        p.id,
    )
    .execute(pool)
    .await
    .context("set_order_status")?;
    Ok(())
}

/// 回填交易所分配的订单 ID（下单成功后更新）。
pub async fn set_exchange_order_id(
    pool: &SqlitePool,
    id: i64,
    exchange_order_id: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE orders SET exchange_order_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        exchange_order_id,
        id,
    )
    .execute(pool)
    .await
    .context("set_exchange_order_id")?;
    Ok(())
}
