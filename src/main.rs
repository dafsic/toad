mod config;
mod db;
mod exchange;
mod engine;
mod api;
mod bot;
mod sse;
mod assets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO:
    // 1. 初始化 tracing subscriber
    // 2. 解析 Config（clap + 环境变量）
    // 3. 初始化数据库连接池并运行 migrations
    // 4. 构建 ExchangeAdapter（Kraken + Hyperliquid）
    // 5. 启动 Grid Engine（恢复活跃网格链路）
    // 6. 启动 Telegram Bot（独立 tokio task）
    // 7. 启动 Axum HTTP 服务器（REST API + SSE + 前端静态资源）
    todo!()
}
