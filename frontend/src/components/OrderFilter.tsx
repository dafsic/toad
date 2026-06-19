import type { OrderFilters } from '@/hooks/useOrders'
import type { Exchange, OrderStatus, Side } from '@/types/order'
import { cn } from '@/lib/utils'

interface Props {
    filters: OrderFilters
    onChange: (f: OrderFilters) => void
}

const STATUSES: { value: OrderStatus | ''; label: string }[] = [
    { value: '', label: 'All' },
    { value: 'open', label: 'Open' },
    { value: 'partially_filled', label: 'Partial' },
    { value: 'filled', label: 'Filled' },
    { value: 'cancelled', label: 'Cancelled' },
    { value: 'failed', label: 'Failed' },
    { value: 'pending', label: 'Pending' },
]

export default function OrderFilter({ filters, onChange }: Props) {
    function set<K extends keyof OrderFilters>(key: K, value: OrderFilters[K]) {
        onChange({ ...filters, [key]: value })
    }

    return (
        <div className="flex flex-wrap items-center gap-2">
            {/* Status tabs */}
            <div className="flex rounded-lg border border-border overflow-hidden bg-muted">
                {STATUSES.map(s => (
                    <button
                        key={s.value}
                        onClick={() => set('status', s.value as OrderStatus | '')}
                        className={cn(
                            'px-3 py-1.5 text-xs font-medium transition-colors',
                            filters.status === s.value
                                ? 'bg-xmr text-white'
                                : 'hover:bg-secondary text-muted-foreground hover:text-foreground',
                        )}
                    >
                        {s.label}
                    </button>
                ))}
            </div>

            {/* Exchange filter */}
            <select
                value={filters.exchange}
                onChange={e => set('exchange', e.target.value as Exchange | '')}
                className="rounded-lg border border-border bg-background px-2 py-1.5 text-xs outline-none focus:border-xmr focus:ring-1 focus:ring-xmr/20 text-foreground"
            >
                <option value="">All exchanges</option>
                <option value="kraken">Kraken</option>
                <option value="hyperliquid">Hyperliquid</option>
            </select>

            {/* Side filter */}
            <select
                value={filters.side}
                onChange={e => set('side', e.target.value as Side | '')}
                className="rounded-lg border border-border bg-background px-2 py-1.5 text-xs outline-none focus:border-xmr focus:ring-1 focus:ring-xmr/20 text-foreground"
            >
                <option value="">Buy/Sell</option>
                <option value="buy">Buy</option>
                <option value="sell">Sell</option>
            </select>

            {/* Auto filter */}
            <select
                value={filters.is_auto === undefined ? '' : String(filters.is_auto)}
                onChange={e => set('is_auto', e.target.value === '' ? undefined : e.target.value === 'true')}
                className="rounded-lg border border-border bg-background px-2 py-1.5 text-xs outline-none focus:border-xmr focus:ring-1 focus:ring-xmr/20 text-foreground"
            >
                <option value="">Manual+Auto</option>
                <option value="false">Manual only</option>
                <option value="true">Auto only 🤖</option>
            </select>
        </div>
    )
}
