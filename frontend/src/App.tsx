import { useCallback, useEffect, useState } from 'react'
import ExchangePanel from '@/components/ExchangePanel'
import OrderFilter from '@/components/OrderFilter'
import OrderList from '@/components/OrderList'
import { useOrders } from '@/hooks/useOrders'
import { useSSE } from '@/hooks/useSSE'
import { LoginPage } from '@/pages/LoginPage'

export default function App() {
    const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null)

    // Check auth status by probing a protected endpoint.
    // The JWT is stored in an HttpOnly cookie (set by the server), so we cannot
    // read it from JavaScript. A 200 response means we are authenticated.
    useEffect(() => {
        fetch('/api/orders?limit=1')
            .then((r) => setIsAuthenticated(r.ok))
            .catch(() => setIsAuthenticated(false))
    }, [])

    const { state, setFilters, loadMore, updateOrderStatus, onOrderCreated, fetchPage } = useOrders()

    // Initial load
    useEffect(() => {
        if (isAuthenticated) {
            fetchPage(state.filters)
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [isAuthenticated])

    const handleSSECreated = useCallback(() => {
        onOrderCreated()
    }, [onOrderCreated])

    const handleSSEUpdated = useCallback((id: number, status: string) => {
        updateOrderStatus(id, status)
    }, [updateOrderStatus])

    useSSE(handleSSECreated, handleSSEUpdated)

    // Checking
    if (isAuthenticated === null) {
        return (
            <div className="min-h-screen flex items-center justify-center">
                <div className="text-muted-foreground">Loading...</div>
            </div>
        )
    }

    // Not authenticated, show login page
    if (!isAuthenticated) {
        return <LoginPage />
    }

    // Authenticated, show main page
    return (
        <div className="min-h-screen bg-background font-mono">
            <header className="px-6 py-4 border-b border-border flex items-center justify-between">
                <div className="flex items-center gap-3">
                    <span className="text-xl">🐸</span>
                    <span className="text-border select-none">·</span>
                    <span className="text-sm font-semibold text-xmr">XMR/USDC</span>
                    <span className="text-xs text-muted-foreground">GRID BOT</span>
                </div>
                <div className="flex items-center gap-2">
                    <span className={`h-2 w-2 rounded-full flex-shrink-0 ${state.error ? 'bg-red-500' : 'bg-green-500'}`} />
                    <span className="text-xs text-muted-foreground tracking-widest uppercase">
                        {state.error ? 'OFFLINE' : 'LIVE'}
                    </span>
                </div>
            </header>

            <main className="p-4 lg:p-6 space-y-4">
                {/* Exchange panels side by side */}
                <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
                    <ExchangePanel exchange="kraken" onCreated={onOrderCreated} />
                    <ExchangePanel exchange="hyperliquid" onCreated={onOrderCreated} />
                    <ExchangePanel exchange="mexc_spot" onCreated={onOrderCreated} />
                </div>

                {/* Order list spanning full width */}
                <div className="space-y-3">
                    <OrderFilter filters={state.filters} onChange={setFilters} />
                    <OrderList
                        items={state.items}
                        loading={state.loading}
                        error={state.error}
                        nextCursor={state.nextCursor}
                        onLoadMore={loadMore}
                        onCancelled={(id) => updateOrderStatus(id, 'cancelled')}
                        onDeleted={onOrderCreated}
                    />
                </div>
            </main>
        </div>
    )
}

