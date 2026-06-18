import type { OrderFilters } from '@/hooks/useOrders'
import type { Exchange, OrderStatus, Side } from '@/types/order'
import { cn } from '@/lib/utils'

interface Props {
    filters: OrderFilters
    onChange: (f: OrderFilters) => void
}

const STATUSES: { value: OrderStatus | ''; label: string }[] = [
    { value: '', label: '全部' },
    { value: 'open', label: '挂单' },
    { value: 'partially_filled', label: '部分成交' },
    { value: 'filled', label: '成交' },
    { value: 'cancelled', label: '取消' },
    { value: 'failed', label: '失败' },
    { value: 'pending', label: '等待' },
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
                <option value="">全部交易所</option>
                <option value="kraken">Kraken</option>
                <option value="hyperliquid">Hyperliquid</option>
            </select>

            {/* Side filter */}
            <select
                value={filters.side}
                onChange={e => set('side', e.target.value as Side | '')}
                className="rounded-lg border border-border bg-background px-2 py-1.5 text-xs outline-none focus:border-xmr focus:ring-1 focus:ring-xmr/20 text-foreground"
            >
                <option value="">买/卖</option>
                <option value="buy">买入</option>
                <option value="sell">卖出</option>
            </select>

            {/* Auto filter */}
            <select
                value={filters.is_auto === undefined ? '' : String(filters.is_auto)}
                onChange={e => set('is_auto', e.target.value === '' ? undefined : e.target.value === 'true')}
                className="rounded-lg border border-border bg-background px-2 py-1.5 text-xs outline-none focus:border-xmr focus:ring-1 focus:ring-xmr/20 text-foreground"
            >
                <option value="">手动+自动</option>
                <option value="false">仅手动</option>
                <option value="true">仅自动 🤖</option>
            </select>
        </div>
    )
}
