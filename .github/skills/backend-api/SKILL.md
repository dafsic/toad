---
name: toad-backend-api
description: >
  Complete reference for the Toad Grid Bot backend REST API and SSE stream.
  Use this skill whenever you are building, modifying, or debugging any part of
  the Toad frontend (React components, hooks, API calls, SSE subscriptions,
  TypeScript types) and need to know endpoint URLs, request/response shapes,
  pagination mechanics, error codes, or real-time event formats. Trigger this
  skill at the first sign of any frontend ↔ backend integration work — placing
  orders, listing/filtering orders, cancelling orders, or subscribing to live
  updates — even if the user has not explicitly mentioned "API" or "backend".
---

# Toad Grid Bot — Backend API Skill

Use this document to understand and integrate with the Toad backend REST API and SSE stream.

---

## Base URL

When running locally: `http://localhost:3000`  
All API endpoints are prefixed with `/api`.

---

## Data Types

### Order Object

```json
{
  "id": 42,
  "exchange": "kraken",         // "kraken" | "hyperliquid"
  "symbol": "XMR/USDC",
  "side": "buy",                // "buy" | "sell"
  "quantity": 2.5,
  "price": 145.80,
  "price_change": 1.50,
  "leverage": 1,                // always 1 for Kraken; user-specified for Hyperliquid
  "is_auto": false,             // true = created by grid engine (reverse leg)
  "parent_order_id": null,      // id of the filled order that triggered this one
  "exchange_order_id": "TXID123",
  "status": "open",             // "pending" | "open" | "filled" | "cancelled" | "failed"
  "filled_price": null,         // populated when status = "filled"
  "created_at": "2026-06-16T10:00:00",
  "updated_at": "2026-06-16T10:00:05"
}
```

### Order Status Flow

```
pending → open → filled      (normal fill)
                → cancelled  (user cancelled)
         → failed            (exchange rejected)
```

---

## Endpoints

### Place Order

```
POST /api/orders
Content-Type: application/json
```

**Request body:**

```json
{
  "exchange":    "kraken",   // required: "kraken" | "hyperliquid"
  "side":        "buy",      // required: "buy" | "sell"
  "quantity":    2.5,        // required: > 0, XMR amount
  "price":       145.80,     // required: > 0, limit price in USDC
  "price_change": 1.50,      // required: > 0, grid step size
  "leverage":    1           // optional: default 1; Kraken ignores (always 1); Hyperliquid >= 1
}
```

**Response `201 Created`:**

```json
{ /* Order object */ }
```

**Error responses:**
- `400 Bad Request` — validation failed (message in body)
- `502 Bad Gateway` — exchange API rejected the order (message in body)

---

### List Orders (cursor pagination)

```
GET /api/orders
```

**Query parameters:**

| Parameter   | Type    | Required | Description |
|-------------|---------|----------|-------------|
| `exchange`  | string  | No       | Filter: `"kraken"` or `"hyperliquid"` |
| `side`      | string  | No       | Filter: `"buy"` or `"sell"` |
| `status`    | string  | No       | Filter: `"pending"` / `"open"` / `"filled"` / `"cancelled"` / `"failed"` |
| `is_auto`   | bool    | No       | Filter: `true` (grid-generated) / `false` (manual) |
| `before_id` | integer | No       | Cursor: return orders with `id < before_id`; omit for first page |
| `limit`     | integer | No       | Page size, default `20`, max `100` |

**Response `200 OK`:**

```json
{
  "items": [ /* array of Order objects, newest first */ ],
  "next_cursor": 17   // pass as before_id for next page; null means no more data
}
```

**Pagination example:**

```
# First page (20 newest orders)
GET /api/orders?status=open&limit=20

# Next page
GET /api/orders?status=open&limit=20&before_id=<next_cursor from previous response>
```

---

### Cancel Order

```
DELETE /api/orders/:id
```

**Path parameter:** `id` — integer order id

**Response `204 No Content`** — success, no body.

**Error responses:**
- `404 Not Found` — order not found
- `400 Bad Request` — order is not in `open` status
- `502 Bad Gateway` — exchange API cancel failed (message in body)

---

## SSE Real-time Events

```
GET /api/sse
Accept: text/event-stream
```

Establishes a persistent Server-Sent Events connection. The server pushes a keep-alive comment every 30 seconds to prevent proxy timeouts.

Each event is a JSON object in the `data` field:

### `order_created`

Sent when a new order is placed (manual or grid engine auto-generated).

```json
{ "type": "order_created", "order_id": 42 }
```

### `order_updated`

Sent when an order's status changes.

```json
{ "type": "order_updated", "order_id": 42, "status": "filled" }
```

Possible `status` values: `"open"` / `"filled"` / `"cancelled"` / `"failed"`

### Client-side usage (TypeScript)

```typescript
const es = new EventSource('/api/sse')

es.onmessage = (e) => {
  const event = JSON.parse(e.data) as
    | { type: 'order_created'; order_id: number }
    | { type: 'order_updated'; order_id: number; status: string }

  if (event.type === 'order_created') {
    // refresh order list or prepend new order
  } else if (event.type === 'order_updated') {
    // update the matching order in local state
  }
}

es.onerror = () => {
  // EventSource auto-reconnects; no manual retry needed
}

// cleanup
// es.close()
```

---

## Notes

- All amounts are in XMR (quantity) and USDC (price).
- Grid engine automatically creates reverse orders after a fill; these have `is_auto: true` and `parent_order_id` set.
- Cancelling an order only stops future chain legs; already-filled ancestors are unaffected.
- Hyperliquid orders use isolated margin mode. `leverage` is set per order before placement.
