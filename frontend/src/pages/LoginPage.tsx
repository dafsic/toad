import { useEffect, useState } from 'react'

export function LoginPage() {
    const [code, setCode] = useState<string>('')
    const [loading, setLoading] = useState(true)
    const [error, setError] = useState<string>('')
    const [copied, setCopied] = useState(false)

    useEffect(() => {
        fetch('/api/auth/request', { method: 'POST' })
            .then((r) => r.json())
            .then((data: { code: string }) => {
                const loginCode = data.code
                setCode(loginCode)
                setLoading(false)

                const es = new EventSource(`/api/auth/wait/${loginCode}`)
                es.onmessage = async (e) => {
                    try {
                        const msg = JSON.parse(e.data)
                        if (msg.ready || msg.token) {
                            // Claim the token via server so it is set as HttpOnly cookie (never touches JS)
                            await fetch(`/api/auth/complete/${loginCode}`, {
                                method: 'POST',
                            })
                            window.location.href = '/'
                        }
                    } catch {
                        setError('Login failed. Refresh and try again.')
                    } finally {
                        es.close()
                    }
                }
                es.onerror = () => {
                    es.close()
                    setError('Connection timeout. Refresh and try again.')
                }
            })
            .catch(() => {
                setLoading(false)
                setError('Failed to get code. Refresh the page.')
            })
    }, [])

    return (
        <div className="min-h-screen bg-canvas-dark text-on-dark font-sans flex flex-col">
            {/* nav-bar — matches the main app header */}
            <header className="h-16 px-6 lg:px-8 border-b border-hairline-dark flex items-center">
                <span className="text-xl leading-none">🐸</span>
                <span className="mx-3 text-hairline-dark select-none">·</span>
                <span className="text-sm font-semibold text-primary font-display tracking-tight">XMR/USDC</span>
                <span className="ml-2 text-xs text-on-dark-mute">Grid Bot</span>
            </header>

            <div className="flex-1 flex items-center justify-center p-6">
                <div className="w-full max-w-md">
                    {/* feature-card-dark */}
                    <div className="bg-surface-elevated rounded-lg border border-hairline-dark overflow-hidden">
                        {/* Card header band — surface-deep tint + hairline divider */}
                        <div className="px-6 py-4 border-b border-hairline-dark bg-surface-deep flex items-center gap-2.5">
                            <span className="h-2.5 w-2.5 rounded-full bg-primary inline-block flex-shrink-0" />
                            <span className="text-sm font-semibold font-display tracking-tight">
                                Telegram authentication
                            </span>
                        </div>

                        <div className="p-6 space-y-6">
                            {loading && !error && (
                                <p className="text-sm text-on-dark-mute animate-pulse">Initializing…</p>
                            )}

                            {error ? (
                                <p className="text-sm text-accent-danger">{error}</p>
                            ) : code ? (
                                <>
                                    <div className="space-y-2.5">
                                        <p className="text-sm text-on-dark-mute">Send to bot</p>
                                        <button
                                            onClick={() => {
                                                navigator.clipboard.writeText(`/login ${code}`)
                                                setCopied(true)
                                                setTimeout(() => setCopied(false), 2000)
                                            }}
                                            // text-input style, clickable
                                            className="w-full flex items-center justify-between rounded-md border border-hairline-dark bg-canvas-dark px-4 py-3.5 text-base text-on-dark hover:border-primary transition-colors group h-14"
                                        >
                                            <span className="font-mono">
                                                /login <span className="font-semibold tracking-widest">{code}</span>
                                            </span>
                                            <span className="text-sm text-on-dark-mute group-hover:text-primary transition-colors">
                                                {copied ? '✓ Copied' : 'Copy'}
                                            </span>
                                        </button>
                                    </div>

                                    <div className="flex items-center gap-2">
                                        <span className="h-2 w-2 rounded-full bg-accent-light-green animate-pulse flex-shrink-0" />
                                        <span className="text-sm text-on-dark-mute">Waiting for verification</span>
                                    </div>

                                    <p className="text-xs text-on-dark-mute/60">
                                        Code expires in 5 minutes
                                    </p>
                                </>
                            ) : null}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    )
}
