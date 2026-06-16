use clap::Parser;

/// Toad Grid Bot — XMR/USDC 无限链式反向网格交易机器人。
///
/// 所有配置项均可通过环境变量或命令行参数传入，优先级：CLI 参数 > 环境变量 > 默认值。
#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct Config {
    // ── Telegram ────────────────────────────────────────────────────────────
    /// Telegram Bot Token
    #[arg(long, env = "TELEGRAM_BOT_TOKEN")]
    pub telegram_bot_token: String,

    /// 允许操作机器人的 Telegram User ID
    #[arg(long, env = "ALLOWED_TELEGRAM_USER_ID")]
    pub allowed_telegram_user_id: u64,

    // ── Kraken ──────────────────────────────────────────────────────────────
    /// Kraken API Key
    #[arg(long, env = "KRAKEN_API_KEY")]
    pub kraken_api_key: String,

    /// Kraken API Secret（Base64 编码）
    #[arg(long, env = "KRAKEN_API_SECRET")]
    pub kraken_api_secret: String,

    // ── Hyperliquid ─────────────────────────────────────────────────────────
    /// Hyperliquid API 钱包私钥（十六进制，可选 0x 前缀）
    #[arg(long, env = "HYPERLIQUID_PRIVATE_KEY")]
    pub hyperliquid_private_key: String,

    /// Hyperliquid 主账户地址（API agent wallet 模式时必填；普通钱包留空）
    #[arg(long, env = "HYPERLIQUID_ACCOUNT_ADDRESS", default_value = "")]
    pub hyperliquid_account_address: String,

    /// 连接 Hyperliquid 测试网
    #[arg(long, env = "HYPERLIQUID_TESTNET", default_value_t = false)]
    pub hyperliquid_testnet: bool,

    // ── Server ──────────────────────────────────────────────────────────────
    /// HTTP 监听地址
    #[arg(long, env = "SERVER_ADDR", default_value = "0.0.0.0:3000")]
    pub server_addr: String,

    // ── Database ────────────────────────────────────────────────────────────
    /// SQLite 数据库 URL
    #[arg(long, env = "DATABASE_URL", default_value = "sqlite:data/bot.db")]
    pub database_url: String,
}

impl Config {
    /// 从命令行参数和环境变量解析配置。
    /// 优先加载 `.env` 文件（若存在），再由 clap 解析。
    pub fn parse() -> Self {
        // 尽力加载 .env（文件不存在时静默跳过）
        let _ = dotenvy::dotenv();
        <Self as Parser>::parse()
    }
}
