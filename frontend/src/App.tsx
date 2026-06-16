import OrderForm from '@/components/OrderForm'
import OrderFilter from '@/components/OrderFilter'
import OrderList from '@/components/OrderList'
import { useSSE } from '@/hooks/useSSE'

export default function App() {
    // TODO: 订阅 SSE，收到事件后刷新订单列表
    useSSE()

    return (
        <div className="min-h-screen bg-background p-6">
            <header className="mb-8">
                <h1 className="text-2xl font-bold">Toad Grid Bot</h1>
                <p className="text-muted-foreground">XMR/USDC 无限链式反向网格交易</p>
            </header>
            <main className="grid gap-6 lg:grid-cols-[400px_1fr]">
                <OrderForm />
                <div className="flex flex-col gap-4">
                    <OrderFilter />
                    <OrderList />
                </div>
            </main>
        </div>
    )
}
