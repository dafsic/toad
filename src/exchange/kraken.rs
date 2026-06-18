use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use futures::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256, Sha512};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::exchange::{ExchangeAdapter, FillEvent, OrderConfirmation, OrderRequest};

const REST_BASE: &str = "https://api.kraken.com";
const WS_URL: &str = "wss://ws-auth.kraken.com/v2";

/// Kraken 现货交易所适配器。
///
/// REST API 采用 JSON body + HMAC-SHA512 签名。
/// 成交监听通过 WebSocket v2 的 `executions` 频道实现，自动重连。
pub struct KrakenAdapter {
    api_key: String,
    api_secret: String,
    client: Client,
    /// 单调递增 nonce（以启动时毫秒时间戳为起点）
    nonce: Arc<AtomicU64>,
}

impl KrakenAdapter {
    pub fn new(api_key: String, api_secret: String) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            api_key,
            api_secret,
            client: Client::new(),
            nonce: Arc::new(AtomicU64::new(now_ms)),
        }
    }

    /// 生成下一个唯一 nonce。
    fn next_nonce(&self) -> u64 {
        self.nonce.fetch_add(1, Ordering::Relaxed)
    }

    /// 计算 Kraken API 签名。
    ///
    /// ```text
    /// API-Sign = Base64(HMAC-SHA512(Base64-decode(secret),
    ///                               URI-path + SHA256(nonce_str + json_body)))
    /// ```
    fn sign(&self, path: &str, nonce: u64, body: &str) -> anyhow::Result<String> {
        // SHA256(nonce_str + body)
        let mut hasher = Sha256::new();
        hasher.update(nonce.to_string().as_bytes());
        hasher.update(body.as_bytes());
        let sha256_digest = hasher.finalize();

        // HMAC-SHA512(Base64-decode(secret), path_bytes + sha256_digest_bytes)
        let secret_bytes = BASE64
            .decode(&self.api_secret)
            .context("invalid Kraken API secret (must be base64-encoded)")?;
        let mut mac = Hmac::<Sha512>::new_from_slice(&secret_bytes)
            .map_err(|e| anyhow!("HMAC init error: {e}"))?;
        mac.update(path.as_bytes());
        mac.update(&sha256_digest);

        Ok(BASE64.encode(mac.finalize().into_bytes()))
    }

    /// 向 Kraken 私有 REST 端点发送已签名的 JSON 请求。
    async fn post_private<T>(&self, path: &str, mut body: serde_json::Value) -> anyhow::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let nonce = self.next_nonce();
        body["nonce"] = serde_json::json!(nonce);
        let body_str = serde_json::to_string(&body)?;
        let signature = self.sign(path, nonce, &body_str)?;

        let resp = self
            .client
            .post(format!("{REST_BASE}{path}"))
            .header("API-Key", &self.api_key)
            .header("API-Sign", signature)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await
            .with_context(|| format!("POST {path}"))?;

        let http_status = resp.status();
        let text = resp.text().await?;

        if !http_status.is_success() {
            return Err(anyhow!("HTTP {http_status} from {path}: {text}"));
        }

        let v: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("parsing {path} response"))?;

        // Kraken 返回的错误在 "error" 数组中
        if let Some(errors) = v["error"].as_array() {
            if !errors.is_empty() {
                return Err(anyhow!(
                    "Kraken API error ({path}): {}",
                    errors
                        .iter()
                        .filter_map(|e| e.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        serde_json::from_value(v["result"].clone())
            .with_context(|| format!("deserializing result from {path}"))
    }

    /// 获取 WebSocket 认证 token（有效期 15 分钟，连接期间持续有效）。
    async fn ws_token(&self) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct WsTokenResult {
            token: String,
        }
        let r: WsTokenResult =
            self.post_private("/0/private/GetWebSocketsToken", serde_json::json!({}))
                .await?;
        Ok(r.token)
    }
}

// ── REST 响应类型 ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AddOrderResult {
    txid: Vec<String>,
}

#[derive(Deserialize)]
struct CancelOrderResult {
    count: u32,
}

#[derive(Deserialize)]
struct OrderInfo {
    status: String,
}

// ── WebSocket 消息类型 ────────────────────────────────────────────────────────

/// `executions` 频道推送的外层消息。
#[derive(Deserialize)]
struct WsExecutionMsg {
    channel: String,
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    data: Vec<WsExecutionData>,
}

/// 单条执行报告（订单状态变更或成交事件）。
#[derive(Deserialize)]
struct WsExecutionData {
    order_id: String,
    /// 事件类型：`pending_new` / `new` / `trade` / `filled` / `canceled` / `expired`…
    /// `trade` = 部分成交，`filled` = 完全成交
    exec_type: String,
    /// 已累计成交数量（Kraken WebSocket 提供 cumulative 值）
    #[serde(default)]
    cum_qty: Option<f64>,
}

// ── ExchangeAdapter 实现 ──────────────────────────────────────────────────────

#[async_trait]
impl ExchangeAdapter for KrakenAdapter {
    /// 提交 GTC 限价单。
    /// Kraken 现货不需要杠杆，`req.leverage` 字段忽略（固定为 1）。
    async fn place_limit_order(&self, req: &OrderRequest) -> anyhow::Result<OrderConfirmation> {
        let result: AddOrderResult = self
            .post_private(
                "/0/private/AddOrder",
                serde_json::json!({
                    "ordertype":   "limit",
                    "type":        req.side,
                    "volume":      format!("{:.8}", req.quantity),
                    "pair":        "XMRUSDC",
                    "price":       format!("{:.5}", req.price),
                    "timeinforce": "GTC",
                }),
            )
            .await?;

        let txid = result
            .txid
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Kraken AddOrder returned empty txid list"))?;

        tracing::info!(
            txid,
            side  = req.side,
            price = req.price,
            qty   = req.quantity,
            "kraken order placed"
        );
        Ok(OrderConfirmation {
            exchange_order_id: txid,
        })
    }

    /// 取消挂单（按 txid）。
    async fn cancel_order(&self, exchange_order_id: &str, _symbol: &str) -> anyhow::Result<()> {
        let result: CancelOrderResult = self
            .post_private(
                "/0/private/CancelOrder",
                serde_json::json!({ "txid": exchange_order_id }),
            )
            .await?;

        if result.count == 0 {
            tracing::warn!(txid = exchange_order_id, "kraken cancel: 0 orders cancelled");
        } else {
            tracing::info!(txid = exchange_order_id, "kraken order cancelled");
        }
        Ok(())
    }

    /// 查询单个订单状态，返回标准化字符串：
    /// `"open"` | `"filled"` | `"cancelled"` | `"unknown"`
    async fn get_order_status(
        &self,
        exchange_order_id: &str,
        _symbol: &str,
    ) -> anyhow::Result<String> {
        // QueryOrders 返回 txid -> OrderInfo 的 Map
        let result: std::collections::HashMap<String, OrderInfo> = self
            .post_private(
                "/0/private/QueryOrders",
                serde_json::json!({ "txid": exchange_order_id }),
            )
            .await?;

        let info = result
            .get(exchange_order_id)
            .ok_or_else(|| anyhow!("order {exchange_order_id} not found in QueryOrders response"))?;

        // Kraken 状态映射：
        //   pending / open  -> "open"
        //   closed          -> "filled"   (closed = 完全成交)
        //   canceled        -> "cancelled"
        //   expired         -> "cancelled"
        let status = match info.status.as_str() {
            "open" | "pending" => "open",
            "closed"           => "filled",
            "canceled"
            | "expired"        => "cancelled",
            other              => {
                tracing::warn!(txid = exchange_order_id, status = other, "unknown Kraken order status");
                "unknown"
            }
        };
        Ok(status.to_string())
    }

    /// 订阅成交事件（WebSocket v2 `executions` 频道）。
    ///
    /// 仅在 `exec_type == "filled"`（完全成交）时向 `tx` 发送 `FillEvent`。
    /// 连接断开后指数退避重连；`tx` 关闭时（引擎退出）干净退出。
    async fn subscribe_fills(&self, tx: mpsc::Sender<FillEvent>) -> anyhow::Result<()> {
        const INITIAL_DELAY_MS: u64 = 1_000;
        const MAX_DELAY_MS: u64 = 30_000;
        let mut delay_ms = INITIAL_DELAY_MS;

        loop {
            // 每次（重）连接都获取新 token，防止 token 在断线期间过期
            let token = match self.ws_token().await {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("kraken: failed to get WS token: {e:#}, retrying in {delay_ms}ms");
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(MAX_DELAY_MS);
                    continue;
                }
            };

            let ws_stream = match connect_async(WS_URL).await {
                Ok((stream, _)) => stream,
                Err(e) => {
                    tracing::error!("kraken: WS connect failed: {e:#}, retrying in {delay_ms}ms");
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(MAX_DELAY_MS);
                    continue;
                }
            };

            delay_ms = INITIAL_DELAY_MS; // 成功连接后重置退避
            tracing::info!("kraken: WS connected to {WS_URL}");

            let (mut sink, mut stream) = ws_stream.split();

            // 订阅 executions 频道，不请求历史快照
            let subscribe = serde_json::json!({
                "method": "subscribe",
                "params": {
                    "channel":     "executions",
                    "token":       token,
                    "snap_orders": false,
                    "snap_trades": false,
                }
            });
            if let Err(e) = sink.send(Message::Text(subscribe.to_string())).await {
                tracing::error!("kraken: WS subscribe send failed: {e:#}");
                continue;
            }

            // 消息处理循环
            'recv: loop {
                let msg = match stream.next().await {
                    Some(Ok(m))  => m,
                    Some(Err(e)) => {
                        tracing::warn!("kraken: WS error: {e:#}");
                        break 'recv;
                    }
                    None => {
                        tracing::warn!("kraken: WS stream closed by server");
                        break 'recv;
                    }
                };

                let text = match msg {
                    Message::Text(t) => t,
                    Message::Ping(data) => {
                        // tungstenite 通常自动处理 Ping/Pong，但以防万一手动回复
                        let _ = sink.send(Message::Pong(data)).await;
                        continue 'recv;
                    }
                    Message::Close(_) => {
                        tracing::info!("kraken: WS close frame received");
                        break 'recv;
                    }
                    _ => continue 'recv,
                };

                // 只处理 executions 频道的 update 消息（skip snapshot & heartbeat）
                let exec_msg: WsExecutionMsg = match serde_json::from_str(&text) {
                    Ok(m) => m,
                    Err(_) => continue 'recv, // heartbeat / subscription ack 等
                };
                if exec_msg.channel != "executions" || exec_msg.msg_type != "update" {
                    continue 'recv;
                }

                for item in exec_msg.data {
                    // 处理部分成交（trade）和完全成交（filled）事件。
                    // 两者都携带 cum_qty（累计已成交数量），用于更新 DB 中的成交进度。
                    // 引擎仅更新 filled_quantity，不在此挂对手单（由轮询负责）。
                    if item.exec_type != "trade" && item.exec_type != "filled" {
                        continue;
                    }
                    let Some(cum_qty) = item.cum_qty else {
                        tracing::warn!(order_id = item.order_id, "kraken: fill event missing cum_qty");
                        continue;
                    };
                    if cum_qty <= 0.0 {
                        continue;
                    }

                    tracing::info!(
                        order_id = item.order_id,
                        cum_qty,
                        exec_type = item.exec_type,
                        "kraken fill progress"
                    );

                    let event = FillEvent {
                        exchange_order_id: item.order_id,
                        filled_quantity: cum_qty,
                    };
                    if tx.send(event).await.is_err() {
                        // 接收端（GridEngine）已关闭，干净退出
                        return Ok(());
                    }
                }
            }

            tracing::info!("kraken: WS disconnected, reconnecting in {delay_ms}ms…");
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            delay_ms = (delay_ms * 2).min(MAX_DELAY_MS);
        }
    }
}

