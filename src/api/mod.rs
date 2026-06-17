use std::sync::Arc;

use axum::{
    http::Method,
    middleware,
    routing::{delete, get, post},
    Router,
};
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

use crate::config::Config;
use crate::exchange::ExchangeAdapter;
use crate::sse::SseSender;
use crate::auth::AuthStore;

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
    /// 认证会话存储（验证码 → 会话）
    pub auth_store: AuthStore,
}

/// 构建 Axum 路由树。
///
/// - `/api/*`  REST JSON 接口（需要认证）
/// - `/api/sse` SSE 实时推送（需要认证）
/// - `/api/auth/*` 认证端点（无需认证）
/// - `/*`       rust-embed 托管的前端静态资源
pub fn router(state: AppState) -> Router {
    // 认证路由（无需认证）
    let auth_routes = Router::new()
        .route("/request", post(crate::auth::handlers::request_login))
        .route("/wait/:code", get(crate::auth::handlers::wait_login));

    // API 路由（需要认证）
    let protected_api = Router::new()
        .route("/orders", post(handlers::create_order))
        .route("/orders", get(handlers::list_orders))
        .route("/orders/{id}", delete(handlers::cancel_order))
        .route("/sse", get(crate::sse::sse_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::middleware::auth_middleware,
        ));

    // 开发阶段允许前端 dev server（localhost:5173）跨域访问
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any)
        .allow_origin(Any)
        .allow_credentials(true); // 允许 cookie

    Router::new()
        .nest("/api/auth", auth_routes)
        .nest("/api", protected_api)
        .fallback(crate::assets::static_handler)
        .layer(cors)
        .with_state(state)
}
