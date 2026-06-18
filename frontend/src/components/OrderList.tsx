import { useState } from 'react'
import { cancelOrder } from '@/lib/api'
import type { Order, OrderStatus } from '@/types/order'
import { cn } from '@/lib/utils'
import ExchangeLogo from '@/components/ExchangeLogo'

interface Props {
    items: Order[]
    loading: boolean
    error: string | null
    nextCursor: number | null
    onLoadMore: () => void
    onCancelled: (id: number) => void
}

const STATUS_COLORS: Record<OrderStatus, string> = {
    pending: 'text-yellow-400',
    open: 'text-blue-400',
    partially_filled: 'text-cyan-400',
    filled: 'text-green-400',
    cancelled: 'text-muted-foreground',
    failed: 'text-red-400',
}

const STATUS_LABELS: Record<OrderStatus, string> = {
    pending: '等待',
    open: '挂单',
    partially_filled: '部分成交',
    filled: '成交',
    cancelled: '取消',
    failed: '失败',
}

export default function OrderList({ items, loading, error, nextCursor, onLoadMore, onCancelled }: Props) {
    const [cancelling, setCancelling] = useState<Set<number>>(new Set())
    const [cancelError, setCancelError] = useState<string | null>(null)

    async function handleCancel(id: number) {
        setCancelError(null)
        setCancelling(s => new Set(s).add(id))
        try {
            await cancelOrder(id)
            onCancelled(id)
        } catch (e) {
            setCancelError(`取消 #${id} 失败: ${e}`)
        } finally {
            setCancelling(s => { const n = new Set(s); n.delete(id); return n })
        }
    }

    if (error) {
        return <div className="rounded-lg border bg-card p-4 text-red-400 text-xs">{error}</div>
    }

    return (
        <div className="border border-border bg-card rounded-xl shadow-sm flex flex-col">
            <div className="px-4 py-3 border-b border-border bg-secondary flex items-center justify-between">
                <span className="font-bold text-sm tracking-wide">Orders</span>
                {loading && <span className="text-xs text-muted-foreground animate-pulse">加载中…</span>}
            </div>

            {cancelError && (
                <div className="px-4 py-2 text-xs text-red-400 border-b">{cancelError}</div>
            )}

            {items.length === 0 && !loading ? (
                <div className="px-4 py-10 text-center text-muted-foreground text-xs tracking-wider">NO ORDERS YET</div>
            ) : (
                <>
                    {/* Header */}
                    <div className="grid grid-cols-[40px_1fr_1fr_1fr_1fr_1fr_1fr_56px] gap-2 px-4 py-2 border-b border-border bg-secondary text-xs text-muted-foreground tracking-widest uppercase">
                        <span>ID</span>
                        <span>交易所</span>
                        <span>方向</span>
                        <span className="text-right">数量</span>
                        <span className="text-right">价格</span>
                        <span className="text-right">价差/×</span>
                        <span>状态</span>
                        <span></span>
                    </div>

                    <div className="divide-y divide-border/50 overflow-y-auto max-h-[60vh]">
                        {items.map(order => (
                            <OrderRow
                                key={order.id}
                                order={order}
                                cancelling={cancelling.has(order.id)}
                                onCancel={() => handleCancel(order.id)}
                            />
                        ))}
                    </div>

                    {nextCursor && (
                        <div className="px-4 py-3 border-t">
                            <button
                                onClick={onLoadMore}
                                disabled={loading}
                                className="w-full rounded-md border py-1.5 text-xs text-muted-foreground hover:bg-secondary transition-colors disabled:opacity-50"
                            >
                                {loading ? '加载中…' : '加载更多'}
                            </button>
                        </div>
                    )}
                </>
            )}
        </div>
    )
}

function OrderRow({ order, cancelling, onCancel }: {
    order: Order
    cancelling: boolean
    onCancel: () => void
}) {
    const sideColor = order.side === 'buy' ? 'text-green-400' : 'text-red-400'

    return (
        <div className={cn(
            'grid grid-cols-[40px_1fr_1fr_1fr_1fr_1fr_1fr_56px] gap-2 px-4 py-2.5 text-xs hover:bg-secondary/60 transition-colors items-center border-b border-border/50 last:border-0',
            order.status === 'filled' && 'opacity-60',
            order.status === 'cancelled' && 'opacity-40',
        )}>
            <span className="text-muted-foreground font-mono">
                #{order.id}
                {order.is_auto && <span className="ml-1 text-[10px] opacity-60">🤖</span>}
            </span>
            <span className="flex items-center gap-1.5 truncate">
                <ExchangeLogo exchange={order.exchange} size={14} />
                <span className="capitalize text-xs">{order.exchange === 'kraken' ? 'Kraken' : 'HL'}</span>
            </span>
            <span className={cn('font-medium', sideColor)}>
                {order.side === 'buy' ? '买' : '卖'}
            </span>
            <span className="text-right font-mono">{order.quantity.toFixed(4)}</span>
            <span className="text-right font-mono">
                {order.filled_price != null
                    ? <span className="text-green-400">{order.filled_price.toFixed(4)}</span>
                    : order.price.toFixed(4)
                }
            </span>
            <span className="text-right font-mono text-muted-foreground">
                {order.price_change.toFixed(2)}
                {order.leverage > 1 && <span className="ml-0.5 text-yellow-400">×{order.leverage}</span>}
            </span>
            <span className={cn('font-medium', STATUS_COLORS[order.status as OrderStatus])}>
                {STATUS_LABELS[order.status as OrderStatus] ?? order.status}
            </span>
            <div className="flex justify-end">
                {order.status === 'open' && (
                    <button
                        onClick={onCancel}
                        disabled={cancelling}
                        className="rounded px-2 py-0.5 text-[10px] text-red-400 border border-red-400/30 hover:bg-red-400/10 transition-colors disabled:opacity-40"
                    >
                        {cancelling ? '…' : '取消'}
                    </button>
                )}
            </div>
        </div>
    )
}
