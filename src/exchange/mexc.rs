use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::exchange::{ExchangeAdapter, ExchangeKind, FillEvent, OrderConfirmation, OrderRequest};

const REST_BASE: &str = "https://api.mexc.com";
const WS_URL: &str = "wss://wbs-api.mexc.com/ws";
/// listenKey 保活间隔（MEXC 要求每 30 分钟续期一次，有效期 60 分钟）
const LISTEN_KEY_KEEPALIVE_SECS: u64 = 30 * 60;

/// MEXC 现货交易所适配器。
///
/// REST API 采用 query string + HMAC-SHA256 签名（Binance 风格）。
/// 成交监听通过用户数据流（listenKey）实现，自动重连与保活。
pub struct MexcSpotAdapter {
    api_key: String,
    api_secret: String,
    client: Client,
}

impl MexcSpotAdapter {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            client: Client::new(),
        }
    }

    /// 当前毫秒时间戳（MEXC 要求 timestamp 接近服务器时间，故用真实时间）。
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// 计算 MEXC API 签名：HMAC-SHA256(secret, query_string)。
    fn sign(&self, query_string: &str) -> anyhow::Result<String> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())
            .map_err(|e| anyhow!("HMAC init error: {e}"))?;
        mac.update(query_string.as_bytes());
        Ok(hex::encode(mac.finalize().into_bytes()))
    }

    /// 发送已签名的私有请求。`path` 为完整路径（含 `/api/v3/`），
    /// `params` 为业务查询参数（不含签名相关字段）。
    ///
    /// `method` 决定请求方式；签名追加到 query string 末尾。
    async fn send_private<T>(
        &self,
        method: reqwest::Method,
        path: &str,
        params: &[(&str, String)],
    ) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let timestamp = self.now_ms();
        let mut all = params.to_vec();
        all.push(("timestamp", timestamp.to_string()));
        all.push(("recvWindow", "5000".to_string()));

        // 按参数顺序拼接（MEXC 要求签名覆盖的 query string 与实际请求一致）
        let query_string = all
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        let signature = self.sign(&query_string)?;
        let signed_qs = format!("{query_string}&signature={signature}");

        let url = format!("{REST_BASE}{path}?{signed_qs}");

        let resp = self
            .client
            .request(method, url)
            .header("X-MEXC-APIKEY", &self.api_key)
            .send()
            .await
            .with_context(|| path.to_string())?;

        let http_status = resp.status();
        let text = resp.text().await?;

        if !http_status.is_success() {
            return Err(anyhow!("HTTP {http_status} from {path}: {text}"));
        }

        // MEXC 错误体形如 {"code":xxxxx,"msg":"..."}
        let v: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("parsing {path} response"))?;
        if let Some(code) = v.get("code").and_then(|c| c.as_i64()) {
            if !(0..1000).contains(&code) {
                let msg = v.get("msg").and_then(|m| m.as_str()).unwrap_or("unknown");
                return Err(anyhow!("MEXC API error ({path}): code={code} {msg}"));
            }
        }

        serde_json::from_value(v)
            .with_context(|| format!("deserializing result from {path}"))
    }

    /// 获取用户数据流 listenKey。
    ///
    /// MEXC 的 `/api/v3/userDataStream` 同样需要签名（timestamp + signature）。
    async fn post_listen_key(&self) -> anyhow::Result<String> {
        // MEXC API 使用 camelCase 字段名
        #[derive(Deserialize)]
        #[allow(non_snake_case)]
        struct ListenKeyResult {
            listenKey: String,
        }
        let r: ListenKeyResult = self
            .send_private(reqwest::Method::POST, "/api/v3/userDataStream", &[])
            .await?;
        Ok(r.listenKey)
    }
}

// ── REST 响应类型 ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct CreateOrderResult {
    #[serde(default)]
    orderId: Option<String>,
    /// MEXC 现货可能返回 transId / clientOrderId，取 orderId 为主
    #[serde(default)]
    transId: Option<String>,
}

#[derive(Deserialize)]
struct QueryOrderResult {
    status: String,
}

// ── WebSocket 消息类型 ────────────────────────────────────────────────────────

/// 用户数据流推送的订单事件（MEXC spot v3 `spot@private.orders.v3.api`）。
///
/// 外层含 `channel` 字段标识频道，`privateOrders` 为订单详情对象。
/// 字段名使用 camelCase（MEXC API 约定），故结构体加 `#[allow(non_snake_case)]`。
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct WsOrderMsg {
    #[serde(default)]
    channel: Option<String>,
    /// 订单详情（订单事件时存在；订阅确认 / 心跳时缺失）
    #[serde(default)]
    privateOrders: Option<WsPrivateOrders>,
}

/// `privateOrders` 对象内的订单字段。
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct WsPrivateOrders {
    /// 订单 ID（字符串）
    id: String,
    /// 累计已成交数量（字符串，需 parse 成 f64）
    cumulativeQuantity: String,
    /// 订单状态：1 未成交 / 2 完全成交 / 3 部分成交 / 4 已撤销 / 5 部分撤销
    #[serde(default)]
    status: Option<i64>,
}

// ── ExchangeAdapter 实现 ──────────────────────────────────────────────────────

#[async_trait]
impl ExchangeAdapter for MexcSpotAdapter {
    fn kind(&self) -> ExchangeKind {
        ExchangeKind::Spot
    }

    /// 提交 GTC 限价单。
    /// MEXC 现货不需要杠杆，`req.leverage` 字段忽略（固定为 1）。
    async fn place_limit_order(&self, req: &OrderRequest) -> anyhow::Result<OrderConfirmation> {
        let result: CreateOrderResult = self
            .send_private(
                reqwest::Method::POST,
                "/api/v3/order",
                &[
                    ("symbol", crate::exchange::EXCHANGE_SYMBOL.to_string()),
                    ("side", req.side.to_uppercase()),
                    ("type", "LIMIT".to_string()),
                    ("quantity", format!("{:.8}", req.quantity)),
                    ("price", format!("{:.5}", req.price)),
                ],
            )
            .await?;

        let oid = result
            .orderId
            .or(result.transId)
            .ok_or_else(|| anyhow!("MEXC CreateOrder returned empty orderId/transId"))?;

        tracing::info!(
            oid,
            side = req.side,
            price = req.price,
            qty = req.quantity,
            "mexc_spot order placed"
        );
        Ok(OrderConfirmation {
            exchange_order_id: oid,
        })
    }

    /// 取消挂单（按 orderId）。
    async fn cancel_order(&self, exchange_order_id: &str, _symbol: &str) -> anyhow::Result<()> {
        // MEXC DELETE 忽略返回体
        let _: serde_json::Value = self
            .send_private(
                reqwest::Method::DELETE,
                "/api/v3/order",
                &[
                    ("symbol", crate::exchange::EXCHANGE_SYMBOL.to_string()),
                    ("orderId", exchange_order_id.to_string()),
                ],
            )
            .await?;

        tracing::info!(oid = exchange_order_id, "mexc_spot order cancelled");
        Ok(())
    }

    /// 查询单个订单状态，返回标准化字符串：
    /// `"open"` | `"filled"` | `"cancelled"` | `"unknown"`
    async fn get_order_status(
        &self,
        exchange_order_id: &str,
        _symbol: &str,
    ) -> anyhow::Result<String> {
        let r: QueryOrderResult = self
            .send_private(
                reqwest::Method::GET,
                "/api/v3/order",
                &[
                    ("symbol", crate::exchange::EXCHANGE_SYMBOL.to_string()),
                    ("orderId", exchange_order_id.to_string()),
                ],
            )
            .await?;

        // MEXC 状态映射：
        //   NEW / PARTIALLY_FILLED -> "open"
        //   FILLED                 -> "filled"
        //   CANCELED / EXPIRED     -> "cancelled"
        let status = match r.status.as_str() {
            "NEW" | "PARTIALLY_FILLED" => "open",
            "FILLED" => "filled",
            "CANCELED" | "EXPIRED" => "cancelled",
            other => {
                tracing::warn!(
                    oid = exchange_order_id,
                    status = other,
                    "unknown MEXC order status"
                );
                "unknown"
            }
        };
        Ok(status.to_string())
    }

    /// 订阅成交事件（用户数据流）。
    ///
    /// 流程：获取 listenKey → 连接 `wss://wbs.mexc.com?listenKey=…`
    /// → 每 30 分钟 PUT 续期 → 解析订单事件推送累计成交量 → FillEvent。
    /// 连接断开后指数退避重连。
    async fn subscribe_fills(&self, tx: mpsc::Sender<FillEvent>) -> anyhow::Result<()> {
        const INITIAL_DELAY_MS: u64 = 1_000;
        const MAX_DELAY_MS: u64 = 30_000;
        let mut delay_ms = INITIAL_DELAY_MS;

        loop {
            let listen_key = match self.post_listen_key().await {
                Ok(k) => k,
                Err(e) => {
                    tracing::error!(
                        "mexc_spot: failed to get listenKey: {e:#}, retrying in {delay_ms}ms"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(MAX_DELAY_MS);
                    continue;
                }
            };

            let ws_url = format!("{WS_URL}?listenKey={listen_key}");
            let ws_stream = match connect_async(&ws_url).await {
                Ok((stream, _)) => stream,
                Err(e) => {
                    tracing::error!(
                        "mexc_spot: WS connect failed: {e:#}, retrying in {delay_ms}ms"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(MAX_DELAY_MS);
                    continue;
                }
            };

            delay_ms = INITIAL_DELAY_MS;
            tracing::info!("mexc_spot: WS connected");

            let (mut sink, mut stream) = ws_stream.split();

            // 订阅订单私有频道（MEXC 要求连接后 30 秒内发送订阅，否则断开）
            let subscribe = serde_json::json!({
                "method": "SUBSCRIPTION",
                "params": ["spot@private.orders.v3.api"]
            });
            if let Err(e) = sink.send(Message::Text(subscribe.to_string())).await {
                tracing::error!("mexc_spot: WS subscribe send failed: {e:#}");
                continue;
            }
            tracing::info!("mexc_spot: subscribed to spot@private.orders.v3.api");

            // 启动 listenKey 保活任务（PUT 续期同样需要签名）
            let keepalive_key = listen_key.clone();
            let keepalive_api_key = self.api_key.clone();
            let keepalive_api_secret = self.api_secret.clone();
            let keepalive_client = self.client.clone();
            let keepalive_handle = tokio::spawn(async move {
                let mut ticker =
                    tokio::time::interval(tokio::time::Duration::from_secs(LISTEN_KEY_KEEPALIVE_SECS));
                loop {
                    ticker.tick().await;
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let query_string = format!(
                        "listenKey={keepalive_key}&timestamp={timestamp}&recvWindow=5000"
                    );
                    let mut mac = match Hmac::<Sha256>::new_from_slice(keepalive_api_secret.as_bytes()) {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!("mexc_spot: keepalive HMAC init failed: {e:#}");
                            continue;
                        }
                    };
                    mac.update(query_string.as_bytes());
                    let signature = hex::encode(mac.finalize().into_bytes());
                    let url = format!(
                        "{REST_BASE}/api/v3/userDataStream?{query_string}&signature={signature}"
                    );
                    if let Err(e) = keepalive_client
                        .put(url)
                        .header("X-MEXC-APIKEY", &keepalive_api_key)
                        .send()
                        .await
                    {
                        tracing::warn!("mexc_spot: listenKey keepalive failed: {e:#}");
                    }
                }
            });

            // 消息处理循环
            'recv: loop {
                let msg = match stream.next().await {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        tracing::warn!("mexc_spot: WS error: {e:#}");
                        break 'recv;
                    }
                    None => {
                        tracing::warn!("mexc_spot: WS stream closed by server");
                        break 'recv;
                    }
                };

                let text = match msg {
                    Message::Text(t) => t,
                    Message::Ping(data) => {
                        let _ = sink.send(Message::Pong(data)).await;
                        continue 'recv;
                    }
                    Message::Close(_) => {
                        tracing::info!("mexc_spot: WS close frame received");
                        break 'recv;
                    }
                    _ => continue 'recv,
                };

                // MEXC 订单事件：channel 为 spot@private.orders.v3.api 且含 privateOrders
                let ws_msg: WsOrderMsg = match serde_json::from_str(&text) {
                    Ok(m) => m,
                    Err(_) => continue 'recv, // 心跳 / 订阅确认等
                };

                // 仅处理订单频道消息
                let is_order_channel = ws_msg
                    .channel
                    .as_deref()
                    .is_some_and(|c| c.starts_with("spot@private.orders"));
                if !is_order_channel {
                    continue 'recv;
                }

                let orders = match ws_msg.privateOrders {
                    Some(o) => o,
                    None => continue 'recv,
                };

                // 累计成交量 > 0 时推送（部分成交 status=3 或完全成交 status=2）
                let cum_qty: f64 = match orders.cumulativeQuantity.parse() {
                    Ok(v) => v,
                    Err(_) => continue 'recv,
                };
                if cum_qty <= 0.0 {
                    continue 'recv;
                }

                tracing::info!(
                    order_id = orders.id,
                    cum_qty = cum_qty,
                    status = orders.status.unwrap_or(0),
                    "mexc_spot fill progress"
                );
                let event = FillEvent {
                    exchange_order_id: orders.id,
                    filled_quantity: cum_qty,
                };
                if tx.send(event).await.is_err() {
                    keepalive_handle.abort();
                    return Ok(());
                }
            }

            keepalive_handle.abort();
            tracing::info!("mexc_spot: WS disconnected, reconnecting in {delay_ms}ms…");
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            delay_ms = (delay_ms * 2).min(MAX_DELAY_MS);
        }
    }
}
