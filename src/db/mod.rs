use sqlx::SqlitePool;

pub mod migrations {
    // sqlx::migrate!() 宏自动扫描 src/db/migrations/ 目录
}

/// 初始化 SQLite 连接池，并执行所有 migrations。
pub async fn init_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    // TODO:
    // 1. 确保数据库文件父目录存在
    // 2. SqlitePoolOptions::new().connect(database_url)
    // 3. sqlx::migrate!("src/db/migrations").run(&pool)
    todo!()
}
