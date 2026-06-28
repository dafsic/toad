import { useEffect, useState } from 'react'
import { Routes, Route, Navigate } from 'react-router-dom'
import TradingPairListPage from '@/pages/TradingPairListPage'
import TradeAndHistoryPage from '@/pages/TradeAndHistoryPage'
import { LoginPage } from '@/pages/LoginPage'

export default function App() {
    const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null)

    useEffect(() => {
        fetch('/api/orders?limit=1')
            .then((r) => setIsAuthenticated(r.ok))
            .catch(() => setIsAuthenticated(false))
    }, [])

    if (isAuthenticated === null) {
        return (
            <div className="min-h-screen flex items-center justify-center bg-canvas-dark font-sans">
                <div className="text-on-dark-mute text-sm">Loading…</div>
            </div>
        )
    }

    if (!isAuthenticated) {
        return <LoginPage />
    }

    return (
        <Routes>
            <Route path="/" element={<TradingPairListPage />} />
            <Route path="/trade/:exchange" element={<TradeAndHistoryPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
    )
}
