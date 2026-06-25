import { useState } from 'react'
import { cancelOrder, deleteOrder } from '@/lib/api'
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
    onDeleted: (id: number) => void
}

// Status → DESIGN.md accent palette
const STATUS_COLORS: Record<OrderStatus, string> = {
    pending: 'text-accent-yellow',
    open: 'text-accent-light-blue',
    partially_filled: 'text-accent-warning',
    filled: 'text-accent-light-green',
    cancelled: 'text-on-dark-mute',
    failed: 'text-accent-danger',
}

const STATUS_LABELS: Record<OrderStatus, string> = {
    pending: 'Pending',
    open: 'Open',
    partially_filled: 'Partial',
    filled: 'Filled',
    cancelled: 'Cancelled',
    failed: 'Failed',
}

const STATUS_DOTS: Record<OrderStatus, string> = {
    pending: 'bg-accent-yellow',
    open: 'bg-accent-light-blue',
    partially_filled: 'bg-accent-warning',
    filled: 'bg-accent-light-green',
    cancelled: 'bg-on-dark-mute',
    failed: 'bg-accent-danger',
}

export default function OrderList({ items, loading, error, nextCursor, onLoadMore, onCancelled, onDeleted }: Props) {
    const [cancelling, setCancelling] = useState<Set<number>>(new Set())
    const [deleting, setDeleting] = useState<Set<number>>(new Set())
    const [actionError, setActionError] = useState<string | null>(null)

    async function handleCancel(id: number) {
        setActionError(null)
        setCancelling(s => new Set(s).add(id))
        try {
            await cancelOrder(id)
            onCancelled(id)
        } catch (e) {
            setActionError(`Cancel #${id} failed: ${e}`)
        } finally {
            setCancelling(s => { const n = new Set(s); n.delete(id); return n })
        }
    }

    async function handleDelete(id: number) {
        setActionError(null)
        setDeleting(s => new Set(s).add(id))
        try {
            await deleteOrder(id)
            onDeleted(id)
        } catch (e) {
            setActionError(`Delete #${id} failed: ${e}`)
        } finally {
            setDeleting(s => { const n = new Set(s); n.delete(id); return n })
        }
    }

    if (error) {
        return (
            <div className="rounded-lg border border-hairline-dark bg-surface-elevated p-6 text-accent-danger text-sm">
                {error}
            </div>
        )
    }

    return (
        // feature-card-dark
        <div className="bg-surface-elevated rounded-lg border border-hairline-dark flex flex-col overflow-hidden">
            <div className="px-6 py-4 border-b border-hairline-dark bg-surface-deep flex items-center justify-between">
                <span className="font-semibold text-base font-display tracking-tight">Orders</span>
                {loading && <span className="text-xs text-on-dark-mute animate-pulse">Loading…</span>}
            </div>

            {actionError && (
                <div className="px-6 py-3 text-sm text-accent-danger border-b border-hairline-dark bg-accent-danger/5">
                    {actionError}
                </div>
            )}

            {items.length === 0 && !loading ? (
                <div className="px-6 py-16 text-center text-on-dark-mute text-sm">No orders yet</div>
            ) : (
                <>
                    {/* Header row — body-sm, on-dark-mute */}
                    <div className="grid grid-cols-[44px_1fr_1fr_1fr_1fr_1fr_1.2fr_72px] gap-3 px-6 py-3 border-b border-hairline-dark bg-surface-deep text-xs text-on-dark-mute">
                        <span>ID</span>
                        <span>Exchange</span>
                        <span>Side</span>
                        <span className="text-right">Quantity</span>
                        <span className="text-right">Price</span>
                        <span className="text-right">Δ / ×</span>
                        <span>Status</span>
                        <span></span>
                    </div>

                    <div className="divide-y divide-divider-soft overflow-y-auto max-h-[60vh]">
                        {items.map(order => (
                            <OrderRow
                                key={order.id}
                                order={order}
                                cancelling={cancelling.has(order.id)}
                                deleting={deleting.has(order.id)}
                                onCancel={() => handleCancel(order.id)}
                                onDelete={() => handleDelete(order.id)}
                            />
                        ))}
                    </div>

                    {nextCursor && (
                        <div className="px-6 py-4 border-t border-hairline-dark">
                            <button
                                onClick={onLoadMore}
                                disabled={loading}
                                // button-outline-dark pill
                                className="w-full rounded-full border border-on-dark py-2.5 text-sm text-on-dark hover:bg-on-dark hover:text-canvas-dark transition-colors disabled:opacity-50"
                            >
                                {loading ? 'Loading…' : 'Load more'}
                            </button>
                        </div>
                    )}
                </>
            )}
        </div>
    )
}

function OrderRow({ order, cancelling, deleting, onCancel, onDelete }: {
    order: Order
    cancelling: boolean
    deleting: boolean
    onCancel: () => void
    onDelete: () => void
}) {
    const sideColor = order.side === 'buy' ? 'text-accent-light-green' : 'text-accent-danger'
    const canDelete = order.status === 'filled' || order.status === 'cancelled' || order.status === 'failed'

    return (
        <div className={cn(
            'grid grid-cols-[44px_1fr_1fr_1fr_1fr_1fr_1.2fr_72px] gap-3 px-6 py-3.5 text-sm hover:bg-surface-deep transition-colors items-center',
            order.status === 'filled' && 'opacity-60',
            order.status === 'cancelled' && 'opacity-40',
        )}>
            <span className="text-on-dark-mute font-mono">
                #{order.id}
                {order.is_auto && <span className="ml-1 text-xs opacity-60">🤖</span>}
            </span>
            <span className="flex items-center gap-2 truncate">
                <ExchangeLogo exchange={order.exchange} size={16} />
                <span className="text-sm">
                    {order.exchange === 'kraken' ? 'Kraken'
                        : order.exchange === 'hyperliquid' ? 'HL'
                        : 'MEXC'}
                </span>
            </span>
            <span className={cn('font-medium', sideColor)}>
                {order.side === 'buy' ? 'Buy' : 'Sell'}
            </span>
            <span className="text-right font-mono">{order.quantity.toFixed(4)}</span>
            <span className="text-right font-mono">
                {order.filled_price != null
                    ? <span className="text-accent-light-green">{order.filled_price.toFixed(4)}</span>
                    : order.price.toFixed(4)
                }
            </span>
            <span className="text-right font-mono text-on-dark-mute">
                {order.price_change.toFixed(2)}
                {order.leverage > 1 && <span className="ml-1 text-accent-yellow">×{order.leverage}</span>}
            </span>
            <span className={cn('font-medium flex items-center gap-1.5', STATUS_COLORS[order.status as OrderStatus])}>
                <span className={cn('h-1.5 w-1.5 rounded-full flex-shrink-0', STATUS_DOTS[order.status as OrderStatus])} />
                {STATUS_LABELS[order.status as OrderStatus] ?? order.status}
            </span>
            <div className="flex justify-end">
                {order.status === 'open' && (
                    <button
                        onClick={onCancel}
                        disabled={cancelling}
                        // outline pill in danger
                        className="rounded-full px-3 py-1 text-xs text-accent-danger border border-accent-danger/40 hover:bg-accent-danger/10 transition-colors disabled:opacity-40"
                    >
                        {cancelling ? '…' : 'Cancel'}
                    </button>
                )}
                {canDelete && (
                    <button
                        onClick={onDelete}
                        disabled={deleting}
                        // subtle outline pill
                        className="rounded-full px-3 py-1 text-xs text-on-dark-mute border border-hairline-dark hover:text-accent-danger hover:border-accent-danger/40 hover:bg-accent-danger/10 transition-colors disabled:opacity-40"
                    >
                        {deleting ? '…' : 'Delete'}
                    </button>
                )}
            </div>
        </div>
    )
}
