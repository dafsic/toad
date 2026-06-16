import { useEffect } from 'react'

type SseEvent =
    | { type: 'order_created'; order_id: number }
    | { type: 'order_updated'; order_id: number; status: string }

export function useSSE(
    onOrderCreated: (id: number) => void,
    onOrderUpdated: (id: number, status: string) => void,
) {
    useEffect(() => {
        let es: EventSource
        let retryTimer: ReturnType<typeof setTimeout>

        function connect() {
            es = new EventSource('/api/sse')

            es.onmessage = (e) => {
                try {
                    const event = JSON.parse(e.data) as SseEvent
                    if (event.type === 'order_created') {
                        onOrderCreated(event.order_id)
                    } else if (event.type === 'order_updated') {
                        onOrderUpdated(event.order_id, event.status)
                    }
                } catch {
                    // ignore parse errors
                }
            }

            es.onerror = () => {
                es.close()
                // EventSource auto-reconnects, but add a small delay to avoid tight loops
                retryTimer = setTimeout(connect, 3000)
            }
        }

        connect()

        return () => {
            clearTimeout(retryTimer)
            es?.close()
        }
    }, [onOrderCreated, onOrderUpdated])
}

