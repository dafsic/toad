import { useState, useCallback } from 'react'
import type { Order, OrderStatus, Exchange, Side } from '@/types/order'
import { listOrders } from '@/lib/api'

export interface OrderFilters {
    exchange: Exchange | ''
    side: Side | ''
    status: OrderStatus | ''
    is_auto: boolean | undefined
}

export interface OrdersState {
    items: Order[]
    nextCursor: number | null
    loading: boolean
    error: string | null
    filters: OrderFilters
}

const DEFAULT_FILTERS: OrderFilters = {
    exchange: '',
    side: '',
    status: 'open',
    is_auto: undefined,
}

export function useOrders() {
    const [state, setState] = useState<OrdersState>({
        items: [],
        nextCursor: null,
        loading: false,
        error: null,
        filters: DEFAULT_FILTERS,
    })

    const fetchPage = useCallback(async (
        filters: OrderFilters,
        beforeId?: number,
        append = false,
    ) => {
        setState(s => ({ ...s, loading: true, error: null }))
        try {
            const query = {
                ...(filters.exchange ? { exchange: filters.exchange } : {}),
                ...(filters.side ? { side: filters.side } : {}),
                ...(filters.status ? { status: filters.status } : {}),
                ...(filters.is_auto !== undefined ? { is_auto: filters.is_auto } : {}),
                ...(beforeId ? { before_id: beforeId } : {}),
                limit: 30,
            }
            const page = await listOrders(query as Parameters<typeof listOrders>[0])
            setState(s => ({
                ...s,
                loading: false,
                items: append ? [...s.items, ...page.items] : page.items,
                nextCursor: page.next_cursor ?? null,
            }))
        } catch (e) {
            setState(s => ({ ...s, loading: false, error: String(e) }))
        }
    }, [])

    const setFilters = useCallback((filters: OrderFilters) => {
        setState(s => ({ ...s, filters, items: [], nextCursor: null }))
        fetchPage(filters)
    }, [fetchPage])

    const loadMore = useCallback(() => {
        setState(s => {
            if (s.nextCursor) {
                fetchPage(s.filters, s.nextCursor, true)
            }
            return s
        })
    }, [fetchPage])

    /** Optimistically update a single order's status (used by SSE) */
    const updateOrderStatus = useCallback((orderId: number, status: string) => {
        setState(s => ({
            ...s,
            items: s.items.map(o => o.id === orderId ? { ...o, status: status as OrderStatus } : o),
        }))
    }, [])

    /** Refresh the current first page (called on new order created) */
    const onOrderCreated = useCallback(() => {
        setState(s => {
            fetchPage(s.filters)
            return s
        })
    }, [fetchPage])

    return { state, setFilters, loadMore, updateOrderStatus, onOrderCreated, fetchPage }
}

