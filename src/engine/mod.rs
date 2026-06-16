use std::sync::Arc;

use anyhow::Context;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::db::order::{
    CreateOrder, UpdateOrderFilled, UpdateOrderStatus,
    get_order_by_exchange_id, insert_order, list_open_orders,
    mark_order_filled, set_exchange_order_id, set_order_status,
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
        Self { db, kraken, hyperliquid, sse_tx, config }
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
    /// 2. 启动两个交易所的 `subscribe_fills`，汇入统一 fill channel。
    /// 3. 循环处理 FillEvent，触发链式反向下单。
    pub async fn run(self) -> anyhow::Result<()> {
        // 重启恢复日志
        let open_orders = list_open_orders(&self.db)
            .await
            .context("loading open orders on startup")?;
        if !open_orders.is_empty() {
            tracing::info!(count = open_orders.len(), "restored open orders from db");
        }

        let (fill_tx, mut fill_rx) = mpsc::channel::<FillEvent>(FILL_CHANNEL_CAPACITY);

        // 将 self 包在 Arc 中，以便两个 spawn 和主循环共享
        let engine = Arc::new(self);

        // 启动 Kraken 成交监听
        {
            let tx = fill_tx.clone();
            let adapter = Arc::clone(&engine.kraken);
            tokio::spawn(async move {
                loop {
                    if let Err(e) = adapter.subscribe_fills(tx.clone()).await {
                        tracing::error!("kraken subscribe_fills error: {e:#}");
                    }
                    // 适配器内部已做重连；若任务意外退出，此处等待后重试
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            });
        }

        // 启动 Hyperliquid 成交监听
        {
            let tx = fill_tx.clone();
            let adapter = Arc::clone(&engine.hyperliquid);
            tokio::spawn(async move {
                loop {
                    if let Err(e) = adapter.subscribe_fills(tx.clone()).await {
                        tracing::error!("hyperliquid subscribe_fills error: {e:#}");
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            });
        }

        // 丢掉最后一个 clone，确保 fill_rx 在两个 adapter 都退出后能感知关闭
        drop(fill_tx);

        // 主事件循环
        while let Some(event) = fill_rx.recv().await {
            let eng = Arc::clone(&engine);
            tokio::spawn(async move {
                if let Err(e) = eng.handle_fill(event).await {
                    tracing::error!("handle_fill error: {e:#}");
                }
            });
        }

        tracing::warn!("grid engine fill channel closed, shutting down");
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
            id             = order.id,
            exchange       = order.exchange,
            side           = order.side,
            filled_price   = event.filled_price,
            qty            = event.quantity,
            "order filled, triggering reverse grid leg"
        );

        // 2. 标记原订单为已成交
        mark_order_filled(&self.db, &UpdateOrderFilled {
            id: order.id,
            filled_price: event.filled_price,
        })
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
            id           = order.id,
            exchange     = order.exchange,
            side         = order.side,
            qty          = event.quantity,
            price        = event.filled_price,
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
            tracing::warn!(
                id = order.id,
                "reverse price <= 0, skipping grid leg"
            );
            return Ok(());
        }

        let leverage = order.leverage.max(1) as u32;

        // 4. 先在数据库中创建 pending 状态的反向订单（id 优先，便于崩溃恢复）
        let new_id = insert_order(&self.db, &CreateOrder {
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
        })
        .await
        .context("insert reverse order")?;

        let _ = self.sse_tx.send(SseEvent::OrderCreated { order_id: new_id });

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
                set_order_status(&self.db, &UpdateOrderStatus { id: new_id, status: "open" })
                    .await
                    .context("set_order_status open")?;

                tracing::info!(
                    new_id,
                    exchange_order_id = conf.exchange_order_id,
                    side              = reverse_side,
                    price             = reverse_price,
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
                    side     = reverse_side,
                    qty      = order.quantity,
                    price    = reverse_price,
                    pc       = order.price_change,
                    lev      = leverage,
                );
                if let Err(e) = crate::bot::send_notification(&self.config, &notify_open).await {
                    tracing::warn!("telegram notify (open) failed: {e:#}");
                }
            }
            Err(e) => {
                // 下单失败：标记为 failed，不中断引擎
                tracing::error!(new_id, "place reverse order failed: {e:#}");
                set_order_status(&self.db, &UpdateOrderStatus { id: new_id, status: "failed" })
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
                    side     = reverse_side,
                    price    = reverse_price,
                    err      = e,
                );
                if let Err(e2) = crate::bot::send_notification(&self.config, &notify_fail).await {
                    tracing::warn!("telegram notify (failed) failed: {e2:#}");
                }
            }
        }

        Ok(())
    }
}

