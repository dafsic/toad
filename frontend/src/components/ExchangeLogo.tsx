import type { Exchange } from '@/types/order'

interface Props {
    exchange: Exchange
    size?: number
    className?: string
}

export default function ExchangeLogo({ exchange, size = 20, className = '' }: Props) {
    const src = exchange === 'kraken'
        ? '/kraken.png'
        : exchange === 'hyperliquid'
            ? '/hyperliquid.png'
            : '/mexc.png'
    const alt = exchange === 'kraken'
        ? 'Kraken'
        : exchange === 'hyperliquid'
            ? 'Hyperliquid'
            : 'MEXC'

    return (
        <img
            src={src}
            alt={alt}
            width={size}
            height={size}
            className={`inline-block rounded-sm flex-shrink-0 ${className}`}
            style={{ imageRendering: 'pixelated' }}
        />
    )
}
