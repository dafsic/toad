use axum::{extract::{Path, Query, State}, Json};
use serde::{Deserialize, Serialize};
use crate::api::AppState;

// ── 请求 / 响应结构体 ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub exchange: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub price_change: f64,
    /// 杠杆倍数。Kraken 现货固定为 1，Hyperliquid 永续合约由用户指定（≥1）。
    /// 若未传入，默认值为 1。
    #[serde(default = "default_leverage")]
    pub leverage: u32,
}

fn default_leverage() -> u32 { 1 }

#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub exchange: Option<String>,
    pub side: Option<String>,
    pub status: Option<String>,
    pub is_auto: Option<bool>,
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
    pub leverage: u32,
    pub is_auto: bool,
    pub parent_order_id: Option<i64>,
    pub exchange_order_id: Option<String>,
    pub status: String,
    pub filled_price: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/orders — 手动下单
pub async fn create_order(
    State(_state): State<AppState>,
    Json(_req): Json<CreateOrderRequest>,
) -> Json<OrderResponse> {
    // TODO:
    // 1. 验证参数（exchange 合法、quantity/price/price_change > 0、leverage >= 1）
    // 2. Kraken 下单时强制 leverage = 1
    // 3. 调用对应 ExchangeAdapter::place_limit_order
    // 4. 将订单写入数据库（status='open', is_auto=0, leverage=req.leverage）
    // 5. 通过 SSE 推送新订单事件
    todo!()
}

/// GET /api/orders — 查询挂单列表（支持筛选）
pub async fn list_orders(
    State(_state): State<AppState>,
    Query(_query): Query<ListOrdersQuery>,
) -> Json<Vec<OrderResponse>> {
    // TODO: 按条件 SELECT FROM orders
    todo!()
}

/// DELETE /api/orders/:id — 取消挂单
pub async fn cancel_order(
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> axum::http::StatusCode {
    // TODO:
    // 1. 查询订单，确认 status='open'
    // 2. 调用 ExchangeAdapter::cancel_order
    // 3. 更新数据库 status='cancelled'
    // 4. 通过 SSE 推送取消事件
    todo!()
}
