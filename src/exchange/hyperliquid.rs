use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures::StreamExt;
use hypersdk::Address;
use hypersdk::hypercore::{
    self, NonceHandler, PrivateKeySigner,
    types::{
        BatchCancel, BatchOrder, Cancel, Incoming, OrderGrouping, OrderResponseStatus,
        OrderTypePlacement, Side as HlSide, Subscription, TimeInForce,
        OrderRequest as HlOrderRequest,
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::exchange::{ExchangeAdapter, ExchangeKind, FillEvent, OrderConfirmation, OrderRequest};

/// Hyperliquid 永续合约交易所适配器（逐仓模式）。
///
/// 使用 [hypersdk](https://github.com/infinitefield/hypersdk) 库实现。
/// 每次下单前自动以逐仓（isolated）模式设置目标杠杆倍数。
pub struct HyperliquidAdapter {
    client: hypersdk::hypercore::HttpClient,
    signer: PrivateKeySigner,
    /// 用于 info 查询的账户地址（API 钱包模式下为主账户地址）
    user_address: Address,
    /// 市场名称 -> PerpMarket 映射（含 asset index 和 tick 规则）
    markets: HashMap<String, hypersdk::hypercore::PerpMarket>,
    nonce: Arc<NonceHandler>,
    /// oid -> 累计已成交数量。
    /// Hyperliquid WebSocket Fill.sz 是单次成交量（非累计），
    /// 需在本地累加以提供累计值给引擎。
    filled_tracker: Arc<Mutex<HashMap<u64, f64>>>,
}

impl HyperliquidAdapter {
    /// 初始化适配器。
    ///
    /// - `private_key`：API 钱包的私钥（十六进制，可选 0x 前缀）
    /// - `account_address`：主账户地址。传空字符串时使用私钥推导地址；
    ///   当私钥为 API agent wallet 时须显式传入主账户地址。
    /// - `testnet`：true 连接测试网，false 连接主网
    pub async fn new(
        private_key: &str,
        account_address: &str,
        testnet: bool,
    ) -> anyhow::Result<Self> {
        let signer: PrivateKeySigner = private_key
            .trim()
            .trim_start_matches("0x")
            .parse()
            .context("invalid Hyperliquid private key")?;

        let user_address: Address = if account_address.trim().is_empty() {
            signer.address()
        } else {
            account_address
                .trim()
                .parse()
                .context("invalid HYPERLIQUID_ACCOUNT_ADDRESS")?
        };

        let client = if testnet {
            hypercore::testnet()
        } else {
            hypercore::mainnet()
        };

        let perps = client
            .perps()
            .await
            .map_err(|e| anyhow!("fetching Hyperliquid perp markets: {e}"))?;

        let markets = perps.into_iter().map(|m| (m.name.clone(), m)).collect();

        tracing::info!(
            signer   = %signer.address(),
            account  = %user_address,
            testnet  = testnet,
            "hyperliquid adapter ready"
        );

        Ok(Self {
            client,
            signer,
            user_address,
            markets,
            nonce: Arc::new(NonceHandler::default()),
            filled_tracker: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// 从交易对名称解析出 Hyperliquid coin symbol。
    /// 支持 "XMR/USDC"、"XMR" 两种写法，统一返回 "XMR"。
    fn coin(symbol: &str) -> &str {
        symbol.split('/').next().unwrap_or(symbol)
    }

    /// 查找市场元数据。
    fn market(&self, coin: &str) -> anyhow::Result<&hypersdk::hypercore::PerpMarket> {
        self.markets
            .get(coin)
            .ok_or_else(|| anyhow!("unknown Hyperliquid market: {coin}"))
    }

    /// 将 f64 价格转为 Decimal，并按市场 tick 规则取整（保守模式：
    /// 买单向下取整，卖单向上取整，确保 maker 挂单不会立刻成交）。
    fn round_price(
        &self,
        coin: &str,
        price: f64,
        is_buy: bool,
    ) -> anyhow::Result<Decimal> {
        let market = self.market(coin)?;
        let raw = Decimal::try_from(price)
            .map_err(|e| anyhow!("invalid price {price}: {e}"))?;
        let side = if is_buy { HlSide::Bid } else { HlSide::Ask };
        market
            .round_by_side(side, raw, /* conservative */ true)
            .ok_or_else(|| anyhow!("cannot round price {price} for {coin}"))
    }

    /// 将 f64 数量转为 Decimal，按市场 sz_decimals 截断。
    fn round_size(&self, coin: &str, qty: f64) -> anyhow::Result<Decimal> {
        let market = self.market(coin)?;
        let sz_decimals = market.sz_decimals.clamp(0, u32::MAX as i64) as u32;
        Decimal::try_from(qty)
            .map(|d| d.round_dp(sz_decimals))
            .map_err(|e| anyhow!("invalid size {qty}: {e}"))
    }

    /// 下单前设置逐仓杠杆。
    /// Hyperliquid 要求每次操作前明确设置，以确保仓位模式一致。
    async fn set_isolated_leverage(&self, coin: &str, leverage: u32) -> anyhow::Result<()> {
        let market = self.market(coin)?;
        self.client
            .update_leverage(
                &self.signer,
                market.index,
                /* is_cross */ false, // 逐仓模式
                leverage,
                self.nonce.next(),
                None,
                None,
            )
            .await
            .map_err(|e| anyhow!("update_leverage({coin}, {leverage}x isolated): {e}"))?;
        tracing::debug!(coin, leverage, "set isolated leverage");
        Ok(())
    }
}

#[async_trait]
impl ExchangeAdapter for HyperliquidAdapter {
    fn kind(&self) -> ExchangeKind {
        ExchangeKind::Perp
    }

    /// 提交逐仓限价单（GTC）。
    ///
    /// 流程：
    /// 1. 按市场规则取整价格和数量
    /// 2. 设置逐仓杠杆（update_leverage, is_cross=false）
    /// 3. 提交 BatchOrder
    /// 4. 返回交易所分配的 oid（exchange_order_id）
    async fn place_limit_order(&self, req: &OrderRequest) -> anyhow::Result<OrderConfirmation> {
        let coin = Self::coin(&req.symbol);
        let is_buy = req.side == "buy";
        let market = self.market(coin)?;

        let limit_px = self.round_price(coin, req.price, is_buy)?;
        let sz = self.round_size(coin, req.quantity)?;

        // 下单前设置逐仓杠杆
        self.set_isolated_leverage(coin, req.leverage).await?;

        let batch = BatchOrder {
            orders: vec![HlOrderRequest {
                asset: market.index,
                is_buy,
                limit_px,
                sz,
                reduce_only: false,
                order_type: OrderTypePlacement::Limit {
                    tif: TimeInForce::Gtc,
                },
                cloid: Default::default(),
            }],
            grouping: OrderGrouping::Na,
            builder: None,
        };

        let statuses = self
            .client
            .place(&self.signer, batch, self.nonce.next(), None, None)
            .await
            .map_err(|e| anyhow!("place_limit_order({coin}): {e}"))?;

        let status = statuses
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("empty order response from Hyperliquid"))?;

        let oid = match status {
            OrderResponseStatus::Resting { oid, .. } => oid,
            OrderResponseStatus::Filled { oid, .. } => oid,
            OrderResponseStatus::Error(e) => return Err(anyhow!("order rejected: {e}")),
            other => return Err(anyhow!("unexpected order status: {other:?}")),
        };

        tracing::info!(coin, %limit_px, %sz, leverage = req.leverage, oid, "order placed");
        Ok(OrderConfirmation {
            exchange_order_id: oid.to_string(),
        })
    }

    /// 取消挂单（按 oid）。
    async fn cancel_order(&self, exchange_order_id: &str, symbol: &str) -> anyhow::Result<()> {
        let coin = Self::coin(symbol);
        let market = self.market(coin)?;
        let oid: u64 = exchange_order_id
            .parse()
            .context("invalid Hyperliquid oid")?;

        let batch = BatchCancel {
            cancels: vec![Cancel {
                asset: market.index,
                oid,
            }],
        };

        let statuses = self
            .client
            .cancel(&self.signer, batch, self.nonce.next(), None, None)
            .await
            .map_err(|e| anyhow!("cancel_order({oid}): {e}"))?;

        for status in statuses {
            if let OrderResponseStatus::Error(e) = status {
                return Err(anyhow!("cancel rejected: {e}"));
            }
        }

        tracing::info!(coin, oid, "order cancelled");
        Ok(())
    }

    /// 查询单个订单状态，返回标准化字符串：
    /// "open" | "filled" | "cancelled" | "unknown"
    async fn get_order_status(
        &self,
        exchange_order_id: &str,
        _symbol: &str,
    ) -> anyhow::Result<String> {
        let oid: u64 = exchange_order_id
            .parse()
            .context("invalid Hyperliquid oid")?;

        let update = self
            .client
            .order_status(self.user_address, either::Left(oid))
            .await
            .map_err(|e| anyhow!("order_status({oid}): {e}"))?;

        let status_str = match update {
            None => "unknown",
            Some(u) if u.status.is_filled() => "filled",
            Some(u) if u.status.is_finished() => "cancelled",
            Some(_) => "open",
        };
        Ok(status_str.to_string())
    }

    /// 订阅成交事件（WebSocket `userFills` 频道）。
    ///
    /// 使用 hypersdk `WebSocket` 实现，自动重连并重新订阅。
    /// Hyperliquid `Fill.sz` 是单次成交量（非累计），适配器在本地
    /// 按 `oid` 累加得到累计已成交数量后发送给引擎。
    /// 引擎仅更新 filled_quantity，不在此挂对手单（由轮询负责）。
    async fn subscribe_fills(
        &self,
        tx: mpsc::Sender<FillEvent>,
    ) -> anyhow::Result<()> {
        let user = self.user_address;
        let tracker = Arc::clone(&self.filled_tracker);
        let mut ws = self.client.websocket();
        ws.subscribe(Subscription::UserFills { user });

        tracing::info!(%user, "subscribed to Hyperliquid userFills");

        while let Some(event) = ws.next().await {
            match event {
                hypersdk::hypercore::ws::Event::Connected => {
                    tracing::info!("hyperliquid ws connected");
                }
                hypersdk::hypercore::ws::Event::Disconnected => {
                    tracing::warn!("hyperliquid ws disconnected, reconnecting…");
                }
                hypersdk::hypercore::ws::Event::Message(Incoming::UserFills {
                    fills,
                    is_snapshot,
                    ..
                }) => {
                    if is_snapshot {
                        // 快照消息是历史成交，不用于更新进度
                        continue;
                    }
                    for fill in fills {
                        let Some(sz) = fill.sz.to_f64() else { continue };

                        // 累加单次成交量到该 oid 的累计记录
                        let cumulative = {
                            let mut map = tracker.lock().await;
                            let entry = map.entry(fill.oid).or_insert(0.0);
                            *entry += sz;
                            *entry
                        };

                        tracing::info!(
                            oid = fill.oid,
                            fill_sz = sz,
                            cumulative,
                            "hyperliquid fill progress"
                        );

                        let event = FillEvent {
                            exchange_order_id: fill.oid.to_string(),
                            filled_quantity: cumulative,
                        };
                        if tx.send(event).await.is_err() {
                            // 接收端已关闭，退出
                            return Ok(());
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

