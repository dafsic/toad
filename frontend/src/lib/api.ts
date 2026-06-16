import type { Order, CreateOrderRequest, ListOrdersQuery } from '@/types/order'

const BASE = '/api'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
    const res = await fetch(`${BASE}${path}`, {
        headers: { 'Content-Type': 'application/json' },
        ...init,
    })
    if (!res.ok) {
        const text = await res.text()
        throw new Error(`${res.status} ${text}`)
    }
    return res.json() as Promise<T>
}

/** POST /api/orders — 手动下单 */
export function createOrder(req: CreateOrderRequest): Promise<Order> {
    return request('/orders', { method: 'POST', body: JSON.stringify(req) })
}

/** GET /api/orders — 查询订单列表 */
export function listOrders(query?: ListOrdersQuery): Promise<Order[]> {
    const params = new URLSearchParams(
        Object.entries(query ?? {}).filter(([, v]) => v != null) as [string, string][]
    )
    const qs = params.size ? `?${params}` : ''
    return request(`/orders${qs}`)
}

/** DELETE /api/orders/:id — 取消挂单 */
export function cancelOrder(id: number): Promise<void> {
    return request(`/orders/${id}`, { method: 'DELETE' })
}
