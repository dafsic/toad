import { Link } from 'react-router-dom'
import { useExchanges } from '@/hooks/useExchanges'
import { exchangeToPair } from '@/types/pair'
import ExchangeLogo from '@/components/ExchangeLogo'
import { usePrice } from '@/hooks/usePrice'
import { cn } from '@/lib/utils'

export default function TradingPairListPage() {
    const { exchanges, loading, error } = useExchanges()

    if (loading) {
        return (
            <div className="min-h-screen flex items-center justify-center bg-canvas-dark text-on-dark-mute text-sm">
                Loading…
            </div>
        )
    }

    if (error) {
        return (
            <div className="min-h-screen flex items-center justify-center bg-canvas-dark text-accent-danger text-sm p-6">
                {error}
            </div>
        )
    }

    return (
        <div className="min-h-screen bg-canvas-dark text-on-dark font-sans">
            <Header />
            <main className="p-6 lg:p-8 max-w-[1400px] mx-auto">
                <div className="mb-6">
                    <h1 className="text-xl font-semibold font-display tracking-tight">Trading Pairs</h1>
                    <p className="text-sm text-on-dark-mute mt-1">Select a market to trade</p>
                </div>

                {exchanges.length === 0 ? (
                    <div className="bg-surface-elevated rounded-lg border border-hairline-dark p-6 text-on-dark-mute text-sm">
                        No exchanges enabled. Configure API credentials in <code className="text-on-dark">.env</code>.
                    </div>
                ) : (
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                        {exchanges.map(ex => {
                            const pair = exchangeToPair(ex)
                            return <PairCard key={ex.name} pair={pair} />
                        })}
                    </div>
                )}
            </main>
        </div>
    )
}

function PairCard({ pair }: { pair: import('@/types/pair').TradingPair }) {
    const { price, loading: priceLoading, error: priceError } = usePrice(pair.exchange.name)

    return (
        <Link
            to={`/trade/${pair.exchange.name}`}
            className="group block bg-surface-elevated rounded-lg border border-hairline-dark overflow-hidden hover:border-primary transition-colors"
        >
            <div className="px-6 py-4 border-b border-hairline-dark bg-surface-deep flex items-center justify-between">
                <div className="flex items-center gap-2.5">
                    <ExchangeLogo exchange={pair.exchange.name} size={24} />
                    <span className="text-sm font-semibold font-display tracking-tight">{pair.exchange.label}</span>
                </div>
                <span className={cn(
                    'text-[10px] uppercase tracking-wider font-semibold border rounded px-1.5 py-0.5',
                    pair.kind === 'spot'
                        ? 'border-accent-light-green text-accent-light-green'
                        : 'border-primary text-primary',
                )}>
                    {pair.kind === 'spot' ? 'Spot' : 'Perp'}
                </span>
            </div>
            <div className="p-6 flex items-center justify-between">
                <div>
                    <p className="text-xs text-on-dark-mute">Symbol</p>
                    <p className="text-base font-semibold font-display tracking-tight">{pair.symbol}</p>
                </div>
                <div className="text-right">
                    <p className="text-xs text-on-dark-mute">Price</p>
                    {priceLoading ? (
                        <span className="text-base font-semibold font-display animate-pulse">···</span>
                    ) : priceError ? (
                        <span className="text-sm text-on-dark-mute">—</span>
                    ) : (
                        <p className="text-base font-semibold font-display tracking-tight">${price}</p>
                    )}
                </div>
            </div>
            <div className="px-6 pb-4">
                <span className="inline-flex items-center justify-center w-full rounded-full py-2.5 text-sm font-semibold bg-primary text-on-primary group-hover:bg-primary-bright transition-colors">
                    Trade
                </span>
            </div>
        </Link>
    )
}

function Header() {
    return (
        <header className="h-16 px-6 lg:px-8 border-b border-hairline-dark flex items-center justify-between sticky top-0 bg-canvas-dark z-10">
            <div className="flex items-center gap-3">
                <Link to="/" className="text-xl leading-none hover:opacity-80">🐸</Link>
                <span className="text-hairline-dark select-none">·</span>
                <span className="text-sm font-semibold text-primary font-display tracking-tight">XMR/USDC</span>
                <span className="text-xs text-on-dark-mute">Grid Bot</span>
            </div>
            <nav className="flex items-center gap-4">
                <Link to="/" className="text-sm font-medium text-on-dark hover:text-primary transition-colors">Pairs</Link>
            </nav>
        </header>
    )
}
