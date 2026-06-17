import { useCallback, useEffect, useState } from 'react'
import ExchangePanel from '@/components/ExchangePanel'
import OrderFilter from '@/components/OrderFilter'
import OrderList from '@/components/OrderList'
import { useOrders } from '@/hooks/useOrders'
import { useSSE } from '@/hooks/useSSE'
import { LoginPage } from '@/pages/LoginPage'

function getCookie(name: string): string | null {
    const value = `; ${document.cookie}`
    const parts = value.split(`; ${name}=`)
    if (parts.length === 2) return parts.pop()?.split(';').shift() ?? null
    return null
}

export default function App() {
    const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null)

    // 检查认证状态
    useEffect(() => {
        const token = getCookie('auth_token')
        setIsAuthenticated(!!token)
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

    // 检查中
    if (isAuthenticated === null) {
        return (
            <div className="min-h-screen flex items-center justify-center">
                <div className="text-muted-foreground">Loading...</div>
            </div>
        )
    }

    // 未认证，显示登录页
    if (!isAuthenticated) {
        return <LoginPage />
    }

    // 已认证，显示主页面
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
                {/* Two exchange panels side by side */}
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                    <ExchangePanel exchange="kraken" onCreated={onOrderCreated} />
                    <ExchangePanel exchange="hyperliquid" onCreated={onOrderCreated} />
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
                    />
                </div>
            </main>
        </div>
    )
}

