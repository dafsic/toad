import { useEffect, useCallback } from 'react'
import { useParams, Link } from 'react-router-dom'
import { useExchanges } from '@/hooks/useExchanges'
import { useOrders } from '@/hooks/useOrders'
import { useSSE } from '@/hooks/useSSE'
import ExchangePanel from '@/components/ExchangePanel'
import ExchangeLogo from '@/components/ExchangeLogo'
import OrderFilter from '@/components/OrderFilter'
import OrderList from '@/components/OrderList'
import { ChevronLeft } from 'lucide-react'
import type { Side } from '@/types/order'

export default function TradeAndHistoryPage() {
    const { exchange } = useParams<{ exchange: string }>()
    const { exchanges, loading: exchangesLoading, error: exchangesError } = useExchanges()
    const {
        state,
        setFilters,
        loadMore,
        updateOrderStatus,
        onOrderCreated,
    } = useOrders()

    const info = exchanges.find(e => e.name === exchange)

    // 初始化：默认过滤当前交易所，状态显示 all
    useEffect(() => {
        if (!exchange) return
        const initialFilters = {
            exchange,
            side: '' as Side | '',
            status: '' as '' | 'pending' | 'open' | 'partially_filled' | 'filled' | 'cancelled' | 'failed',
            is_auto: undefined as boolean | undefined,
        }
        setFilters(initialFilters)
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [exchange])

    const panels = exchanges.map(ex => ({ ...ex, enabled: true }))

    const handleSSECreated = useCallback(() => {
        onOrderCreated()
    }, [onOrderCreated])

    const handleSSEUpdated = useCallback((id: number, status: string) => {
        updateOrderStatus(id, status)
    }, [updateOrderStatus])

    useSSE(handleSSECreated, handleSSEUpdated)

    const handleOrderCreated = useCallback(() => {
        // 刷新当前过滤条件下的订单列表
        onOrderCreated()
    }, [onOrderCreated])

    if (exchangesLoading) {
        return (
            <div className="min-h-screen flex items-center justify-center bg-canvas-dark text-on-dark-mute text-sm">
                Loading…
            </div>
        )
    }

    if (exchangesError || !info) {
        return (
            <div className="min-h-screen bg-canvas-dark text-on-dark font-sans flex flex-col">
                <Header exchange={exchange ?? ''} />
                <main className="flex-1 flex items-center justify-center p-6 text-accent-danger text-sm">
                    {exchangesError || `Exchange '${exchange}' is not enabled.`}
                </main>
            </div>
        )
    }

    return (
        <div className="min-h-screen bg-canvas-dark text-on-dark font-sans">
            <Header exchange={info.name} />

            <main className="p-6 lg:p-8 max-w-[1400px] mx-auto space-y-6">
                {/* 页面标题 */}
                <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                        <ExchangeLogo exchange={info.name} size={32} />
                        <div>
                            <h1 className="text-lg font-semibold font-display tracking-tight">{info.label}</h1>
                            <p className="text-sm text-on-dark-mute">
                                {info.kind === 'spot' ? 'Spot' : 'Perpetual'} · XMR/USDC
                            </p>
                        </div>
                    </div>
                    <Link
                        to="/"
                        className="inline-flex items-center gap-1 text-sm text-on-dark-mute hover:text-primary transition-colors"
                    >
                        <ChevronLeft size={16} /> Back to pairs
                    </Link>
                </div>

                {/* 上部：下单区 */}
                <section>
                    <ExchangePanel
                        exchange={info.name}
                        kind={info.kind}
                        label={info.label}
                        enabled
                        onCreated={handleOrderCreated}
                    />
                </section>

                {/* 下部：订单记录区 */}
                <section className="space-y-4">
                    <div className="flex items-center justify-between">
                        <h2 className="text-base font-semibold font-display tracking-tight">Order History</h2>
                    </div>
                    <OrderFilter filters={state.filters} exchanges={panels} onChange={setFilters} />
                    <OrderList
                        items={state.items}
                        loading={state.loading}
                        error={state.error}
                        nextCursor={state.nextCursor}
                        onLoadMore={loadMore}
                        onCancelled={(id) => updateOrderStatus(id, 'cancelled')}
                        onDeleted={onOrderCreated}
                    />
                </section>
            </main>
        </div>
    )
}

function Header({ exchange }: { exchange: string }) {
    return (
        <header className="h-16 px-6 lg:px-8 border-b border-hairline-dark flex items-center justify-between sticky top-0 bg-canvas-dark z-10">
            <div className="flex items-center gap-3">
                <Link to="/" className="text-xl leading-none hover:opacity-80">🐸</Link>
                <span className="text-hairline-dark select-none">·</span>
                <span className="text-sm font-semibold text-primary font-display tracking-tight">XMR/USDC</span>
                <span className="text-xs text-on-dark-mute">Grid Bot</span>
            </div>
            <nav className="flex items-center gap-4">
                <Link
                    to="/"
                    className="text-sm font-medium text-on-dark-mute hover:text-primary transition-colors"
                >
                    Pairs
                </Link>
                <Link
                    to={`/trade/${exchange}`}
                    className="text-sm font-medium text-on-dark hover:text-primary transition-colors"
                >
                    Trade
                </Link>
            </nav>
        </header>
    )
}
