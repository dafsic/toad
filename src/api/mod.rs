use std::sync::Arc;

use axum::{
    http::{header, Method},
    middleware,
    routing::{delete, get, post},
    Router,
};
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

use crate::auth::AuthStore;
use crate::config::Config;
use crate::exchange::ExchangeRegistry;
use crate::sse::SseSender;

pub mod handlers;

/// 共享的应用状态，注入至所有 Axum handler。
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: SqlitePool,
    /// 交易所适配器注册表（kraken / hyperliquid / mexc_spot）。
    /// 由 `main.rs` 在启动时组装，取代原来散落的显式字段。
    pub adapters: Arc<ExchangeRegistry>,
    /// SSE 广播发送端，handler 和引擎均通过此推送事件
    pub sse_tx: SseSender,
    /// 认证会话存储（验证码 → 会话）
    pub auth_store: AuthStore,
}

/// 构建 Axum 路由树。
///
/// - `/api/price/:exchange` 公开行情查询（无需认证，后端代理避免 CORS）
/// - `/api/auth/*` 认证端点（无需认证）
/// - `/api/*`  REST JSON 接口（需要认证）
/// - `/api/sse` SSE 实时推送（需要认证）
/// - `/*`       rust-embed 托管的前端静态资源
pub fn router(state: AppState) -> Router {
    // 公开行情路由（无需认证）
    let public_api = Router::new()
        .route("/health", get(handlers::health))
        .route("/{exchange}", get(handlers::get_price));

    // 认证路由（无需认证）
    let auth_routes = Router::new()
        .route("/request", post(crate::auth::handlers::request_login))
        .route("/wait/{code}", get(crate::auth::handlers::wait_login))
        .route("/complete/{code}", post(crate::auth::handlers::complete_login));

    // API 路由（需要认证）
    let protected_api = Router::new()
        .route("/orders", post(handlers::create_order))
        .route("/orders", get(handlers::list_orders))
        .route("/orders/{id}", delete(handlers::cancel_order))
        .route("/orders/{id}/hard", delete(handlers::delete_order))
        .route("/sse", get(crate::sse::sse_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::middleware::auth_middleware,
        ));

    // Permissive CORS (frontend is embedded; dev server also hits via proxy).
    // For production behind a reverse proxy you may want to restrict origins.
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::COOKIE, header::AUTHORIZATION])
        .allow_origin(Any)
        .allow_credentials(false);

    Router::new()
        .route("/api/exchanges", get(handlers::list_exchanges))
        .nest("/api/price", public_api)
        .nest("/api/auth", auth_routes)
        .nest("/api", protected_api)
        .fallback(crate::assets::static_handler)
        .layer(cors)
        .with_state(state)
}
