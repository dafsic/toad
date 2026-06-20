use std::sync::Arc;

use anyhow::Context;
use teloxide::{
    dispatching::{HandlerExt, UpdateFilterExt},
    prelude::*,
    types::{Message, ParseMode},
    utils::command::BotCommands,
};
use tokio_util::sync::CancellationToken;

use crate::api::AppState;
use crate::db::order::{
    cancel_order_db, get_order, list_orders_page, set_order_status, OrderFilter, PageParams,
    UpdateOrderStatus,
};
use crate::exchange::ExchangeAdapter;
use crate::sse::SseEvent;

// ── 命令定义 ──────────────────────────────────────────────────────────────────

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Toad Grid Bot commands:")]
enum Command {
    #[command(description = "Show help")]
    Start,
    #[command(
        description = "Place order: /order <exchange> <side> <qty> <price> <price_change> [leverage]"
    )]
    Order(String),
    #[command(description = "List orders: /orders [open|filled|cancelled|...]")]
    Orders(String),
    #[command(description = "Cancel order: /cancel <id>")]
    Cancel(String),
    #[command(description = "Login to web UI: /login <code>")]
    Login(String),
}

// ── 权限校验 ──────────────────────────────────────────────────────────────────

/// 返回消息来源的 user_id；group/channel 消息没有 from 字段时返回 0。
fn sender_id(msg: &Message) -> u64 {
    msg.from.as_ref().map_or(0, |u| u.id.0)
}

/// 快捷发送纯文本回复。
async fn reply(bot: &Bot, msg: &Message, text: &str) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

// ── 命令处理器 ────────────────────────────────────────────────────────────────

async fn handle_start(bot: Bot, msg: Message, state: Arc<AppState>) -> ResponseResult<()> {
    if sender_id(&msg) != state.config.allowed_telegram_user_id {
        return reply(&bot, &msg, "⛔ Unauthorized").await;
    }

    let help = Command::descriptions()
        .to_string()
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    reply(&bot, &msg, &format!("🐸 <b>Toad Grid Bot</b>\n\n{help}")).await
}

/// `/order kraken buy 2.5 145.80 1.50 [leverage]`
async fn handle_order(
    bot: Bot,
    msg: Message,
    args: String,
    state: Arc<AppState>,
) -> ResponseResult<()> {
    if sender_id(&msg) != state.config.allowed_telegram_user_id {
        return reply(&bot, &msg, "⛔ Unauthorized").await;
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 5 {
        return reply(
            &bot, &msg,
            "Usage: <code>/order &lt;exchange&gt; &lt;side&gt; &lt;qty&gt; &lt;price&gt; &lt;price_change&gt; [leverage]</code>\n\
             exchange: kraken / hyperliquid / mexc_spot\n\
             Example: <code>/order kraken buy 2.5 145.80 1.50</code>",
        ).await;
    }

    let exchange = parts[0];
    let side = parts[1];

    // 适配器查找 + 校验
    let adapter: Arc<dyn ExchangeAdapter> = match state.adapters.get(exchange) {
        Some(a) => Arc::clone(a),
        None => return reply(&bot, &msg, "❌ exchange must be kraken / hyperliquid / mexc_spot").await,
    };

    let quantity: f64 = match parts[2].parse() {
        Ok(v) => v,
        Err(_) => return reply(&bot, &msg, "❌ Invalid qty").await,
    };
    let price: f64 = match parts[3].parse() {
        Ok(v) => v,
        Err(_) => return reply(&bot, &msg, "❌ Invalid price").await,
    };
    let price_change: f64 = match parts[4].parse() {
        Ok(v) => v,
        Err(_) => return reply(&bot, &msg, "❌ Invalid price_change").await,
    };
    let leverage: u32 = if parts.len() >= 6 {
        match parts[5].parse() {
            Ok(v) => v,
            Err(_) => return reply(&bot, &msg, "❌ Invalid leverage").await,
        }
    } else {
        1
    };

    // basic validation (reject NaN/Infinity + negative/zero)
    if side != "buy" && side != "sell" {
        return reply(&bot, &msg, "❌ side must be buy or sell").await;
    }
    if !quantity.is_finite() || quantity <= 0.0 {
        return reply(&bot, &msg, "❌ qty must be a finite number > 0").await;
    }
    if !price.is_finite() || price <= 0.0 {
        return reply(&bot, &msg, "❌ price must be a finite number > 0").await;
    }
    if !price_change.is_finite() || price_change < 0.0 {
        return reply(&bot, &msg, "❌ price_change cannot be negative (0=assisted, no reverse leg)").await;
    }

    let effective_leverage = adapter.kind().effective_leverage(leverage);

    // Write pending first
    let db_id = match crate::db::order::insert_order(
        &state.db,
        &crate::db::order::CreateOrder {
            exchange,
            symbol: crate::exchange::TRADING_SYMBOL,
            side,
            quantity,
            price,
            price_change,
            leverage: effective_leverage,
            is_auto: false,
            parent_order_id: None,
            exchange_order_id: None,
            status: "pending",
        },
    )
    .await
    {
        Ok(id) => id,
        Err(e) => return reply(&bot, &msg, &format!("❌ Database error: {e:#}")).await,
    };

    // Submit to exchange
    match adapter
        .place_limit_order(&crate::exchange::OrderRequest {
            symbol: crate::exchange::TRADING_SYMBOL.to_string(),
            side: side.to_string(),
            quantity,
            price,
            leverage: effective_leverage,
        })
        .await
    {
        Ok(conf) => {
            let _ =
                crate::db::order::set_exchange_order_id(&state.db, db_id, &conf.exchange_order_id)
                    .await;
            let _ = set_order_status(
                &state.db,
                &UpdateOrderStatus {
                    id: db_id,
                    status: "open",
                },
            )
            .await;
            let _ = state
                .sse_tx
                .send(SseEvent::OrderCreated { order_id: db_id });
            reply(&bot, &msg, &format!(
                "✅ Order placed\nID: <code>{db_id}</code>\nExchange order: <code>{}</code>\n{exchange} {side} {quantity} @ {price}  Δ{price_change}  ×{effective_leverage}",
                conf.exchange_order_id
            )).await
        }
        Err(e) => {
            let _ = set_order_status(
                &state.db,
                &UpdateOrderStatus {
                    id: db_id,
                    status: "failed",
                },
            )
            .await;
            reply(&bot, &msg, &format!("❌ Exchange rejected: {e:#}")).await
        }
    }
}

/// `/orders [status]`  — status 默认为 open
async fn handle_orders(
    bot: Bot,
    msg: Message,
    args: String,
    state: Arc<AppState>,
) -> ResponseResult<()> {
    if sender_id(&msg) != state.config.allowed_telegram_user_id {
        return reply(&bot, &msg, "⛔ Unauthorized").await;
    }

    let status_filter = args.split_whitespace().next().unwrap_or("open");
    let filter = OrderFilter {
        status: Some(status_filter),
        ..Default::default()
    };
    let page = PageParams {
        before_id: None,
        limit: 20,
    };

    let orders = match list_orders_page(&state.db, &filter, &page).await {
        Ok(v) => v,
        Err(e) => return reply(&bot, &msg, &format!("❌ Query failed: {e:#}")).await,
    };

    if orders.is_empty() {
        return reply(&bot, &msg, &format!("📭 No {status_filter} orders")).await;
    }

    let lines: Vec<String> = orders
        .iter()
        .map(|o| {
            let auto_tag = if o.is_auto != 0 { " 🤖" } else { "" };
            let filled = o
                .filled_price
                .map_or(String::new(), |p| format!(" → {p:.4}"));
            // 部分成交时显示已成交数量
            let partial = if o.status == "partially_filled" && o.filled_quantity > 0.0 {
                format!(" ({:.4}/{:.4})", o.filled_quantity, o.quantity)
            } else {
                String::new()
            };
            format!(
                "<code>{:>4}</code>  {} {}  {:.4} @ {:.4}  Δ{:.4}  ×{}{}{}{}",
                o.id,
                o.exchange,
                o.side,
                o.quantity,
                o.price,
                o.price_change,
                o.leverage,
                filled,
                partial,
                auto_tag
            )
        })
        .collect();

    reply(
        &bot,
        &msg,
        &format!(
            "📋 <b>{status_filter} orders</b> (latest {})\n\n{}",
            orders.len(),
            lines.join("\n")
        ),
    )
    .await
}

/// `/cancel <id>`
async fn handle_cancel(
    bot: Bot,
    msg: Message,
    args: String,
    state: Arc<AppState>,
) -> ResponseResult<()> {
    if sender_id(&msg) != state.config.allowed_telegram_user_id {
        return reply(&bot, &msg, "⛔ Unauthorized").await;
    }

    let id: i64 = match args.trim().parse() {
        Ok(v) => v,
        Err(_) => return reply(&bot, &msg, "Usage: <code>/cancel &lt;id&gt;</code>").await,
    };

    let order = match get_order(&state.db, id).await {
        Ok(Some(o)) => o,
        Ok(None) => return reply(&bot, &msg, &format!("❌ Order {id} not found")).await,
        Err(e) => return reply(&bot, &msg, &format!("❌ Database error: {e:#}")).await,
    };

    if order.status != "open" && order.status != "partially_filled" {
        return reply(
            &bot,
            &msg,
            &format!("❌ Order {id} is '{}', cannot cancel", order.status),
        )
        .await;
    }

    let exchange_oid = match order.exchange_order_id.as_deref() {
        Some(s) => s.to_string(),
        None => return reply(&bot, &msg, "❌ Order has no exchange ID yet").await,
    };

    let adapter: Arc<dyn ExchangeAdapter> = match state.adapters.get(&order.exchange) {
        Some(a) => Arc::clone(a),
        None => return reply(&bot, &msg, &format!("❌ No adapter for {}", order.exchange)).await,
    };

    match adapter.cancel_order(&exchange_oid, &order.symbol).await {
        Ok(()) => {
            // Conditional UPDATE: only cancel if still open/partially_filled.
            // Prevents clobbering a fill that the engine detected concurrently.
            match cancel_order_db(&state.db, id).await {
                Ok(true) => {
                    let _ = state.sse_tx.send(SseEvent::OrderUpdated {
                        order_id: id,
                        status: "cancelled".into(),
                    });
                    reply(&bot, &msg, &format!("✅ Order <code>{id}</code> cancelled")).await
                }
                Ok(false) => {
                    reply(&bot, &msg, &format!("⚠️ Order {id} was filled during cancel, not cancelled")).await
                }
                Err(e) => {
                    tracing::error!("cancel_order_db failed: {e:#}");
                    reply(&bot, &msg, &format!("❌ Database error after exchange cancel: {e:#}")).await
                }
            }
        }
        Err(e) => reply(&bot, &msg, &format!("❌ Exchange cancel failed: {e:#}")).await,
    }
}

/// `/login <验证码>` — Web 界面登录验证
async fn handle_login(
    bot: Bot,
    msg: Message,
    args: String,
    state: Arc<AppState>,
) -> ResponseResult<()> {
    let user_id = sender_id(&msg);
    if user_id != state.config.allowed_telegram_user_id {
        return reply(&bot, &msg, "⛔ Unauthorized user").await;
    }

    let code = args.trim();
    if code.is_empty() {
        return reply(&bot, &msg, "Usage: <code>/login &lt;code&gt;</code>").await;
    }

    let mut store = state.auth_store.write().await;

    match store.get_mut(code) {
        Some(session) => {
            // 生成 JWT token
            let token = match crate::auth::generate_token(user_id, &state.config.jwt_secret) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("generate token failed: {e:#}");
                    return reply(&bot, &msg, "❌ Failed to generate token").await;
                }
            };

            session.user_id = Some(user_id);
            session.token = Some(token.clone());

            // notify the waiting SSE (send the token internally; it will be stored and a non-secret signal emitted)
            if let Some(tx) = session.tx.take() {
                let _ = tx.send(token);
            }

            tracing::info!(code, user_id, "login successful");
            reply(&bot, &msg, "✅ Login successful! You can now use the web UI.").await
        }
        None => reply(&bot, &msg, "❌ Invalid or expired code (valid for 5 minutes)").await,
    }
}

// ── 公共入口 ──────────────────────────────────────────────────────────────────

/// 启动 Telegram Bot（在独立 tokio task 中运行）。
pub async fn start(state: Arc<AppState>, shutdown_token: CancellationToken) -> anyhow::Result<()> {
    let bot = Bot::new(&state.config.telegram_bot_token);

    let handler = Update::filter_message()
        .branch(
            // 命令处理分支
            dptree::entry()
                .filter_command::<Command>()
                .branch(
                    dptree::case![Command::Start]
                        .endpoint(handle_start),
                )
                .branch(
                    dptree::case![Command::Order(args)]
                        .endpoint(handle_order),
                )
                .branch(
                    dptree::case![Command::Orders(args)]
                        .endpoint(handle_orders),
                )
                .branch(
                    dptree::case![Command::Cancel(args)]
                        .endpoint(handle_cancel),
                )
                .branch(
                    dptree::case![Command::Login(args)]
                        .endpoint(handle_login),
                ),
        )
        .branch(
            // 纯数字验证码（用户直接发送 6 位数字，不带 /login 前缀）
            Message::filter_text().endpoint(
                |bot: Bot, msg: Message, text: String, state: Arc<AppState>| async move {
                    let trimmed = text.trim();
                    if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                        handle_login(bot, msg, trimmed.to_string(), state).await
                    } else {
                        Ok(())
                    }
                },
            ),
        );

    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![state])
        .build();

    // 向 Telegram 注册命令菜单（显示在输入框左侧的 / 菜单）
    if let Err(e) = bot.set_my_commands(Command::bot_commands()).await {
        tracing::warn!("failed to set bot commands: {e:#}");
    }

    // 用 tokio::select! 监听关闭信号
    tokio::select! {
        _ = shutdown_token.cancelled() => {
            tracing::info!("telegram bot received shutdown signal");
        }
        _ = dispatcher.dispatch() => {
            tracing::warn!("telegram bot dispatcher exited unexpectedly");
        }
    }

    Ok(())
}

/// 向授权用户发送主动通知（成交、告警等）。
pub async fn send_notification(config: &crate::config::Config, text: &str) -> anyhow::Result<()> {
    let bot = Bot::new(&config.telegram_bot_token);
    let chat_id = ChatId(config.allowed_telegram_user_id as i64);
    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .await
        .context("telegram send_notification")?;
    Ok(())
}
