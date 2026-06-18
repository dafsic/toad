import { useEffect, useState, useRef } from 'react'
import type { Exchange } from '@/types/order'

interface PriceState {
    price: string | null
    loading: boolean
    error: boolean
}

async function fetchKrakenPrice(): Promise<string> {
    const res = await fetch('https://api.kraken.com/0/public/Ticker?pair=XMRUSDC')
    const json = await res.json()
    const price = json.result?.XMRUSDC?.c?.[0]
    if (!price) throw new Error('no price')
    return parseFloat(price).toFixed(2)
}

async function fetchHyperliquidPrice(): Promise<string> {
    const res = await fetch('https://api.hyperliquid.xyz/info', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ type: 'allMids' }),
    })
    const json = await res.json()
    const price = json['XMR']
    if (!price) throw new Error('no price')
    return parseFloat(price).toFixed(2)
}

const FETCHERS: Record<Exchange, () => Promise<string>> = {
    kraken: fetchKrakenPrice,
    hyperliquid: fetchHyperliquidPrice,
}

/** 轮询获取交易所 XMR/USDC 最新价格，每 15 秒更新一次 */
export function usePrice(exchange: Exchange): PriceState {
    const [state, setState] = useState<PriceState>({ price: null, loading: true, error: false })
    const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

    useEffect(() => {
        let alive = true

        async function fetch() {
            try {
                const price = await FETCHERS[exchange]()
                if (alive) setState({ price, loading: false, error: false })
            } catch {
                if (alive) setState(prev => ({ ...prev, loading: false, error: true }))
            }
        }

        fetch()
        timerRef.current = setInterval(fetch, 15_000)

        return () => {
            alive = false
            if (timerRef.current) clearInterval(timerRef.current)
        }
    }, [exchange])

    return state
}
