import { useState, type FormEvent } from 'react'
import { createOrder } from '@/lib/api'
import { EXCHANGE_LABELS, SPOT_EXCHANGES, type Exchange, type Side } from '@/types/order'
import { cn } from '@/lib/utils'
import ExchangeLogo from '@/components/ExchangeLogo'
import { usePrice } from '@/hooks/usePrice'

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

    const isSpot = SPOT_EXCHANGES.includes(exchange)
    const { price: marketPrice, loading: priceLoading, error: priceError } = usePrice(exchange)

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
                price_change: priceChange === '' ? 0 : parseFloat(priceChange),
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
        <div className="border border-border bg-card rounded-xl overflow-hidden flex flex-col">
            {/* Exchange header */}
            <div className="px-4 py-3 border-b border-border bg-secondary flex items-center justify-between">
                <div className="flex items-center gap-2">
                    <ExchangeLogo exchange={exchange} size={24} />
                    <span className="text-sm font-bold tracking-wide text-foreground">
                        {EXCHANGE_LABELS[exchange]}
                    </span>
                </div>
                <div className="flex items-center gap-2">
                    {priceLoading ? (
                        <span className="text-xs text-muted-foreground animate-pulse tracking-widest">···</span>
                    ) : priceError ? (
                        <span className="text-xs text-muted-foreground">—</span>
                    ) : (
                        <span className="text-sm font-bold font-mono text-xmr">${marketPrice}</span>
                    )}
                    <span className="text-xs text-muted-foreground">XMR/USDC</span>
                </div>
            </div>

            <form onSubmit={handleSubmit} className="p-4 flex flex-col gap-3">
                {/* Buy / Sell */}
                <div className="grid grid-cols-2 gap-1 p-1 bg-secondary rounded-lg">
                    {(['buy', 'sell'] as Side[]).map(s => (
                        <button
                            key={s}
                            type="button"
                            onClick={() => setSide(s)}
                            className={cn(
                                'py-2 text-xs font-bold tracking-widest uppercase rounded-md transition-all',
                                side === s
                                    ? s === 'buy'
                                        ? 'bg-green-600 text-white shadow-sm'
                                        : 'bg-red-600 text-white shadow-sm'
                                    : 'text-muted-foreground hover:text-foreground',
                            )}
                        >
                            {s === 'buy' ? 'BUY' : 'SELL'}
                        </button>
                    ))}
                </div>

                <Field label="PRICE (USDC)" value={price} onChange={setPrice} placeholder="145.80" />
                <Field label="QTY (XMR)" value={quantity} onChange={setQuantity} placeholder="2.5" />
                <Field label="Δ PRICE (0=ASSISTED)" value={priceChange} onChange={setPriceChange} placeholder="1.50" required={false} />

                {!isSpot && (
                    <div className="space-y-1.5">
                        <label className="text-xs text-muted-foreground tracking-widest uppercase flex justify-between">
                            <span>LEVERAGE</span>
                            <span className="text-foreground font-bold">×{leverage}</span>
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
                    <p className="text-xs text-green-400 tracking-wider font-medium">
                        ✓ ORDER #{successId} SUBMITTED
                    </p>
                )}

                <button
                    type="submit"
                    disabled={loading}
                    className={cn(
                        'w-full rounded-lg py-2.5 text-sm font-bold tracking-widest uppercase transition-all',
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
    required = true,
}: {
    label: string
    value: string
    onChange: (v: string) => void
    placeholder?: string
    required?: boolean
}) {
    return (
        <div className="space-y-1.5">
            <label className="text-xs text-muted-foreground tracking-widest uppercase">{label}</label>
            <input
                type="number"
                step="any"
                min="0"
                required={required}
                value={value}
                onChange={e => onChange(e.target.value)}
                placeholder={placeholder}
                className="w-full rounded-lg border border-border bg-card px-3 py-2 text-sm font-mono outline-none focus:border-xmr focus:ring-1 focus:ring-xmr/20 placeholder:text-muted-foreground/40 transition-colors"
            />
        </div>
    )
}
