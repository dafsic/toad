use anyhow::Context;
use sqlx::{sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions, SqlitePool};
use std::str::FromStr;

pub mod order;

/// 初始化 SQLite 连接池并执行所有 migrations。
///
/// `database_url` 格式：`sqlite:data/bot.db` 或裸路径 `data/bot.db`。
/// 若父目录不存在则自动创建。
pub async fn init_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    // 规范化 URL：sqlx 要求 "sqlite:" 前缀
    let url = if database_url.starts_with("sqlite:") {
        database_url.to_string()
    } else {
        format!("sqlite:{database_url}")
    };

    // 从 URL 中提取文件路径，确保父目录存在
    let file_path = url.trim_start_matches("sqlite:");
    if let Some(parent) = std::path::Path::new(file_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating db directory: {}", parent.display()))?;
        }
    }

    let opts = SqliteConnectOptions::from_str(&url)
        .context("invalid DATABASE_URL")?
        .create_if_missing(true)
        // WAL 模式：允许读写并发，减少锁竞争
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        // 写操作超时等待（并发写时最多等 5 秒）
        .busy_timeout(std::time::Duration::from_secs(5))
        // 外键约束默认关闭，需手动启用
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await
        .context("connecting to SQLite")?;

    // 运行所有 migrations
    sqlx::migrate!("src/db/migrations")
        .run(&pool)
        .await
        .context("running migrations")?;

    tracing::info!(db = %file_path, "database ready");
    Ok(pool)
}
