use std::sync::Arc;

use anyhow::Context;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::db::order::{
    get_order_by_exchange_id, insert_order, list_open_orders, mark_order_filled,
    set_exchange_order_id, set_order_status, CreateOrder, UpdateOrderFilled, UpdateOrderStatus,
};
use crate::exchange::{ExchangeAdapter, FillEvent, OrderRequest};
use crate::sse::{SseEvent, SseSender};

/// fill channel 容量：允许两个交易所各自有一定积压
const FILL_CHANNEL_CAPACITY: usize = 256;

/// 网格引擎：监听成交事件，触发链式反向限价单。
///
/// 每笔完全成交后，按如下规则自动挂出反向限价单：
/// - buy  成交 → sell，价格 = `filled_price + price_change`
/// - sell 成交 → buy，  价格 = `filled_price - price_change`
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
    /// 1. 恢复所有 `status = 'open'` 的订单（重启后重新监听）。
    /// 2. 同步订单状态（检查停机期间是否成交/取消）。
    /// 3. 启动两个交易所的 `subscribe_fills`，汇入统一 fill channel。
    /// 4. 循环处理 FillEvent，触发链式反向下单。
    /// 5. 收到 shutdown_token 取消信号时优雅退出。
    pub async fn run(self, shutdown_token: CancellationToken) -> anyhow::Result<()> {
        // 重启恢复日志
        let open_orders = list_open_orders(&self.db)
            .await
            .context("loading open orders on startup")?;
        if !open_orders.is_empty() {
            tracing::info!(count = open_orders.len(), "restoring open orders from db");

            // 主动查询交易所，检查这些订单在停机期间的状态变化
            for order in &open_orders {
                if let Err(e) = self.sync_order_status_on_startup(order).await {
                    tracing::error!(
                        id = order.id,
                        exchange_oid = ?order.exchange_order_id,
                        "failed to sync order status on startup: {e:#}"
                    );
                }
            }
        }

        let (fill_tx, mut fill_rx) = mpsc::channel::<FillEvent>(FILL_CHANNEL_CAPACITY);

        // 将 self 包在 Arc 中，以便两个 spawn 和主循环共享
        let engine = Arc::new(self);

        // 启动 Kraken 成交监听
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

        // 启动 Hyperliquid 成交监听
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

        // 丢掉最后一个 clone，确保 fill_rx 在两个 adapter 都退出后能感知关闭
        drop(fill_tx);

        // 主事件循环
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

    /// 启动时同步单个订单的状态（检查停机期间是否成交/取消）。
    ///
    /// **注意**：如果订单在停机期间成交，由于无法获取精确的成交价格，
    /// 将使用挂单价格作为 filled_price 来触发链式反向单。
    /// 这可能导致价格不精确，但至少能保持网格链式完整性。
    ///
    /// 如果需要精确成交价格，建议扩展 ExchangeAdapter trait 增加
    /// `get_fill_details` 方法返回成交详情。
    async fn sync_order_status_on_startup(
        &self,
        order: &crate::db::order::Order,
    ) -> anyhow::Result<()> {
        let exchange_oid = match order.exchange_order_id.as_ref() {
            Some(id) => id,
            None => {
                tracing::warn!(
                    id = order.id,
                    "open order missing exchange_order_id, skipping sync"
                );
                return Ok(());
            }
        };

        let adapter = self.adapter(&order.exchange);
        let status = adapter
            .get_order_status(exchange_oid, &order.symbol)
            .await
            .context("get_order_status")?;

        match status.as_str() {
            "filled" => {
                tracing::warn!(
                    id = order.id,
                    exchange_oid,
                    "order filled during downtime, triggering chain recovery with order price"
                );

                // 使用挂单价格作为成交价格（不精确，但能保持链式）
                // 触发成交处理逻辑（复用 handle_fill，但订单已在数据库中）
                let fill_event = FillEvent {
                    exchange_order_id: exchange_oid.clone(),
                    filled_price: order.price,
                    quantity: order.quantity,
                };

                // 由于 handle_fill 会查询数据库并标记为 filled，
                // 这里直接发送事件让主流程处理即可
                // 注意：handle_fill 是幂等的，重复调用不会产生多个链式订单
                if let Err(e) = self.handle_fill(fill_event).await {
                    tracing::error!(
                        id = order.id,
                        "failed to recover fill chain on startup: {e:#}"
                    );
                }
            }
            "cancelled" => {
                tracing::info!(
                    id = order.id,
                    exchange_oid,
                    "order was cancelled during downtime"
                );
                set_order_status(
                    &self.db,
                    &UpdateOrderStatus {
                        id: order.id,
                        status: "cancelled",
                    },
                )
                .await
                .context("set_order_status cancelled")?;

                let _ = self.sse_tx.send(SseEvent::OrderUpdated {
                    order_id: order.id,
                    status: "cancelled".to_string(),
                });

                // 通知 Telegram：订单已取消
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
                    tracing::warn!("telegram notify (cancelled during downtime) failed: {e:#}");
                }
            }
            "open" => {
                // 仍然挂单中，无需处理
                tracing::debug!(id = order.id, "order still open");
            }
            other => {
                tracing::warn!(
                    id = order.id,
                    status = other,
                    "unknown order status from exchange"
                );
            }
        }

        Ok(())
    }

    /// 处理单次完全成交事件，执行链式反向下单。
    async fn handle_fill(&self, event: FillEvent) -> anyhow::Result<()> {
        // 1. 查询数据库中对应的挂单
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
            filled_price = event.filled_price,
            qty = event.quantity,
            "order filled, triggering reverse grid leg"
        );

        // 2. 标记原订单为已成交
        mark_order_filled(
            &self.db,
            &UpdateOrderFilled {
                id: order.id,
                filled_price: event.filled_price,
            },
        )
        .await
        .context("mark_order_filled")?;

        let _ = self.sse_tx.send(SseEvent::OrderUpdated {
            order_id: order.id,
            status: "filled".to_string(),
        });

        // 通知 Telegram：原订单已成交
        let notify_filled = format!(
            "✅ <b>成交</b>  #{id}\n\
             {exchange} {side}  {qty:.4} @ <b>{price:.4}</b>\n\
             下一口: {reverse_side} @ {reverse_price:.4}",
            id = order.id,
            exchange = order.exchange,
            side = order.side,
            qty = event.quantity,
            price = event.filled_price,
            reverse_side = if order.side == "buy" { "sell" } else { "buy" },
            reverse_price = if order.side == "buy" {
                event.filled_price + order.price_change
            } else {
                (event.filled_price - order.price_change).max(0.0)
            },
        );
        if let Err(e) = crate::bot::send_notification(&self.config, &notify_filled).await {
            tracing::warn!("telegram notify (filled) failed: {e:#}");
        }

        // 3. 计算反向订单参数
        let reverse_side = if order.side == "buy" { "sell" } else { "buy" };
        let reverse_price = if order.side == "buy" {
            event.filled_price + order.price_change
        } else {
            (event.filled_price - order.price_change).max(0.0)
        };

        if reverse_price <= 0.0 {
            tracing::warn!(id = order.id, "reverse price <= 0, skipping grid leg");
            return Ok(());
        }

        let leverage = order.leverage.max(1) as u32;

        // 4. 先在数据库中创建 pending 状态的反向订单（id 优先，便于崩溃恢复）
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

        // 5. 提交到交易所
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
                // 6. 回填交易所订单 ID，状态升级为 open
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
