export type Exchange = 'kraken' | 'hyperliquid'
export type Side = 'buy' | 'sell'
export type OrderStatus = 'pending' | 'open' | 'partially_filled' | 'filled' | 'cancelled' | 'failed'

export interface Order {
    id: number
    exchange: Exchange
    symbol: string
    side: Side
    quantity: number
    price: number
    price_change: number
    /** 杠杆倍数；Kraken 固定为 1，Hyperliquid 永续合约由用户指定 */
    leverage: number
    is_auto: boolean
    parent_order_id: number | null
    exchange_order_id: string | null
    status: OrderStatus
    filled_price: number | null
    /** 累计已成交数量（由 WebSocket 实时更新） */
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
    /** 杠杆倍数；Kraken 传 1，Hyperliquid 传实际杠杆（≥1）。默认 1。 */
    leverage?: number
}

export interface ListOrdersQuery {
    exchange?: Exchange
    side?: Side
    status?: OrderStatus
    is_auto?: boolean
}

