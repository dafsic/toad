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
                setCode(data.code)
                setLoading(false)

                const es = new EventSource(`/api/auth/wait/${data.code}`)
                es.onmessage = (e) => {
                    try {
                        const { token } = JSON.parse(e.data)
                        document.cookie = `auth_token=${token}; path=/; max-age=28800`
                        window.location.href = '/'
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
        <div className="min-h-screen bg-background flex flex-col font-mono">
            {/* Header */}
            <header className="px-6 py-4 border-b border-border flex items-center">
                <span className="text-xl">🐸</span>
                <span className="mx-2 text-border select-none">·</span>
                <span className="text-sm font-semibold text-xmr">XMR/USDC</span>
                <span className="ml-2 text-xs text-muted-foreground">GRID BOT</span>
            </header>

            <div className="flex-1 flex items-center justify-center p-4">
                <div className="w-full max-w-sm">
                    {/* Panel */}
                    <div className="border border-border bg-card rounded-xl overflow-hidden">
                        <div className="px-5 py-3 border-b border-border bg-secondary flex items-center gap-2">
                            <span className="h-3 w-3 rounded-full bg-xmr inline-block flex-shrink-0" />
                            <span className="text-xs font-bold tracking-widest uppercase">
                                TELEGRAM AUTH
                            </span>
                        </div>

                        <div className="p-5 space-y-5">
                            {loading && !error && (
                                <p className="text-xs text-muted-foreground tracking-widest animate-pulse">
                                    INITIALIZING···
                                </p>
                            )}

                            {error ? (
                                <p className="text-xs text-red-400">{error}</p>
                            ) : code ? (
                                <>
                                    <div className="space-y-2">
                                        <p className="text-xs text-muted-foreground tracking-widest uppercase">
                                            Send to bot
                                        </p>
                                        <button
                                            onClick={() => {
                                                navigator.clipboard.writeText(`/login ${code}`)
                                                setCopied(true)
                                                setTimeout(() => setCopied(false), 2000)
                                            }}
                                            className="w-full flex items-center justify-between rounded-lg border border-border bg-secondary px-4 py-3 text-sm hover:border-xmr transition-colors group"
                                        >
                                            <span className="text-foreground">
                                                /login <span className="font-bold tracking-widest">{code}</span>
                                            </span>
                                            <span className="text-xs text-muted-foreground tracking-widest group-hover:text-xmr transition-colors">
                                                {copied ? '✓ COPIED' : 'COPY'}
                                            </span>
                                        </button>
                                    </div>

                                    <div className="flex items-center gap-2">
                                        <span className="h-1.5 w-1.5 rounded-full bg-green-500 animate-pulse flex-shrink-0" />
                                        <span className="text-xs text-muted-foreground tracking-widest uppercase">
                                            Waiting for verification
                                        </span>
                                    </div>

                                    <p className="text-xs text-muted-foreground/40 tracking-wide">
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
