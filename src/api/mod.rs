use std::sync::Arc;

use axum::{
    http::Method,
    routing::{delete, get, post},
    Router,
};
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

use crate::config::Config;
use crate::exchange::ExchangeAdapter;
use crate::sse::SseSender;

pub mod handlers;

/// 共享的应用状态，注入至所有 Axum handler。
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    pub kraken: Arc<dyn ExchangeAdapter>,
    pub hyperliquid: Arc<dyn ExchangeAdapter>,
    /// SSE 广播发送端，handler 和引擎均通过此推送事件
    pub sse_tx: SseSender,
}

/// 构建 Axum 路由树。
///
/// - `/api/*`  REST JSON 接口
/// - `/api/sse` SSE 实时推送
/// - `/*`       rust-embed 托管的前端静态资源
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/orders", post(handlers::create_order))
        .route("/orders", get(handlers::list_orders))
        .route("/orders/{id}", delete(handlers::cancel_order))
        .route("/sse", get(crate::sse::sse_handler));

    // 开发阶段允许前端 dev server（localhost:5173）跨域访问
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any)
        .allow_origin(Any);

    Router::new()
        .nest("/api", api)
        .fallback(crate::assets::static_handler)
        .layer(cors)
        .with_state(state)
}
