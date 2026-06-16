use axum::response::sse::Event;
use tokio::sync::broadcast;

/// SSE 事件类型。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    /// 新订单创建（手动或自动）
    OrderCreated { order_id: i64 },
    /// 订单状态变更（filled / cancelled / failed）
    OrderUpdated { order_id: i64, status: String },
}

/// 全局 SSE broadcast channel，Engine 和 API handler 向此 channel 发送事件。
pub type SseSender = broadcast::Sender<SseEvent>;

/// 创建 SSE broadcast channel，返回 (sender, receiver)。
pub fn create_channel(capacity: usize) -> (SseSender, broadcast::Receiver<SseEvent>) {
    broadcast::channel(capacity)
}

/// Axum SSE handler：GET /api/sse
/// 订阅 broadcast channel，将事件流式推送给浏览器。
pub async fn sse_handler(
    // State(state): State<AppState>,
) -> axum::response::Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
    // TODO:
    // 1. 从 AppState 中克隆 broadcast::Receiver
    // 2. 将 receiver 包装为 tokio_stream::wrappers::BroadcastStream
    // 3. map 每个事件为 axum::response::sse::Event
    // 4. 返回 Sse::new(stream).keep_alive(KeepAlive::default())
    todo!()
}
