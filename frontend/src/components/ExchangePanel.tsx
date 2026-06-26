import { useState, type FormEvent } from 'react'
import { createOrder } from '@/lib/api'
import type { Side } from '@/types/order'
import { cn } from '@/lib/utils'
import ExchangeLogo from '@/components/ExchangeLogo'
import { usePrice } from '@/hooks/usePrice'

interface Props {
    exchange: string
    /** "spot" | "perp" — controls leverage slider visibility */
    kind: 'spot' | 'perp'
    /** Display name shown in the header */
    label: string
    onCreated?: () => void
}

export default function ExchangePanel({ exchange, kind, label, onCreated }: Props) {
    const [side, setSide] = useState<Side>('buy')
    const [quantity, setQuantity] = useState('')
    const [price, setPrice] = useState('')
    const [priceChange, setPriceChange] = useState('')
    const [leverage, setLeverage] = useState('1')
    const [loading, setLoading] = useState(false)
    const [error, setError] = useState<string | null>(null)
    const [successId, setSuccessId] = useState<number | null>(null)

    const isSpot = kind === 'spot'
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
        // feature-card-dark: surface-elevated, rounded-lg (20px), hairline border, no shadow
        <div className="bg-surface-elevated rounded-lg border border-hairline-dark flex flex-col overflow-hidden">
            {/* Exchange header — hairline divider, surface-deep tint to lift it off the card */}
            <div className="px-6 py-4 border-b border-hairline-dark bg-surface-deep flex items-center justify-between">
                <div className="flex items-center gap-2.5">
                    <ExchangeLogo exchange={exchange} size={24} />
                    <span className="text-sm font-semibold text-on-dark font-display tracking-tight">
                        {label}
                    </span>
                </div>
                <div className="flex items-baseline gap-2">
                    {priceLoading ? (
                        <span className="text-xs text-on-dark-mute animate-pulse">···</span>
                    ) : priceError ? (
                        <span className="text-xs text-on-dark-mute">—</span>
                    ) : (
                        <span className="text-base font-semibold font-display text-on-dark tracking-tight">
                            ${marketPrice}
                        </span>
                    )}
                    <span className="text-xs text-on-dark-mute">XMR/USDC</span>
                </div>
            </div>

            <form onSubmit={handleSubmit} className="p-6 flex flex-col gap-4">
                {/* Buy / Sell — pill toggle, rounded-full, semantic green/red */}
                <div className="grid grid-cols-2 gap-2 p-1.5 bg-surface-deep rounded-full">
                    {(['buy', 'sell'] as Side[]).map(s => (
                        <button
                            key={s}
                            type="button"
                            onClick={() => setSide(s)}
                            className={cn(
                                'py-2 text-sm font-semibold rounded-full transition-colors',
                                side === s
                                    ? s === 'buy'
                                        ? 'bg-buy text-white'
                                        : 'bg-sell text-white'
                                    : 'text-on-dark-mute hover:text-on-dark',
                            )}
                        >
                            {s === 'buy' ? 'Buy' : 'Sell'}
                        </button>
                    ))}
                </div>

                <Field label="Price (USDC)" value={price} onChange={setPrice} placeholder="145.80" />
                <Field label="Quantity (XMR)" value={quantity} onChange={setQuantity} placeholder="2.5" />
                <Field
                    label="Δ Price (0 = assisted)"
                    value={priceChange}
                    onChange={setPriceChange}
                    placeholder="1.50"
                    required={false}
                />

                {!isSpot && (
                    <div className="space-y-2">
                        <label className="text-sm text-on-dark-mute flex items-center justify-between">
                            <span>Leverage</span>
                            <span className="text-on-dark font-semibold font-display">×{leverage}</span>
                        </label>
                        <input
                            type="range"
                            min={1}
                            max={5}
                            step={1}
                            value={leverage}
                            onChange={e => setLeverage(e.target.value)}
                            className="w-full accent-primary"
                        />
                    </div>
                )}

                {error && <p className="text-sm text-accent-danger break-all">{error}</p>}
                {successId !== null && (
                    <p className="text-sm text-accent-light-green font-medium">
                        ✓ Order #{successId} submitted
                    </p>
                )}

                {/* Submit — rounded-full pill, semantic buy/sell surface */}
                <button
                    type="submit"
                    disabled={loading}
                    className={cn(
                        'w-full rounded-full py-3.5 text-sm font-semibold transition-colors h-12',
                        side === 'buy'
                            ? 'bg-buy hover:bg-buy-hover text-white'
                            : 'bg-sell hover:bg-sell-hover text-white',
                        loading && 'opacity-60 cursor-not-allowed',
                    )}
                >
                    {loading ? '···' : side === 'buy' ? 'Buy XMR' : 'Sell XMR'}
                </button>
            </form>
        </div>
    )
}

// text-input — 12px radius, 56px height, hairline border, Inter body-md
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
        <div className="space-y-2">
            <label className="text-sm text-on-dark-mute">{label}</label>
            <input
                type="number"
                step="any"
                min="0"
                required={required}
                value={value}
                onChange={e => onChange(e.target.value)}
                placeholder={placeholder}
                className="w-full rounded-md border border-hairline-dark bg-canvas-dark px-4 py-3.5 h-14 text-base text-on-dark font-sans outline-none focus:border-primary focus:ring-2 focus:ring-primary/30 placeholder:text-on-dark-mute/50 transition-colors"
            />
        </div>
    )
}
