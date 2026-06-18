use std::sync::Arc;

use anyhow::Context;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::db::order::{
    get_order_by_exchange_id, insert_order, list_active_orders_by_exchange, mark_order_filled,
    set_exchange_order_id, set_order_status, update_fill_progress, CreateOrder, Order,
    UpdateOrderFilled, UpdateOrderStatus,
};
use crate::exchange::{ExchangeAdapter, FillEvent, OrderRequest};
use crate::sse::{SseEvent, SseSender};

/// fill channel 容量：允许两个交易所各自有一定积压
const FILL_CHANNEL_CAPACITY: usize = 256;
/// 轮询间隔：每 60 秒检查一次各交易所活跃订单的成交状态
const POLL_INTERVAL_SECS: u64 = 60;

/// 网格引擎：WebSocket 更新部分成交进度 + 轮询驱动链式反向挂单。
///
/// 架构：
/// - **WebSocket** 成交事件 → 仅更新 `filled_quantity` + 状态 → `partially_filled`
/// - **轮询**（每 60 秒/交易所）→ 查最低挂卖单和最高挂买单 → 完全成交则挂对手单
///
/// 即使 WebSocket 完全不工作，轮询也能保证网格正常运行。
///
/// 每笔完全成交后，按如下规则自动挂出反向限价单：
/// - buy  成交 → sell，价格 = `挂单价格 + price_change`
/// - sell 成交 → buy，  价格 = `挂单价格 - price_change`
///
/// 反向订单继承原订单的 `price_change` 和 `leverage`。
pub struct GridEngine {
    db: SqlitePool,
    kraken: Arc<dyn ExchangeAdapter>,
    hyperliquid: Arc<dyn ExchangeAdapter>,
    sse_tx: SseSender,
    config: Arc<Config>,
}

impl GridEngine {
    pub fn new(
        db: SqlitePool,
        kraken: Arc<dyn ExchangeAdapter>,
        hyperliquid: Arc<dyn ExchangeAdapter>,
        sse_tx: SseSender,
        config: Arc<Config>,
    ) -> Self {
        Self {
            db,
            kraken,
            hyperliquid,
            sse_tx,
            config,
        }
    }

    /// 选取对应交易所的适配器。
    fn adapter(&self, exchange: &str) -> Arc<dyn ExchangeAdapter> {
        if exchange == "hyperliquid" {
            Arc::clone(&self.hyperliquid)
        } else {
            Arc::clone(&self.kraken)
        }
    }

    /// 启动引擎（消耗 self，在独立 tokio task 中运行）。
    ///
    /// 1. 启动两个交易所的 `subscribe_fills`（WebSocket），汇入统一 fill channel。
    ///    WebSocket 事件**仅用于更新已成交数量**，不挂对手单。
    /// 2. 启动轮询 task：每 60 秒检查各交易所活跃订单（最低卖 + 最高买），
    ///    完全成交则挂对手单。首次 tick 立即执行，替代原启动恢复逻辑。
    /// 3. 主循环处理 FillEvent（仅更新 filled_quantity）。
    /// 4. 收到 shutdown_token 取消信号时优雅退出。
    pub async fn run(self, shutdown_token: CancellationToken) -> anyhow::Result<()> {
        let (fill_tx, mut fill_rx) = mpsc::channel::<FillEvent>(FILL_CHANNEL_CAPACITY);

        // 将 self 包在 Arc 中，以便多个 spawn 和主循环共享
        let engine = Arc::new(self);

        // ── 启动 Kraken 成交监听（WebSocket → 更新已成交数量）──────────────
        {
            let tx = fill_tx.clone();
            let adapter = Arc::clone(&engine.kraken);
            let token = shutdown_token.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = token.cancelled() => {
                            tracing::info!("kraken subscribe_fills received shutdown signal");
                            break;
                        }
                        result = adapter.subscribe_fills(tx.clone()) => {
                            if let Err(e) = result {
                                tracing::error!("kraken subscribe_fills error: {e:#}");
                            }
                            // 适配器内部已做重连；若任务意外退出，此处等待后重试
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            });
        }

        // ── 启动 Hyperliquid 成交监听（WebSocket → 更新已成交数量）─────────
        {
            let tx = fill_tx.clone();
            let adapter = Arc::clone(&engine.hyperliquid);
            let token = shutdown_token.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = token.cancelled() => {
                            tracing::info!("hyperliquid subscribe_fills received shutdown signal");
                            break;
                        }
                        result = adapter.subscribe_fills(tx.clone()) => {
                            if let Err(e) = result {
                                tracing::error!("hyperliquid subscribe_fills error: {e:#}");
                            }
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            });
        }

        // ── 启动轮询 task（每 60 秒检查活跃订单，驱动链式反向挂单）─────────
        {
            let eng = Arc::clone(&engine);
            let token = shutdown_token.clone();
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS));
                // interval 首次 tick 立即返回，用于启动时恢复检查
                loop {
                    tokio::select! {
                        _ = token.cancelled() => {
                            tracing::info!("poll task received shutdown signal");
                            break;
                        }
                        _ = ticker.tick() => {
                            // 每次轮询两个交易所
                            for exchange in ["kraken", "hyperliquid"] {
                                if let Err(e) = eng.poll_exchange(exchange).await {
                                    tracing::error!(exchange, "poll error: {e:#}");
                                }
                            }
                        }
                    }
                }
            });
        }

        // 丢掉最后一个 clone，确保 fill_rx 在两个 adapter 都退出后能感知关闭
        drop(fill_tx);

        // ── 主事件循环：处理 WebSocket FillEvent（仅更新已成交数量）─────────
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("grid engine received shutdown signal");
                    break;
                }
                event = fill_rx.recv() => {
                    match event {
                        Some(event) => {
                            let eng = Arc::clone(&engine);
                            tokio::spawn(async move {
                                if let Err(e) = eng.handle_fill(event).await {
                                    tracing::error!("handle_fill error: {e:#}");
                                }
                            });
                        }
                        None => {
                            tracing::warn!("grid engine fill channel closed");
                            break;
                        }
                    }
                }
            }
        }

        tracing::info!("grid engine shutting down");
        Ok(())
    }

    /// 处理 WebSocket 成交事件：**仅更新已成交数量**，不挂对手单。
    ///
    /// 将订单状态从 `open` 升级为 `partially_filled`，并更新 `filled_quantity`。
    /// 链式反向挂单由 `poll_exchange` 轮询负责。
    async fn handle_fill(&self, event: FillEvent) -> anyhow::Result<()> {
        // 查询数据库中对应的活跃订单
        let order = match get_order_by_exchange_id(&self.db, &event.exchange_order_id).await? {
            Some(o) => o,
            None => {
                // 可能是其他途径产生的成交（如手动在交易所操作），忽略
                tracing::debug!(
                    exchange_order_id = event.exchange_order_id,
                    "fill event for unknown order, skipping"
                );
                return Ok(());
            }
        };

        tracing::info!(
            id = order.id,
            exchange = order.exchange,
            side = order.side,
            filled_quantity = event.filled_quantity,
            order_qty = order.quantity,
            "fill progress update from websocket"
        );

        // 条件更新：仅当 status 为 open 或 partially_filled 时更新
        // 已 filled/cancelled 的订单不会被覆盖（竞态保护）
        let updated = update_fill_progress(&self.db, order.id, event.filled_quantity)
            .await
            .context("update_fill_progress")?;

        if updated {
            let _ = self.sse_tx.send(SseEvent::OrderUpdated {
                order_id: order.id,
                status: "partially_filled".to_string(),
            });
        }

        Ok(())
    }

    /// 轮询单个交易所的活跃订单，检查是否完全成交并挂对手单。
    ///
    /// 1. 从 DB 获取该交易所所有活跃订单（open + partially_filled）
    /// 2. 筛出**最低价卖单**和**最高价买单**（各最多 1 个）
    /// 3. 查询交易所状态：filled → 挂对手单；cancelled → 标记取消 + 通知
    async fn poll_exchange(&self, exchange: &str) -> anyhow::Result<()> {
        let active_orders = list_active_orders_by_exchange(&self.db, exchange).await?;

        if active_orders.is_empty() {
            return Ok(());
        }

        // 筛出最低价卖单和最高价买单
        let mut lowest_sell: Option<&Order> = None;
        let mut highest_buy: Option<&Order> = None;

        for order in &active_orders {
            if order.side == "sell" {
                if lowest_sell.is_none_or(|s| order.price < s.price) {
                    lowest_sell = Some(order);
                }
            } else {
                // buy
                if highest_buy.is_none_or(|b| order.price > b.price) {
                    highest_buy = Some(order);
                }
            }
        }

        // 检查这两个候选订单的交易所状态
        for order in [highest_buy, lowest_sell].into_iter().flatten() {
            let exchange_oid = match order.exchange_order_id.as_ref() {
                Some(id) => id,
                None => {
                    tracing::warn!(
                        id = order.id,
                        exchange,
                        "active order missing exchange_order_id, skipping poll"
                    );
                    continue;
                }
            };

            let adapter = self.adapter(exchange);
            let status = match adapter.get_order_status(exchange_oid, &order.symbol).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(
                        id = order.id,
                        exchange,
                        exchange_oid,
                        "poll: get_order_status failed: {e:#}"
                    );
                    continue;
                }
            };

            match status.as_str() {
                "filled" => {
                    tracing::info!(
                        id = order.id,
                        exchange,
                        exchange_oid,
                        "poll: order fully filled, triggering reverse grid leg"
                    );
                    if let Err(e) = self.handle_filled_order(order).await {
                        tracing::error!(
                            id = order.id,
                            "poll: handle_filled_order error: {e:#}"
                        );
                    }
                }
                "cancelled" => {
                    tracing::info!(
                        id = order.id,
                        exchange_oid,
                        "poll: order was cancelled"
                    );
                    let _ = set_order_status(
                        &self.db,
                        &UpdateOrderStatus {
                            id: order.id,
                            status: "cancelled",
                        },
                    )
                    .await;

                    let _ = self.sse_tx.send(SseEvent::OrderUpdated {
                        order_id: order.id,
                        status: "cancelled".to_string(),
                    });

                    let notify = format!(
                        "🚫 <b>订单已取消</b>  #{id}\n\
                         {exchange} {side}  {qty:.4} @ {price:.4}",
                        id = order.id,
                        exchange = order.exchange,
                        side = order.side,
                        qty = order.quantity,
                        price = order.price,
                    );
                    if let Err(e) = crate::bot::send_notification(&self.config, &notify).await {
                        tracing::warn!("telegram notify (cancelled) failed: {e:#}");
                    }
                }
                "open" => {
                    tracing::debug!(id = order.id, "poll: order still open");
                }
                other => {
                    tracing::warn!(
                        id = order.id,
                        status = other,
                        "poll: unknown order status from exchange"
                    );
                }
            }
        }

        Ok(())
    }

    /// 处理完全成交的订单：标记 filled + 挂反向对手单。
    ///
    /// 由轮询 `poll_exchange` 调用。使用挂单价格作为 filled_price
    ///（无法从状态 API 获取精确成交价，但保持网格链式完整性）。
    async fn handle_filled_order(&self, order: &Order) -> anyhow::Result<()> {
        // 1. 标记原订单为完全成交（竞态保护：仅 open/partially_filled 可更新）
        let marked = mark_order_filled(
            &self.db,
            &UpdateOrderFilled {
                id: order.id,
                filled_price: order.price,
                filled_quantity: order.quantity,
            },
        )
        .await
        .context("mark_order_filled")?;

        if !marked {
            // 已被其他流程（如并发轮询或 WebSocket）处理，跳过
            tracing::debug!(
                id = order.id,
                "handle_filled_order: order already processed, skipping"
            );
            return Ok(());
        }

        let _ = self.sse_tx.send(SseEvent::OrderUpdated {
            order_id: order.id,
            status: "filled".to_string(),
        });

        // 通知 Telegram：原订单已成交
        let reverse_side = if order.side == "buy" { "sell" } else { "buy" };
        let reverse_price = if order.side == "buy" {
            order.price + order.price_change
        } else {
            (order.price - order.price_change).max(0.0)
        };

        let notify_filled = format!(
            "✅ <b>成交</b>  #{id}\n\
             {exchange} {side}  {qty:.4} @ <b>{price:.4}</b>\n\
             下一口: {reverse_side} @ {reverse_price:.4}",
            id = order.id,
            exchange = order.exchange,
            side = order.side,
            qty = order.quantity,
            price = order.price,
            reverse_side = reverse_side,
            reverse_price = reverse_price,
        );
        if let Err(e) = crate::bot::send_notification(&self.config, &notify_filled).await {
            tracing::warn!("telegram notify (filled) failed: {e:#}");
        }

        // 2. 计算反向订单参数（基于原订单挂单价格，确保网格层级间距固定一致）
        if reverse_price <= 0.0 {
            tracing::warn!(id = order.id, "reverse price <= 0, skipping grid leg");
            return Ok(());
        }

        let leverage = order.leverage.max(1) as u32;

        // 3. 先在数据库中创建 pending 状态的反向订单（id 优先，便于崩溃恢复）
        let new_id = insert_order(
            &self.db,
            &CreateOrder {
                exchange: &order.exchange,
                symbol: &order.symbol,
                side: reverse_side,
                quantity: order.quantity,
                price: reverse_price,
                price_change: order.price_change,
                leverage,
                is_auto: true,
                parent_order_id: Some(order.id),
                exchange_order_id: None,
                status: "pending",
            },
        )
        .await
        .context("insert reverse order")?;

        let _ = self
            .sse_tx
            .send(SseEvent::OrderCreated { order_id: new_id });

        // 4. 提交到交易所
        let adapter = self.adapter(&order.exchange);
        let confirmation = adapter
            .place_limit_order(&OrderRequest {
                symbol: order.symbol.clone(),
                side: reverse_side.to_string(),
                quantity: order.quantity,
                price: reverse_price,
                leverage,
            })
            .await;

        match confirmation {
            Ok(conf) => {
                // 5. 回填交易所订单 ID，状态升级为 open
                set_exchange_order_id(&self.db, new_id, &conf.exchange_order_id)
                    .await
                    .context("set_exchange_order_id")?;
                set_order_status(
                    &self.db,
                    &UpdateOrderStatus {
                        id: new_id,
                        status: "open",
                    },
                )
                .await
                .context("set_order_status open")?;

                tracing::info!(
                    new_id,
                    exchange_order_id = conf.exchange_order_id,
                    side = reverse_side,
                    price = reverse_price,
                    leverage,
                    "reverse grid leg placed"
                );

                let _ = self.sse_tx.send(SseEvent::OrderUpdated {
                    order_id: new_id,
                    status: "open".to_string(),
                });

                // 通知 Telegram：反向挂单已提交
                let notify_open = format!(
                    "📌 <b>网格挂单</b>  #{new_id}\n\
                     {exchange} {side} {qty:.4} @ <b>{price:.4}</b>  Δ{pc:.4}  ×{lev}",
                    exchange = order.exchange,
                    side = reverse_side,
                    qty = order.quantity,
                    price = reverse_price,
                    pc = order.price_change,
                    lev = leverage,
                );
                if let Err(e) = crate::bot::send_notification(&self.config, &notify_open).await {
                    tracing::warn!("telegram notify (open) failed: {e:#}");
                }
            }
            Err(e) => {
                // 下单失败：标记为 failed，不中断引擎
                tracing::error!(new_id, "place reverse order failed: {e:#}");
                set_order_status(
                    &self.db,
                    &UpdateOrderStatus {
                        id: new_id,
                        status: "failed",
                    },
                )
                .await
                .context("set_order_status failed")?;

                let _ = self.sse_tx.send(SseEvent::OrderUpdated {
                    order_id: new_id,
                    status: "failed".to_string(),
                });

                // 通知 Telegram：反向下单失败，需要人工介入
                let notify_fail = format!(
                    "❌ <b>网格下单失败</b>  #{new_id}\n\
                     {exchange} {side} @ {price:.4}\n\
                     {err}",
                    exchange = order.exchange,
                    side = reverse_side,
                    price = reverse_price,
                    err = e,
                );
                if let Err(e2) = crate::bot::send_notification(&self.config, &notify_fail).await {
                    tracing::warn!("telegram notify (failed) failed: {e2:#}");
                }
            }
        }

        Ok(())
    }
}
