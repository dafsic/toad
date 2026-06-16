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

export interface PageResponse {
    items: Order[]
    next_cursor: number | null
}

/** POST /api/orders — 手动下单 */
export function createOrder(req: CreateOrderRequest): Promise<Order> {
    return request('/orders', { method: 'POST', body: JSON.stringify(req) })
}

/** GET /api/orders — 查询订单列表（游标分页） */
export function listOrders(query?: ListOrdersQuery & { before_id?: number; limit?: number }): Promise<PageResponse> {
    const params = new URLSearchParams()
    if (query) {
        for (const [k, v] of Object.entries(query)) {
            if (v !== undefined && v !== '') params.set(k, String(v))
        }
    }
    const qs = params.size ? `?${params}` : ''
    return request(`/orders${qs}`)
}

/** DELETE /api/orders/:id — 取消挂单 */
export async function cancelOrder(id: number): Promise<void> {
    const res = await fetch(`${BASE}/orders/${id}`, { method: 'DELETE' })
    if (!res.ok) {
        const text = await res.text()
        throw new Error(`${res.status} ${text}`)
    }
}

