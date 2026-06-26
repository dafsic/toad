mod api;
mod assets;
mod auth;
mod bot;
mod config;
mod db;
mod engine;
mod exchange;
mod sse;

use std::sync::Arc;

use anyhow::Context;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 0. 在任何 TLS 连接建立前安装 rustls crypto provider
    //    Clash TUN 等代理工具会同时激活 aws-lc-rs 和 ring，导致 rustls 无法自动选择
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // 1. 初始化 tracing（RUST_LOG 控制级别，默认 info）
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 2. 解析配置（clap + .env + 环境变量）
    let config = Arc::new(config::Config::parse());

    // Security check: refuse to start with the default JWT secret in release builds.
    if config.jwt_secret == "change-me-in-production" {
        #[cfg(not(debug_assertions))]
        {
            panic!("JWT_SECRET is set to the insecure default. Set JWT_SECRET to a long random string before deploying.");
        }
        #[cfg(debug_assertions)]
        {
            tracing::warn!("JWT_SECRET is set to the insecure default — acceptable in debug builds only");
        }
    }

    tracing::info!(
        server_addr = %config.server_addr,
        hl_testnet  = config.hyperliquid_testnet,
        "toad starting"
    );

    // 3. 初始化数据库连接池并运行 migrations
    let pool = db::init_pool(&config.database_url)
        .await
        .context("init db")?;

    // 4. 构建交易所适配器，组装成统一注册表。
    //    各交易所的 API key/secret 为空时跳过构建，即不启用该交易所的交易与监听。
    let mut adapters = exchange::ExchangeRegistry::new();

    if !config.kraken_api_key.is_empty() && !config.kraken_api_secret.is_empty() {
        let kraken: Arc<dyn exchange::ExchangeAdapter> =
            Arc::new(exchange::kraken::KrakenAdapter::new(
                config.kraken_api_key.clone(),
                config.kraken_api_secret.clone(),
            ));
        adapters.insert("kraken".to_string(), kraken);
        tracing::info!("kraken enabled");
    } else {
        tracing::info!("kraken disabled (empty KRAKEN_API_KEY / KRAKEN_API_SECRET)");
    }

    if !config.hyperliquid_private_key.is_empty() {
        let hyperliquid: Arc<dyn exchange::ExchangeAdapter> = Arc::new(
            exchange::hyperliquid::HyperliquidAdapter::new(
                &config.hyperliquid_private_key,
                &config.hyperliquid_account_address,
                config.hyperliquid_testnet,
            )
            .await
            .context("init hyperliquid adapter")?,
        );
        adapters.insert("hyperliquid".to_string(), hyperliquid);
        tracing::info!("hyperliquid enabled");
    } else {
        tracing::info!("hyperliquid disabled (empty HYPERLIQUID_PRIVATE_KEY)");
    }

    if !config.mexc_api_key.is_empty() && !config.mexc_api_secret.is_empty() {
        let mexc_spot: Arc<dyn exchange::ExchangeAdapter> =
            Arc::new(exchange::mexc::MexcSpotAdapter::new(
                config.mexc_api_key.clone(),
                config.mexc_api_secret.clone(),
            ));
        adapters.insert("mexc_spot".to_string(), mexc_spot);
        tracing::info!("mexc_spot enabled");
    } else {
        tracing::info!("mexc_spot disabled (empty MEXC_API_KEY / MEXC_API_SECRET)");
    }

    if adapters.is_empty() {
        tracing::warn!("no exchanges enabled — only Telegram/price-quote/web UI will work");
    }
    let adapters = Arc::new(adapters);

    // 5. 创建全局关闭信号 token
    let shutdown_token = CancellationToken::new();

    // 6. 创建 SSE broadcast channel
    let sse_tx = sse::create_channel(128);

    // 7. 创建认证会话存储并启动清理任务
    let auth_store = auth::create_store();
    {
        let store = Arc::clone(&auth_store);
        tokio::spawn(async move {
            auth::cleanup_expired_sessions(store).await;
        });
    }

    // 8. 构建共享 AppState
    let state = Arc::new(api::AppState {
        config: Arc::clone(&config),
        db: pool.clone(),
        adapters: Arc::clone(&adapters),
        sse_tx: sse_tx.clone(),
        auth_store,
    });

    // 9. 启动 Grid Engine（独立 task，内部自动重连并恢复 open 订单）
    let engine_handle = {
        let engine = engine::GridEngine::new(
            pool.clone(),
            Arc::clone(&adapters),
            sse_tx.clone(),
            Arc::clone(&config),
        );
        let token = shutdown_token.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.run(token).await {
                tracing::error!("grid engine exited: {e:#}");
            }
        })
    };

    // 10. 启动 Telegram Bot（独立 task）
    let bot_handle = {
        let bot_state = Arc::clone(&state);
        let token = shutdown_token.clone();
        tokio::spawn(async move {
            if let Err(e) = bot::start(bot_state, token).await {
                tracing::error!("telegram bot exited: {e:#}");
            }
        })
    };

    // 11. 启动 Axum HTTP 服务器（阻塞直到收到关闭信号）
    let addr: std::net::SocketAddr = config.server_addr.parse().context("invalid SERVER_ADDR")?;

    let router = api::router((*state).clone());

    tracing::info!(%addr, "http server listening");
    let listener = tokio::net::TcpListener::bind(addr).await.context("bind")?;

    // 启动 HTTP 服务器（graceful shutdown 跟随全局 CancellationToken）
    let server_handle = {
        let token = shutdown_token.clone();
        tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move { token.cancelled().await })
                .await
        })
    };

    // 监听系统信号，立即触发全局关闭（不等 HTTP 连接排空）
    {
        let token = shutdown_token.clone();
        tokio::spawn(async move {
            shutdown_signal().await;
            token.cancel();
        });
    }

    // 阻塞直到收到关闭信号（Ctrl+C / SIGTERM）
    shutdown_token.cancelled().await;
    tracing::info!("triggering shutdown for all background tasks");

    // 等待所有后台任务完成（最多 10 秒）
    let wait_timeout = tokio::time::Duration::from_secs(10);
    let _ = tokio::time::timeout(wait_timeout, async {
        let _ = tokio::join!(engine_handle, bot_handle, server_handle);
    })
    .await;

    tracing::info!("shutdown complete");
    Ok(())
}

/// 监听 SIGINT（Ctrl+C）和 SIGTERM（kill / Docker stop），任一触发即返回。
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler")
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("received Ctrl+C, shutting down") },
        _ = terminate => { tracing::info!("received SIGTERM, shutting down") },
    }
}
