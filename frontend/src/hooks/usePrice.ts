import { useEffect, useState, useRef } from 'react'

interface PriceState {
    price: string | null
    loading: boolean
    error: boolean
}

/** Poll for latest XMR/USDC price from exchange, every 15 seconds.
 *
 * Uses backend proxy `/api/price/:exchange` to avoid CORS issues when the
 * browser talks directly to exchange APIs (MEXC does not send CORS headers).
 */
export function usePrice(exchange: string): PriceState {
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
