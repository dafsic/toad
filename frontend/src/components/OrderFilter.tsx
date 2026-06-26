import type { OrderFilters } from '@/hooks/useOrders'
import type { ExchangeInfo, OrderStatus, Side } from '@/types/order'
import { cn } from '@/lib/utils'

interface Props {
    filters: OrderFilters
    exchanges: ExchangeInfo[]
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

const selectClass =
    'rounded-md border border-hairline-dark bg-surface-elevated px-3 py-2 text-sm text-on-dark outline-none focus:border-primary focus:ring-2 focus:ring-primary/30 transition-colors appearance-none cursor-pointer'

export default function OrderFilter({ filters, exchanges, onChange }: Props) {
    function set<K extends keyof OrderFilters>(key: K, value: OrderFilters[K]) {
        onChange({ ...filters, [key]: value })
    }

    return (
        <div className="flex flex-wrap items-center gap-3">
            {/* Status — sub-nav-pill row, rounded-full chips */}
            <div className="flex flex-wrap gap-2">
                {STATUSES.map(s => (
                    <button
                        key={s.value}
                        onClick={() => set('status', s.value as OrderStatus | '')}
                        className={cn(
                            'px-4 py-2 text-sm font-semibold rounded-full transition-colors',
                            filters.status === s.value
                                ? 'bg-on-dark text-canvas-dark'
                                : 'bg-surface-elevated text-on-dark-mute hover:text-on-dark border border-hairline-dark',
                        )}
                    >
                        {s.label}
                    </button>
                ))}
            </div>

            {/* Exchange filter — options driven by enabled exchanges */}
            <select
                value={filters.exchange}
                onChange={e => set('exchange', e.target.value)}
                className={selectClass}
            >
                <option value="">All exchanges</option>
                {exchanges.map(ex => (
                    <option key={ex.name} value={ex.name}>{ex.label}</option>
                ))}
            </select>

            {/* Side filter */}
            <select
                value={filters.side}
                onChange={e => set('side', e.target.value as Side | '')}
                className={selectClass}
            >
                <option value="">Buy / Sell</option>
                <option value="buy">Buy</option>
                <option value="sell">Sell</option>
            </select>

            {/* Auto filter */}
            <select
                value={filters.is_auto === undefined ? '' : String(filters.is_auto)}
                onChange={e => set('is_auto', e.target.value === '' ? undefined : e.target.value === 'true')}
                className={selectClass}
            >
                <option value="">Manual + Auto</option>
                <option value="false">Manual only</option>
                <option value="true">Auto only 🤖</option>
            </select>
        </div>
    )
}
