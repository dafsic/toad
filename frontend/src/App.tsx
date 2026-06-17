import { useCallback, useEffect, useState } from 'react'
import OrderForm from '@/components/OrderForm'
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
        <div className="min-h-screen bg-background p-4 lg:p-6">
            <header className="mb-6 flex items-center justify-between">
                <div>
                    <h1 className="text-lg font-bold tracking-tight">🐸 Toad Grid Bot</h1>
                    <p className="text-xs text-muted-foreground">XMR/USDC 无限链式反向网格</p>
                </div>
                <div className={`h-2 w-2 rounded-full ${state.error ? 'bg-red-500' : 'bg-green-500'}`}
                    title={state.error ?? 'connected'} />
            </header>

            <main className="grid gap-4 lg:grid-cols-[380px_1fr]">
                <OrderForm onCreated={onOrderCreated} />

                <div className="flex flex-col gap-3">
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

