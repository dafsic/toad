export type Exchange = 'kraken' | 'hyperliquid' | 'mexc_spot'
export type Side = 'buy' | 'sell'
export type OrderStatus = 'pending' | 'open' | 'partially_filled' | 'filled' | 'cancelled' | 'failed'

/** Spot exchanges: UI uses this to hide leverage slider (spot leverage is fixed at 1) */
export const SPOT_EXCHANGES: Exchange[] = ['kraken', 'mexc_spot']

/** Display names for exchanges */
export const EXCHANGE_LABELS: Record<Exchange, string> = {
    kraken: 'Kraken',
    hyperliquid: 'Hyperliquid',
    mexc_spot: 'MEXC',
}

export interface Order {
    id: number
    exchange: Exchange
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
    exchange: Exchange
    side: Side
    quantity: number
    price: number
    price_change: number
    /** Leverage: send 1 for Kraken, actual (>=1) for Hyperliquid. Default 1. */
    leverage?: number
}

export interface ListOrdersQuery {
    exchange?: Exchange
    side?: Side
    status?: OrderStatus
    is_auto?: boolean
}

