import { useEffect } from 'react'

/**
 * 订阅后端 SSE 事件流（GET /api/sse）。
 * 收到事件后触发对应的状态更新（如刷新订单列表）。
 *
 * 事件类型（与后端 SseEvent 对应）：
 *   - order_created: { order_id }
 *   - order_updated: { order_id, status }
 */
export function useSSE() {
    useEffect(() => {
        // TODO:
        // const es = new EventSource('/api/sse')
        // es.onmessage = (e) => { const event = JSON.parse(e.data); ... }
        // es.onerror = () => { /* 重连逻辑 */ }
        // return () => es.close()
    }, [])
}
