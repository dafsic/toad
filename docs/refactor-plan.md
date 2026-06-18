# 重构计划：部分成交状态 + 轮询驱动挂对手单

## 目标

1. 数据库订单状态增加 `partially_filled`（部分成交），增加 `filled_quantity`（已成交数量）字段
2. WebSocket 事件仅用于更新已成交数量，**不挂对手单**
3. 新增轮询功能（每 1 分钟/交易所），查询最低挂卖单和最高挂买单是否完全成交，完全成交则挂对手单
4. 即使 WebSocket 完全不工作，网格也能正常运行

## 架构变更

### 当前架构
```
WebSocket 成交事件 → handle_fill() → 标记 filled + 挂对手单
```

### 新架构
```
WebSocket 成交事件 → 仅更新 filled_quantity + 状态 → partially_filled
轮询(60s/交易所)  → 查最低卖+最高买 → 完全成交 → 挂对手单
```

## 状态流（新）

```
pending → open → partially_filled → filled | cancelled | failed
                   ↑ websocket 部分成交     ↑ 轮询确认完全成交
```

## 文件变更清单

### 1. `src/db/migrations/002_partial_fill.sql`（新增）
- DROP 旧表 + CREATE 新表（开发阶段，无需数据迁移）
- 新增 `filled_quantity REAL NOT NULL DEFAULT 0`
- status CHECK 增加 `'partially_filled'`

### 2. `src/db/order.rs`
- `Order` 结构体增加 `filled_quantity: f64`
- 新增 `update_fill_progress(id, filled_quantity)` — 条件 `WHERE status IN ('open','partially_filled')`
- `mark_order_filled()` 同时写 `filled_quantity = order.quantity`
- `get_order_by_exchange_id()` 查询条件含 `'partially_filled'`
- `list_open_orders()` → `list_active_orders()`（含 `partially_filled`）
- 新增 `list_active_orders_by_exchange(exchange)` 供轮询用

### 3. `src/exchange/mod.rs`
- `FillEvent.quantity` → `filled_quantity`（语义：累计已成交数量）

### 4. `src/exchange/kraken.rs`
- WebSocket 处理 `trade` exec_type（部分成交事件）
- `trade` 和 `filled` 都发送 FillEvent，`filled_quantity = cum_qty`（累计值）

### 5. `src/exchange/hyperliquid.rs`
- 新增 `Arc<Mutex<HashMap<u64, f64>>>`（oid→累计成交量）
- fill 事件累加 `sz`，发送累计值作 `filled_quantity`

### 6. `src/engine/mod.rs`（核心重构）
- `handle_fill()` 精简：查活跃订单 → 条件更新 filled_quantity + partially_filled → SSE。**不挂对手单**
- 新增 `handle_filled_order(order)`：标记 filled + 挂对手单（提取自原 handle_fill 下单部分）
- 新增 `poll_exchange(exchange)`：取活跃订单 → 筛最低卖+最高买 → get_order_status → filled 调 handle_filled_order，cancelled 标记+通知
- `run()`：启动 WS 监听 + 轮询 task（interval 60s，首次 tick 立即执行替代 sync_order_status_on_startup）+ 主循环只处理 FillEvent 更新数量
- 删除 `sync_order_status_on_startup()`

### 7. `src/api/handlers.rs`
- `OrderResponse` 增加 `filled_quantity: f64`

### 8. `src/bot/mod.rs`
- `/orders` 显示 `partially_filled` 状态和 `filled_quantity`

### 9. 前端
- `types/order.ts`：OrderStatus 加 `'partially_filled'`，Order 加 `filled_quantity: number`
- `OrderList.tsx`：状态标签/颜色 + 已成交数量显示
- `OrderFilter.tsx`：增加 `partially_filled` 筛选项

### 10. `README.md` + `AGENTS.md`
- 更新状态流和网格逻辑说明

## 竞态保护
- `update_fill_progress()` 使用 `WHERE status IN ('open','partially_filled')` 条件更新
- `mark_order_filled()` 同样条件更新
- WebSocket 和轮询并发操作同一订单时，后写 filled 的生效，partially_filled 不会覆盖 filled

## 关键决策
- 轮询检测到完全成交时：`filled_price = order.price`，`filled_quantity = order.quantity`
- 反向挂单价格仍用 `order.price ± price_change`
- 轮询首次 tick 立即执行（tokio::time::interval 默认行为），替代启动恢复

## 验证
```bash
DATABASE_URL=sqlite:data/bot.db cargo check
cargo clippy
```
