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

const STATUS_COLORS: Record<OrderStatus, string> = {
    pending: 'text-yellow-400',
    open: 'text-blue-400',
    partially_filled: 'text-cyan-400',
    filled: 'text-green-400',
    cancelled: 'text-muted-foreground',
    failed: 'text-red-400',
}

const STATUS_LABELS: Record<OrderStatus, string> = {
    pending: 'Pending',
    open: 'Open',
    partially_filled: 'Partial',
    filled: 'Filled',
    cancelled: 'Cancelled',
    failed: 'Failed',
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
        return <div className="rounded-lg border bg-card p-4 text-red-400 text-xs">{error}</div>
    }

    return (
        <div className="border border-border bg-card rounded-xl shadow-sm flex flex-col">
            <div className="px-4 py-3 border-b border-border bg-secondary flex items-center justify-between">
                <span className="font-bold text-sm tracking-wide">Orders</span>
                {loading && <span className="text-xs text-muted-foreground animate-pulse">Loading…</span>}
            </div>

            {actionError && (
                <div className="px-4 py-2 text-xs text-red-400 border-b">{actionError}</div>
            )}

            {items.length === 0 && !loading ? (
                <div className="px-4 py-10 text-center text-muted-foreground text-xs tracking-wider">NO ORDERS YET</div>
            ) : (
                <>
                    {/* Header */}
                    <div className="grid grid-cols-[40px_1fr_1fr_1fr_1fr_1fr_1fr_56px] gap-2 px-4 py-2 border-b border-border bg-secondary text-xs text-muted-foreground tracking-widest uppercase">
                        <span>ID</span>
                        <span>Exchange</span>
                        <span>Side</span>
                        <span className="text-right">Quantity</span>
                        <span className="text-right">Price</span>
                        <span className="text-right">Δ/×</span>
                        <span>Status</span>
                        <span></span>
                    </div>

                    <div className="divide-y divide-border/50 overflow-y-auto max-h-[60vh]">
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
                        <div className="px-4 py-3 border-t">
                            <button
                                onClick={onLoadMore}
                                disabled={loading}
                                className="w-full rounded-md border py-1.5 text-xs text-muted-foreground hover:bg-secondary transition-colors disabled:opacity-50"
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
    const sideColor = order.side === 'buy' ? 'text-green-400' : 'text-red-400'
    const canDelete = order.status === 'filled' || order.status === 'cancelled' || order.status === 'failed'

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
                {order.side === 'buy' ? 'Buy' : 'Sell'}
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
                        {cancelling ? '…' : 'Cancel'}
                    </button>
                )}
                {canDelete && (
                    <button
                        onClick={onDelete}
                        disabled={deleting}
                        className="rounded px-2 py-0.5 text-[10px] text-muted-foreground border border-border hover:text-red-400 hover:border-red-400/30 hover:bg-red-400/10 transition-colors disabled:opacity-40"
                    >
                        {deleting ? '…' : 'Delete'}
                    </button>
                )}
            </div>
        </div>
    )
}
