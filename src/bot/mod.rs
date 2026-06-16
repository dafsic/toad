use crate::config::Config;

/// 启动 Telegram Bot。
/// 所有 handler 在执行前校验消息来源的 user_id。
pub async fn start(config: std::sync::Arc<Config>, /* db, state */) -> anyhow::Result<()> {
    // TODO:
    // 1. 创建 Bot::new(config.telegram_bot_token)
    // 2. 注册 command handler：
    //    - /start       — 帮助信息
    //    - /order       — 交互式下单菜单
    //    - /orders      — 查看当前所有挂单
    //    - /cancel <id> — 取消指定挂单
    // 3. 所有 handler 首先调用 check_user_id()
    // 4. Dispatcher::builder(bot, ...).enable_ctrlc_handler().build().dispatch().await
    todo!()
}

/// 校验发送者 user_id 是否为授权用户。
fn check_user_id(user_id: u64, config: &Config) -> bool {
    user_id == config.allowed_telegram_user_id
}

/// 向授权用户发送通知（成交、取消、异常告警）。
pub async fn send_notification(_config: &Config, _message: &str) -> anyhow::Result<()> {
    // TODO: Bot::new(...).send_message(ChatId(allowed_user_id), message).await
    todo!()
}
