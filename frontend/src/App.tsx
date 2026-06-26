import { useCallback, useEffect, useState } from 'react'
import ExchangePanel from '@/components/ExchangePanel'
import OrderFilter from '@/components/OrderFilter'
import OrderList from '@/components/OrderList'
import { useOrders } from '@/hooks/useOrders'
import { useSSE } from '@/hooks/useSSE'
import { useExchanges } from '@/hooks/useExchanges'
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
    const { exchanges } = useExchanges()

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
            <div className="min-h-screen flex items-center justify-center bg-canvas-dark font-sans">
                <div className="text-on-dark-mute text-sm">Loading…</div>
            </div>
        )
    }

    // Not authenticated, show login page
    if (!isAuthenticated) {
        return <LoginPage />
    }

    // Authenticated, show main page
    return (
        <div className="min-h-screen bg-canvas-dark text-on-dark font-sans">
            {/* nav-bar — 64px, canvas-dark, hairline-dark divider */}
            <header className="h-16 px-6 lg:px-8 border-b border-hairline-dark flex items-center justify-between sticky top-0 bg-canvas-dark z-10">
                <div className="flex items-center gap-3">
                    <span className="text-xl leading-none">🐸</span>
                    <span className="text-hairline-dark select-none">·</span>
                    <span className="text-sm font-semibold text-primary font-display tracking-tight">XMR/USDC</span>
                    <span className="text-xs text-on-dark-mute">Grid Bot</span>
                </div>
                <div className="flex items-center gap-2">
                    <span
                        className={`h-2 w-2 rounded-full flex-shrink-0 ${state.error ? 'bg-accent-danger' : 'bg-accent-light-green'}`}
                    />
                    <span className="text-xs text-on-dark-mute uppercase tracking-widest">
                        {state.error ? 'Offline' : 'Live'}
                    </span>
                </div>
            </header>

            <main className="p-6 lg:p-8 space-y-6 max-w-[1400px] mx-auto">
                {/* Exchange panels — driven by enabled exchanges from backend */}
                {exchanges.length > 0 ? (
                    <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
                        {exchanges.map((ex) => (
                            <ExchangePanel
                                key={ex.name}
                                exchange={ex.name}
                                kind={ex.kind}
                                label={ex.label}
                                onCreated={onOrderCreated}
                            />
                        ))}
                    </div>
                ) : (
                    <div className="bg-surface-elevated rounded-lg border border-hairline-dark p-8 text-center">
                        <p className="text-sm text-on-dark-mute">
                            No exchanges enabled. Set API credentials (e.g. <code className="text-on-dark">KRAKEN_API_KEY</code> / <code className="text-on-dark">KRAKEN_API_SECRET</code>) in your <code className="text-on-dark">.env</code> and restart.
                        </p>
                    </div>
                )}

                {/* Order list spanning full width */}
                <div className="space-y-4">
                    <OrderFilter filters={state.filters} exchanges={exchanges} onChange={setFilters} />
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
