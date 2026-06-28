import type { ExchangeInfo } from './order'

/** 前端展示用的交易对记录（目前只有 XMR/USDC，但结构预留多交易对扩展） */
export interface TradingPair {
    symbol: string
    kind: 'spot' | 'perp'
    exchange: ExchangeInfo
}

/** 将后端返回的 ExchangeInfo 转换为交易对记录 */
export function exchangeToPair(exchange: ExchangeInfo): TradingPair {
    return {
        symbol: 'XMR/USDC',
        kind: exchange.kind,
        exchange,
    }
}
