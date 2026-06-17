import { useState, type FormEvent } from 'react'
import { createOrder } from '@/lib/api'
import type { Exchange, Side } from '@/types/order'
import { cn } from '@/lib/utils'

interface Props {
    exchange: Exchange
    onCreated?: () => void
}

export default function ExchangePanel({ exchange, onCreated }: Props) {
    const [side, setSide] = useState<Side>('buy')
    const [quantity, setQuantity] = useState('')
    const [price, setPrice] = useState('')
    const [priceChange, setPriceChange] = useState('')
    const [leverage, setLeverage] = useState('1')
    const [loading, setLoading] = useState(false)
    const [error, setError] = useState<string | null>(null)
    const [successId, setSuccessId] = useState<number | null>(null)

    const isHyperliquid = exchange === 'hyperliquid'

    async function handleSubmit(e: FormEvent) {
        e.preventDefault()
        setError(null)
        setSuccessId(null)
        setLoading(true)
        try {
            const order = await createOrder({
                exchange,
                side,
                quantity: parseFloat(quantity),
                price: parseFloat(price),
                price_change: parseFloat(priceChange),
                leverage: parseInt(leverage, 10),
            })
            setSuccessId(order.id)
            setQuantity('')
            setPrice('')
            setPriceChange('')
            onCreated?.()
        } catch (e) {
            setError(String(e))
        } finally {
            setLoading(false)
        }
    }

    return (
        <div className="border border-border border-l-xmr border-l-2 bg-card rounded-lg overflow-hidden flex flex-col">
            {/* Exchange header */}
            <div className="px-4 py-3 border-b border-border flex items-center justify-between">
                <span className="text-xs font-bold tracking-widest uppercase text-foreground">
                    {exchange === 'kraken' ? 'Kraken' : 'Hyperliquid'}
                </span>
                <span className="text-xs text-xmr tracking-wider">XMR/USDC</span>
            </div>

            <form onSubmit={handleSubmit} className="p-4 flex flex-col gap-3">
                {/* Buy / Sell */}
                <div className="grid grid-cols-2 gap-1">
                    {(['buy', 'sell'] as Side[]).map(s => (
                        <button
                            key={s}
                            type="button"
                            onClick={() => setSide(s)}
                            className={cn(
                                'py-2 text-xs font-bold tracking-widest uppercase rounded transition-colors',
                                side === s
                                    ? s === 'buy'
                                        ? 'bg-green-600 text-white'
                                        : 'bg-red-600 text-white'
                                    : 'bg-secondary text-muted-foreground hover:text-foreground',
                            )}
                        >
                            {s === 'buy' ? 'BUY' : 'SELL'}
                        </button>
                    ))}
                </div>

                <Field label="QTY (XMR)" value={quantity} onChange={setQuantity} placeholder="2.5" />
                <Field label="PRICE (USDC)" value={price} onChange={setPrice} placeholder="145.80" />
                <Field label="Δ PRICE" value={priceChange} onChange={setPriceChange} placeholder="1.50" />

                {isHyperliquid && (
                    <div className="space-y-1.5">
                        <label className="text-xs text-muted-foreground tracking-widest uppercase">
                            LEVERAGE ×{leverage}
                        </label>
                        <input
                            type="range"
                            min={1}
                            max={50}
                            step={1}
                            value={leverage}
                            onChange={e => setLeverage(e.target.value)}
                            className="w-full accent-xmr"
                        />
                    </div>
                )}

                {error && <p className="text-xs text-red-400 break-all">{error}</p>}
                {successId !== null && (
                    <p className="text-xs text-green-400 tracking-wider">
                        ORDER #{successId} SUBMITTED
                    </p>
                )}

                <button
                    type="submit"
                    disabled={loading}
                    className={cn(
                        'w-full rounded py-2 text-xs font-bold tracking-widest uppercase transition-colors',
                        side === 'buy'
                            ? 'bg-green-600 hover:bg-green-500 text-white'
                            : 'bg-red-600 hover:bg-red-500 text-white',
                        loading && 'opacity-60 cursor-not-allowed',
                    )}
                >
                    {loading ? '···' : side === 'buy' ? 'BUY XMR' : 'SELL XMR'}
                </button>
            </form>
        </div>
    )
}

function Field({
    label,
    value,
    onChange,
    placeholder,
}: {
    label: string
    value: string
    onChange: (v: string) => void
    placeholder?: string
}) {
    return (
        <div className="space-y-1.5">
            <label className="text-xs text-muted-foreground tracking-widest uppercase">{label}</label>
            <input
                type="number"
                step="any"
                min="0"
                required
                value={value}
                onChange={e => onChange(e.target.value)}
                placeholder={placeholder}
                className="w-full rounded border border-border bg-secondary px-3 py-1.5 text-sm font-mono outline-none focus:border-xmr focus:ring-0 placeholder:text-muted-foreground/40 transition-colors"
            />
        </div>
    )
}
