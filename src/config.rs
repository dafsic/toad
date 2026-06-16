/// 启动配置，通过 clap derive + env 从命令行参数或环境变量注入。
/// 所有敏感字段加载至此结构体后，后续不再从数据库读取。
#[derive(Debug, Clone)]
pub struct Config {
    // Telegram
    pub telegram_bot_token: String,
    pub allowed_telegram_user_id: u64,

    // Kraken
    pub kraken_api_key: String,
    pub kraken_api_secret: String,

    // Hyperliquid
    /// API 钱包的私钥（十六进制）
    pub hyperliquid_private_key: String,
    /// 主账户地址（API agent wallet 模式时须指定；普通私钥可留空）
    pub hyperliquid_account_address: String,
    /// 是否使用测试网（默认 false）
    pub hyperliquid_testnet: bool,

    // Server
    pub server_addr: String,

    // Database
    pub database_url: String,
}

impl Config {
    /// 解析命令行参数与环境变量，构建 Config。
    pub fn parse() -> Self {
        // TODO: 使用 clap derive 实现
        // - 所有字段支持 --flag 与 ENV_VAR 两种注入方式
        // - database_url 默认值为 "data/bot.db"
        // - server_addr 默认值为 "0.0.0.0:3000"
        // - hyperliquid_testnet 默认 false，可通过 HYPERLIQUID_TESTNET=true 或 --hl-testnet 开启
        // - hyperliquid_account_address 默认空字符串（普通钱包模式）
        todo!()
    }
}
