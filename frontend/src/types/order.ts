// Exchange identifiers are now dynamic (driven by GET /api/exchanges),
// so the union is relaxed to string. The known set is kept for compatibility.
export type Exchange = 'kraken' | 'hyperliquid' | 'mexc_spot'
export type Side = 'buy' | 'sell'
export type OrderStatus = 'pending' | 'open' | 'partially_filled' | 'filled' | 'cancelled' | 'failed'

/** Enabled-exchange descriptor returned by GET /api/exchanges */
export interface ExchangeInfo {
    name: string
    kind: 'spot' | 'perp'
    label: string
}

/** Spot exchanges: UI uses this to hide leverage slider (spot leverage is fixed at 1) */
export const SPOT_EXCHANGES: string[] = ['kraken', 'mexc_spot']

/** Display names for exchanges */
export const EXCHANGE_LABELS: Record<string, string> = {
    kraken: 'Kraken',
    hyperliquid: 'Hyperliquid',
    mexc_spot: 'MEXC',
}

export interface Order {
    id: number
    exchange: string
    symbol: string
    side: Side
    quantity: number
    price: number
    price_change: number
    /** Leverage: Kraken fixed at 1, Hyperliquid perp user-specified */
    leverage: number
    is_auto: boolean
    parent_order_id: number | null
    exchange_order_id: string | null
    status: OrderStatus
    filled_price: number | null
    /** Cumulative filled quantity (updated in realtime from WebSocket) */
    filled_quantity: number
    created_at: string
    updated_at: string
}

export interface CreateOrderRequest {
    exchange: string
    side: Side
    quantity: number
    price: number
    price_change: number
    /** Leverage: send 1 for Kraken, actual (>=1) for Hyperliquid. Default 1. */
    leverage?: number
}

export interface ListOrdersQuery {
    exchange?: string
    side?: Side
    status?: OrderStatus
    is_auto?: boolean
}

