use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
};
use futures::Stream;
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

use crate::api::AppState;

/// SSE 事件类型。
///
/// 序列化为 `{"type": "order_created", "order_id": 42}` 形式，
/// 前端通过 `event.data` 解析。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    /// 新订单创建（手动下单或网格引擎自动生成）
    OrderCreated { order_id: i64 },
    /// 订单状态变更（open / filled / cancelled / failed）
    OrderUpdated { order_id: i64, status: String },
}

/// 全局 SSE broadcast channel 的发送端。
/// 由 `AppState` 持有并注入所有需要推送事件的地方。
pub type SseSender = broadcast::Sender<SseEvent>;

/// 创建 SSE broadcast channel。
/// `capacity`：channel 缓冲的最大事件数，推荐 128。
pub fn create_channel(capacity: usize) -> SseSender {
    broadcast::channel(capacity).0
}

/// `GET /api/sse` — 建立 SSE 连接，向浏览器实时推送订单事件。
///
/// 每 30 秒发送一次 keep-alive 注释，防止反向代理超时断开连接。
/// 慢速消费者（浏览器）落后太多时，`BroadcastStream` 会自动跳过
/// 已被覆盖的消息并继续，不会阻塞其他订阅者。
pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.sse_tx.subscribe();

    let stream = BroadcastStream::new(receiver).filter_map(|result| {
        match result {
            Ok(event) => {
                // 序列化为 JSON 字符串作为 SSE data
                let data = serde_json::to_string(&event).unwrap_or_default();
                Some(Ok(Event::default().data(data)))
            }
            // BroadcastStream::Lagged：消费者落后，跳过，不中断流
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

