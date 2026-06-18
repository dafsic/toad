use async_trait::async_trait;

pub mod kraken;
pub mod hyperliquid;

/// 统一的订单参数，用于向交易所提交限价单。
#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: String,       // "buy" | "sell"
    pub quantity: f64,
    pub price: f64,
    /// 杠杆倍数。Kraken 现货固定传 1；Hyperliquid 永续合约由用户指定，
    /// 链式反向订单从父订单继承此值。
    pub leverage: u32,
}

/// 交易所返回的订单确认信息。
#[derive(Debug, Clone)]
pub struct OrderConfirmation {
    pub exchange_order_id: String,
}

/// 成交回报。
///
/// `filled_quantity` 为**累计已成交数量**（非单次成交量）。
/// 由各适配器从 WebSocket 事件中提取，用于更新 DB 中的部分成交进度。
#[derive(Debug, Clone)]
pub struct FillEvent {
    pub exchange_order_id: String,
    pub filled_quantity: f64,
}

/// 交易所适配器 Trait，Kraken 和 Hyperliquid 各自实现。
#[async_trait]
pub trait ExchangeAdapter: Send + Sync {
    /// 提交限价单（GTC）。
    async fn place_limit_order(&self, req: &OrderRequest) -> anyhow::Result<OrderConfirmation>;

    /// 取消挂单。
    async fn cancel_order(&self, exchange_order_id: &str, symbol: &str) -> anyhow::Result<()>;

    /// 查询单个订单状态。
    async fn get_order_status(&self, exchange_order_id: &str, symbol: &str) -> anyhow::Result<String>;

    /// 订阅成交事件（WebSocket），通过 channel 推送 FillEvent。
    /// FillEvent 携带**累计已成交数量**，引擎仅用于更新部分成交进度，
    /// **不在此触发挂对手单**（由轮询负责）。
    /// 实现应在内部降级为 REST 轮询（当 WebSocket 不可用时）。
    async fn subscribe_fills(
        &self,
        tx: tokio::sync::mpsc::Sender<FillEvent>,
    ) -> anyhow::Result<()>;
}
