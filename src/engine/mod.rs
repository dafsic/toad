use std::sync::Arc;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use crate::exchange::{ExchangeAdapter, FillEvent};

/// 网格引擎：监听成交事件，触发链式反向限价单。
pub struct GridEngine {
    db: SqlitePool,
    kraken: Arc<dyn ExchangeAdapter>,
    hyperliquid: Arc<dyn ExchangeAdapter>,
}

impl GridEngine {
    pub fn new(
        db: SqlitePool,
        kraken: Arc<dyn ExchangeAdapter>,
        hyperliquid: Arc<dyn ExchangeAdapter>,
    ) -> Self {
        Self { db, kraken, hyperliquid }
    }

    /// 启动引擎：
    /// 1. 从数据库恢复所有 status = 'open' 的订单，重新监听。
    /// 2. 启动各交易所 subscribe_fills，将事件汇入统一 channel。
    /// 3. 循环处理 FillEvent，触发链式反向下单逻辑。
    pub async fn run(self) -> anyhow::Result<()> {
        // TODO:
        // let (tx, mut rx) = mpsc::channel::<FillEvent>(128);
        // tokio::spawn(self.kraken.subscribe_fills(tx.clone()));
        // tokio::spawn(self.hyperliquid.subscribe_fills(tx.clone()));
        //
        // while let Some(event) = rx.recv().await {
        //     self.handle_fill(event).await?;
        // }
        todo!()
    }

    /// 处理单次成交事件，执行链式反向下单。
    /// 核心逻辑：
    ///   buy  成交 → sell，价格 = filled_price + price_change
    ///   sell 成交 → buy， 价格 = filled_price - price_change
    async fn handle_fill(&self, _event: FillEvent) -> anyhow::Result<()> {
        // TODO:
        // 1. 根据 exchange_order_id 查询数据库中对应订单
        // 2. 更新订单 status = 'filled', filled_price
        // 3. 计算反向订单参数（side 取反，price 按 price_change 偏移）
        // 4. leverage 从父订单继承（Kraken 永远为 1，Hyperliquid 继承原始值）
        // 5. 调用对应交易所适配器 place_limit_order（含 leverage）
        // 6. 将新订单写入数据库（is_auto=1, parent_order_id=当前订单id, leverage=继承值）
        // 7. 通过 SSE 推送状态更新
        todo!()
    }
}
