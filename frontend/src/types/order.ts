export type Exchange = 'kraken' | 'hyperliquid' | 'mexc_spot'
export type Side = 'buy' | 'sell'
export type OrderStatus = 'pending' | 'open' | 'partially_filled' | 'filled' | 'cancelled' | 'failed'

/** 现货交易所列表：UI 据此隐藏杠杆滑块（现货杠杆固定为 1） */
export const SPOT_EXCHANGES: Exchange[] = ['kraken', 'mexc_spot']

/** 交易所显示名称 */
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

