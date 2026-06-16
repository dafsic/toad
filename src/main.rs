mod api;
mod assets;
mod bot;
mod config;
mod db;
mod engine;
mod exchange;
mod sse;

use std::sync::Arc;

use anyhow::Context;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化 tracing（RUST_LOG 控制级别，默认 info）
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 2. 解析配置（clap + .env + 环境变量）
    let config = Arc::new(config::Config::parse());
    tracing::info!(
        server_addr = %config.server_addr,
        hl_testnet  = config.hyperliquid_testnet,
        "toad starting"
    );

    // 3. 初始化数据库连接池并运行 migrations
    let pool = db::init_pool(&config.database_url)
        .await
        .context("init db")?;

    // 4. 构建交易所适配器
    let kraken: Arc<dyn exchange::ExchangeAdapter> = Arc::new(
        exchange::kraken::KrakenAdapter::new(
            config.kraken_api_key.clone(),
            config.kraken_api_secret.clone(),
        ),
    );

    let hyperliquid: Arc<dyn exchange::ExchangeAdapter> = Arc::new(
        exchange::hyperliquid::HyperliquidAdapter::new(
            &config.hyperliquid_private_key,
            &config.hyperliquid_account_address,
            config.hyperliquid_testnet,
        )
        .await
        .context("init hyperliquid adapter")?,
    );

    // 5. 创建 SSE broadcast channel
    let sse_tx = sse::create_channel(128);

    // 6. 构建共享 AppState
    let state = Arc::new(api::AppState {
        config: Arc::clone(&config),
        db: pool.clone(),
        kraken: Arc::clone(&kraken),
        hyperliquid: Arc::clone(&hyperliquid),
        sse_tx: sse_tx.clone(),
    });

    // 7. 启动 Grid Engine（独立 task，内部自动重连并恢复 open 订单）
    {
        let engine = engine::GridEngine::new(
            pool.clone(),
            Arc::clone(&kraken),
            Arc::clone(&hyperliquid),
            sse_tx.clone(),
            Arc::clone(&config),
        );
        tokio::spawn(async move {
            if let Err(e) = engine.run().await {
                tracing::error!("grid engine exited: {e:#}");
            }
        });
    }

    // 8. 启动 Telegram Bot（独立 task）
    {
        let bot_state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = bot::start(bot_state).await {
                tracing::error!("telegram bot exited: {e:#}");
            }
        });
    }

    // 9. 启动 Axum HTTP 服务器（阻塞直到进程退出）
    let addr: std::net::SocketAddr = config
        .server_addr
        .parse()
        .context("invalid SERVER_ADDR")?;

    let router = api::router((*state).clone());

    tracing::info!(%addr, "http server listening");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("bind")?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum serve")?;

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

