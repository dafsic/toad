use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::db::order::{
    CreateOrder, OrderFilter, PageParams, UpdateOrderStatus,
    delete_order as delete_order_db, get_order, insert_order, list_orders_page,
    set_exchange_order_id, set_order_status,
};
use crate::exchange::OrderRequest;
use crate::sse::SseEvent;

// ── 请求 / 响应结构体 ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub exchange: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub price_change: f64,
    /// 杠杆倍数。Kraken 现货固定为 1，Hyperliquid ≥1。默认 1。
    #[serde(default = "default_leverage")]
    pub leverage: u32,
}

fn default_leverage() -> u32 {
    1
}

/// 游标分页 + 过滤查询参数。
#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub exchange: Option<String>,
    pub side: Option<String>,
    pub status: Option<String>,
    pub is_auto: Option<bool>,
    /// 上一页最后一条记录的 id，不传则从最新开始
    pub before_id: Option<i64>,
    /// 每页条数，默认 20，最大 100
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: i64,
    pub exchange: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub price_change: f64,
    pub leverage: i64,
    pub is_auto: bool,
    pub parent_order_id: Option<i64>,
    pub exchange_order_id: Option<String>,
    pub status: String,
    pub filled_price: Option<f64>,
    /// 累计已成交数量（由 WebSocket 实时更新）
    pub filled_quantity: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// 游标分页响应。
#[derive(Debug, Serialize)]
pub struct PageResponse {
    pub items: Vec<OrderResponse>,
    /// 下一页游标：取本页最后一条的 id。若为 null 表示已无更多数据。
    pub next_cursor: Option<i64>,
}

// ── 错误辅助 ──────────────────────────────────────────────────────────────────

fn internal(e: anyhow::Error) -> (StatusCode, String) {
    tracing::error!("{e:#}");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn bad_request(msg: &str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `POST /api/orders` — 手动下单
///
/// 流程：先写 pending 记录 → 向交易所提交 → 回填 exchange_order_id + status=open。
/// Kraken 现货强制 leverage=1。
pub async fn create_order(
    State(state): State<AppState>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), (StatusCode, String)> {
    // 参数校验
    let adapter = match state.adapters.get(&req.exchange) {
        Some(a) => a.clone(),
        None => {
            return Err(bad_request(&format!(
                "unsupported exchange '{}'",
                req.exchange
            )))
        }
    };
    if req.side != "buy" && req.side != "sell" {
        return Err(bad_request("side must be 'buy' or 'sell'"));
    }
    if req.quantity <= 0.0 {
        return Err(bad_request("quantity must be > 0"));
    }
    if req.price <= 0.0 {
        return Err(bad_request("price must be > 0"));
    }
    if req.price_change < 0.0 {
        return Err(bad_request("price_change must be >= 0 (0 = assisted, no reverse leg)"));
    }

    let leverage = adapter.kind().effective_leverage(req.leverage);

    // 1. 先写 pending 记录（防止崩溃丢单）
    let id = insert_order(&state.db, &CreateOrder {
        exchange:          &req.exchange,
        symbol:            "XMR/USDC",
        side:              &req.side,
        quantity:          req.quantity,
        price:             req.price,
        price_change:      req.price_change,
        leverage,
        is_auto:           false,
        parent_order_id:   None,
        exchange_order_id: None,
        status:            "pending",
    })
    .await
    .map_err(internal)?;

    // 2. 向交易所提交
    let confirmation = adapter
        .place_limit_order(&OrderRequest {
            symbol:   "XMR/USDC".to_string(),
            side:     req.side.clone(),
            quantity: req.quantity,
            price:    req.price,
            leverage,
        })
        .await;

    match confirmation {
        Ok(conf) => {
            set_exchange_order_id(&state.db, id, &conf.exchange_order_id)
                .await
                .map_err(internal)?;
            set_order_status(&state.db, &UpdateOrderStatus { id, status: "open" })
                .await
                .map_err(internal)?;
        }
        Err(e) => {
            let _ = set_order_status(&state.db, &UpdateOrderStatus { id, status: "failed" }).await;
            return Err((StatusCode::BAD_GATEWAY, format!("exchange error: {e:#}")));
        }
    }

    let order = get_order(&state.db, id)
        .await
        .map_err(internal)?
        .ok_or_else(|| internal(anyhow::anyhow!("order {id} disappeared after insert")))?;

    let _ = state.sse_tx.send(SseEvent::OrderCreated { order_id: id });

    Ok((StatusCode::CREATED, Json(order_to_response(order))))
}

/// `GET /api/orders` — 游标分页查询订单列表
///
/// 查询参数：
/// - `exchange`  过滤交易所（kraken / hyperliquid / mexc_spot）
/// - `side`      过滤方向（buy / sell）
/// - `status`    过滤状态（pending / open / filled / cancelled / failed）
/// - `is_auto`   过滤是否自动生成（true / false）
/// - `before_id` 游标：上一页最后一条的 id，不传从最新开始
/// - `limit`     每页条数，默认 20，最大 100
pub async fn list_orders(
    State(state): State<AppState>,
    Query(q): Query<ListOrdersQuery>,
) -> Result<Json<PageResponse>, (StatusCode, String)> {
    let filter = OrderFilter {
        exchange: q.exchange.as_deref(),
        side:     q.side.as_deref(),
        status:   q.status.as_deref(),
        is_auto:  q.is_auto,
    };
    let page = PageParams {
        before_id: q.before_id,
        limit:     q.limit,
    };

    let orders = list_orders_page(&state.db, &filter, &page)
        .await
        .map_err(internal)?;

    let next_cursor = if orders.len() as i64 == page.limit.clamp(1, 100) {
        orders.last().map(|o| o.id)
    } else {
        None
    };

    Ok(Json(PageResponse {
        items: orders.into_iter().map(order_to_response).collect(),
        next_cursor,
    }))
}

/// `DELETE /api/orders/:id` — 取消挂单
pub async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    let order = get_order(&state.db, id)
        .await
        .map_err(internal)?
        .ok_or((StatusCode::NOT_FOUND, format!("order {id} not found")))?;

    if order.status != "open" {
        return Err(bad_request(&format!(
            "order {id} is '{}', only 'open' orders can be cancelled",
            order.status
        )));
    }

    let exchange_order_id = order
        .exchange_order_id
        .as_deref()
        .ok_or_else(|| bad_request("order has no exchange_order_id yet"))?;

    let adapter = match state.adapters.get(&order.exchange) {
        Some(a) => a.clone(),
        None => {
            return Err(internal(anyhow::anyhow!(
                "adapter for exchange '{}' not registered",
                order.exchange
            )))
        }
    };

    adapter
        .cancel_order(exchange_order_id, &order.symbol)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("exchange error: {e:#}")))?;

    set_order_status(&state.db, &UpdateOrderStatus { id, status: "cancelled" })
        .await
        .map_err(internal)?;

    let _ = state.sse_tx.send(SseEvent::OrderUpdated {
        order_id: id,
        status: "cancelled".to_string(),
    });

    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/orders/:id/hard` — 硬删除终态订单
///
/// 仅允许删除 `filled` / `cancelled` / `failed` 状态的订单。
/// `open` / `partially_filled` / `pending` 订单不能删除（防止破坏活跃网格链）。
pub async fn delete_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    let order = get_order(&state.db, id)
        .await
        .map_err(internal)?
        .ok_or((StatusCode::NOT_FOUND, format!("order {id} not found")))?;

    // 仅允许终态订单删除
    let allowed = matches!(order.status.as_str(), "filled" | "cancelled" | "failed");
    if !allowed {
        return Err(bad_request(&format!(
            "order {id} is '{}', only filled/cancelled/failed orders can be deleted",
            order.status
        )));
    }

    let deleted = delete_order_db(&state.db, id)
        .await
        .map_err(internal)?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("order {id} not found")));
    }

    tracing::info!(id, status = order.status, "order deleted from db");
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/price/:exchange` — 查询交易所 XMR/USDC 最新价格（无需认证）。
///
/// 在后端代理外部 API 调用，避免浏览器直接访问交易所 API 时的 CORS 限制。
/// 返回 `{"price": "145.80"}`，价格保留 2 位小数。
pub async fn get_price(
    Path(exchange): Path<String>,
) -> Result<Json<PriceResponse>, (StatusCode, String)> {
    let client = reqwest::Client::new();
    let price = match exchange.as_str() {
        "kraken" => {
            #[derive(Deserialize)]
            struct KrakenTicker {
                result: serde_json::Value,
            }
            let resp: KrakenTicker = client
                .get("https://api.kraken.com/0/public/Ticker?pair=XMRUSDC")
                .send()
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("kraken ticker: {e:#}")))?
                .json()
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("kraken parse: {e:#}")))?;
            let p = resp.result["XMRUSDC"]["c"][0]
                .as_str()
                .ok_or((StatusCode::BAD_GATEWAY, "kraken: no price".to_string()))?;
            p.to_string()
        }
        "hyperliquid" => {
            let resp: serde_json::Value = client
                .post("https://api.hyperliquid.xyz/info")
                .header("Content-Type", "application/json")
                .body(r#"{"type":"allMids"}"#)
                .send()
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("hl ticker: {e:#}")))?
                .json()
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("hl parse: {e:#}")))?;
            resp["XMR"]
                .as_str()
                .ok_or((StatusCode::BAD_GATEWAY, "hyperliquid: no price".to_string()))?
                .to_string()
        }
        "mexc_spot" => {
            #[derive(Deserialize)]
            struct MexcTicker {
                price: String,
            }
            let resp: MexcTicker = client
                .get("https://api.mexc.com/api/v3/ticker/price?symbol=XMRUSDC")
                .send()
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("mexc ticker: {e:#}")))?
                .json()
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("mexc parse: {e:#}")))?;
            resp.price
        }
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unsupported exchange '{other}'"),
            ))
        }
    };

    let formatted = format!("{:.2}", price.parse::<f64>().unwrap_or(0.0));
    Ok(Json(PriceResponse { price: formatted }))
}

#[derive(Debug, Serialize)]
pub struct PriceResponse {
    pub price: String,
}

// ── 辅助转换 ──────────────────────────────────────────────────────────────────

fn order_to_response(o: crate::db::order::Order) -> OrderResponse {
    OrderResponse {
        id:                o.id,
        exchange:          o.exchange,
        symbol:            o.symbol,
        side:              o.side,
        quantity:          o.quantity,
        price:             o.price,
        price_change:      o.price_change,
        leverage:          o.leverage,
        is_auto:           o.is_auto != 0,
        parent_order_id:   o.parent_order_id,
        exchange_order_id: o.exchange_order_id,
        status:            o.status,
        filled_price:      o.filled_price,
        filled_quantity:   o.filled_quantity,
        created_at:        o.created_at,
        updated_at:        o.updated_at,
    }
}
