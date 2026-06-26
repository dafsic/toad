import type { Order, CreateOrderRequest, ListOrdersQuery, ExchangeInfo } from '@/types/order'

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

/** GET /api/exchanges — list enabled exchanges (public, no auth) */
export function listExchanges(): Promise<ExchangeInfo[]> {
    return request('/exchanges')
}

/** POST /api/orders — manual order placement */
export function createOrder(req: CreateOrderRequest): Promise<Order> {
    return request('/orders', { method: 'POST', body: JSON.stringify(req) })
}

/** GET /api/orders — list orders (cursor pagination) */
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

/** DELETE /api/orders/:id — cancel open order */
export async function cancelOrder(id: number): Promise<void> {
    const res = await fetch(`${BASE}/orders/${id}`, { method: 'DELETE' })
    if (!res.ok) {
        const text = await res.text()
        throw new Error(`${res.status} ${text}`)
    }
}

/** DELETE /api/orders/:id/hard — hard delete terminal orders (filled/cancelled/failed) */
export async function deleteOrder(id: number): Promise<void> {
    const res = await fetch(`${BASE}/orders/${id}/hard`, { method: 'DELETE' })
    if (!res.ok) {
        const text = await res.text()
        throw new Error(`${res.status} ${text}`)
    }
}

