import { useState, type FormEvent } from 'react'
import { createOrder } from '@/lib/api'
import { SPOT_EXCHANGES, type Exchange, type Side } from '@/types/order'
import { cn } from '@/lib/utils'

interface Props {
    onCreated?: () => void
}

export default function OrderForm({ onCreated }: Props) {
    const [exchange, setExchange] = useState<Exchange>('kraken')
    const [side, setSide] = useState<Side>('buy')
    const [quantity, setQuantity] = useState('')
    const [price, setPrice] = useState('')
    const [priceChange, setPriceChange] = useState('')
    const [leverage, setLeverage] = useState('1')
    const [loading, setLoading] = useState(false)
    const [error, setError] = useState<string | null>(null)
    const [success, setSuccess] = useState<string | null>(null)

    async function handleSubmit(e: FormEvent) {
        e.preventDefault()
        setError(null)
        setSuccess(null)
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
            setSuccess(`订单 #${order.id} 已提交`)
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
        <div className="rounded-lg border bg-card p-5 space-y-4">
            <h2 className="font-semibold text-base">下单</h2>

            <form onSubmit={handleSubmit} className="space-y-3">
                {/* Exchange + Side */}
                <div className="grid grid-cols-2 gap-2">
                    <div className="space-y-1">
                        <label className="text-xs text-muted-foreground">交易所</label>
                        <select
                            value={exchange}
                            onChange={e => setExchange(e.target.value as Exchange)}
                            className="w-full rounded-md border bg-secondary px-2 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring"
                        >
                            <option value="kraken">Kraken</option>
                            <option value="hyperliquid">Hyperliquid</option>
                            <option value="mexc_spot">MEXC</option>
                        </select>
                    </div>
                    <div className="space-y-1">
                        <label className="text-xs text-muted-foreground">方向</label>
                        <div className="grid grid-cols-2 gap-1">
                            {(['buy', 'sell'] as Side[]).map(s => (
                                <button
                                    key={s}
                                    type="button"
                                    onClick={() => setSide(s)}
                                    className={cn(
                                        'rounded py-1.5 text-xs font-medium transition-colors',
                                        side === s
                                            ? s === 'buy'
                                                ? 'bg-green-600 text-white'
                                                : 'bg-red-600 text-white'
                                            : 'bg-secondary text-muted-foreground hover:text-foreground',
                                    )}
                                >
                                    {s === 'buy' ? '买入' : '卖出'}
                                </button>
                            ))}
                        </div>
                    </div>
                </div>

                {/* Price + Quantity */}
                <div className="grid grid-cols-2 gap-2">
                    <Field label="价格 (USDC)" value={price} onChange={setPrice} placeholder="145.80" />
                    <Field label="数量 (XMR)" value={quantity} onChange={setQuantity} placeholder="2.5" />
                </div>

                {/* Price change + Leverage */}
                <div className="grid grid-cols-2 gap-2">
                    <Field label="价差 Δ (0=辅助)" value={priceChange} onChange={setPriceChange} placeholder="1.50" required={false} />
                    <div className="space-y-1">
                        <label className="text-xs text-muted-foreground">
                            杠杆 ×{leverage}
                            {SPOT_EXCHANGES.includes(exchange) && <span className="ml-1 opacity-50">(现货固定 1)</span>}
                        </label>
                        <input
                            type="range"
                            min={1}
                            max={50}
                            step={1}
                            value={leverage}
                            disabled={SPOT_EXCHANGES.includes(exchange)}
                            onChange={e => setLeverage(e.target.value)}
                            className="w-full accent-green-500 disabled:opacity-40"
                        />
                    </div>
                </div>

                {error && <p className="text-xs text-red-400 break-all">{error}</p>}
                {success && <p className="text-xs text-green-400">{success}</p>}

                <button
                    type="submit"
                    disabled={loading}
                    className={cn(
                        'w-full rounded-md py-2 text-sm font-semibold transition-colors',
                        side === 'buy'
                            ? 'bg-green-600 hover:bg-green-500 text-white'
                            : 'bg-red-600 hover:bg-red-500 text-white',
                        loading && 'opacity-60 cursor-not-allowed',
                    )}
                >
                    {loading ? '提交中…' : side === 'buy' ? '买入下单' : '卖出下单'}
                </button>
            </form>
        </div>
    )
}

function Field({ label, value, onChange, placeholder, required = true }: {
    label: string
    value: string
    onChange: (v: string) => void
    placeholder?: string
    required?: boolean
}) {
    return (
        <div className="space-y-1">
            <label className="text-xs text-muted-foreground">{label}</label>
            <input
                type="number"
                step="any"
                min="0"
                required={required}
                value={value}
                onChange={e => onChange(e.target.value)}
                placeholder={placeholder}
                className="w-full rounded-md border bg-secondary px-2 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring placeholder:text-muted-foreground/50"
            />
        </div>
    )
}
