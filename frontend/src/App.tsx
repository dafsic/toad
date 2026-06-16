import { useCallback, useEffect } from 'react'
import OrderForm from '@/components/OrderForm'
import OrderFilter from '@/components/OrderFilter'
import OrderList from '@/components/OrderList'
import { useOrders } from '@/hooks/useOrders'
import { useSSE } from '@/hooks/useSSE'

export default function App() {
    const { state, setFilters, loadMore, updateOrderStatus, onOrderCreated, fetchPage } = useOrders()

    // Initial load
    useEffect(() => {
        fetchPage(state.filters)
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    const handleSSECreated = useCallback(() => {
        onOrderCreated()
    }, [onOrderCreated])

    const handleSSEUpdated = useCallback((id: number, status: string) => {
        updateOrderStatus(id, status)
    }, [updateOrderStatus])

    useSSE(handleSSECreated, handleSSEUpdated)

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

