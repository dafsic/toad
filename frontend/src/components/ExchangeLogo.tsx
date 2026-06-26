interface Props {
    exchange: string
    size?: number
    className?: string
}

export default function ExchangeLogo({ exchange, size = 20, className = '' }: Props) {
    const src = exchange === 'kraken'
        ? '/kraken.png'
        : exchange === 'hyperliquid'
            ? '/hyperliquid.png'
            : exchange === 'mexc_spot'
                ? '/mexc.png'
                : null
    const alt = exchange === 'kraken'
        ? 'Kraken'
        : exchange === 'hyperliquid'
            ? 'Hyperliquid'
            : exchange === 'mexc_spot'
                ? 'MEXC'
                : exchange

    if (src === null) {
        return (
            <span
                className={`inline-flex items-center justify-center rounded-sm flex-shrink-0 font-semibold text-on-dark-mute ${className}`}
                style={{ width: size, height: size, fontSize: size * 0.5 }}
            >
                {alt.charAt(0).toUpperCase()}
            </span>
        )
    }

    return (
        <img
            src={src}
            alt={alt}
            width={size}
            height={size}
            className={`inline-block rounded-sm flex-shrink-0 object-contain ${className}`}
        />
    )
}
