import { useEffect, useState } from 'react'
import { listExchanges } from '@/lib/api'
import type { ExchangeInfo } from '@/types/order'

interface ExchangesState {
    exchanges: ExchangeInfo[]
    loading: boolean
    error: string | null
}

/** Fetch the list of enabled exchanges once on mount.
 *
 * Drives dynamic rendering of exchange panels and the filter dropdown,
 * so the UI only shows exchanges that the backend has configured API
 * credentials for (see GET /api/exchanges).
 */
export function useExchanges(): ExchangesState {
    const [state, setState] = useState<ExchangesState>({
        exchanges: [],
        loading: true,
        error: null,
    })

    useEffect(() => {
        let alive = true
        listExchanges()
            .then((exchanges) => {
                if (alive) setState({ exchanges, loading: false, error: null })
            })
            .catch((e) => {
                if (alive) setState({ exchanges: [], loading: false, error: String(e) })
            })
        return () => {
            alive = false
        }
    }, [])

    return state
}
