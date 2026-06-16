use crate::config::Config;
use crate::exchange::ExchangeAdapter;
use axum::Router;
use sqlx::SqlitePool;
use std::sync::Arc;

pub mod handlers;

/// 共享的应用状态，注入至所有 Axum handler。
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    pub kraken: Arc<dyn ExchangeAdapter>,
    pub hyperliquid: Arc<dyn ExchangeAdapter>,
    // TODO: 添加 SSE broadcast sender
}

/// 构建 REST API 路由。
/// 所有路由挂载在 /api 前缀下。
pub fn router(state: AppState) -> Router {
    // TODO:
    // Router::new()
    //   .route("/api/orders",          post(handlers::create_order))
    //   .route("/api/orders",          get(handlers::list_orders))
    //   .route("/api/orders/:id",      delete(handlers::cancel_order))
    //   .route("/api/sse",             get(sse::sse_handler))
    //   .fallback(assets::static_handler)  // rust-embed 托管前端
    //   .with_state(state)
    todo!()
}
