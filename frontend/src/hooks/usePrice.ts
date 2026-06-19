import { useEffect, useState, useRef } from 'react'
import type { Exchange } from '@/types/order'

interface PriceState {
    price: string | null
    loading: boolean
    error: boolean
}

/** 轮询获取交易所 XMR/USDC 最新价格，每 15 秒更新一次。
 *
 * 通过后端代理 `/api/price/:exchange` 获取，避免浏览器直连交易所 API
 * 时的 CORS 限制（MEXC 不发送 CORS 头）。
 */
export function usePrice(exchange: Exchange): PriceState {
    const [state, setState] = useState<PriceState>({ price: null, loading: true, error: false })
    const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

    useEffect(() => {
        let alive = true

        async function poll() {
            try {
                const res = await fetch(`/api/price/${exchange}`)
                if (!res.ok) throw new Error(`${res.status}`)
                const json: { price: string } = await res.json()
                if (alive) setState({ price: json.price, loading: false, error: false })
            } catch {
                if (alive) setState(prev => ({ ...prev, loading: false, error: true }))
            }
        }

        poll()
        timerRef.current = setInterval(poll, 15_000)

        return () => {
            alive = false
            if (timerRef.current) clearInterval(timerRef.current)
        }
    }, [exchange])

    return state
}
