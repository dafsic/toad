package bot

import (
	"context"
	"fmt"

	"github.com/dafsic/toad/kraken_grid/dao"
	"github.com/dafsic/toad/kraken_grid/model"
	"github.com/dafsic/toad/utils"
)

func (b *GridBot) PlaceOrder(order *model.Order) {
	err := b.dao.CreateOrder(context.TODO(), order)
	if err != nil {
		b.stopChan <- fmt.Errorf("failed to create order in database: %w", err)
		return
	}

	err = b.krakenAPI.AddOrderWithWebsocket(
		b.privateWS,
		order.Pair,
		b.token,
		order.Side,
		order.Amount,
		order.Price,
		order.ID,
	)
	if err != nil {
		b.stopChan <- fmt.Errorf("failed to place new order: %w", err)
	}
}

func (b *GridBot) rebaseOrders() {
	// cancel all orders
	orders, err := b.dao.GetOpenOrders(context.TODO(), b.Pair())
	if err != nil && err != dao.ErrNotFound {
		b.stopChan <- fmt.Errorf("failed to get open orders from database: %w", err)
		return
	}

	orderIDs := make([]string, len(orders))
	for i, order := range orders {
		orderIDs[i] = order.OrderID
	}

	if len(orders) > 0 {
		err := b.krakenAPI.CancelOrderWithWebsocket(b.privateWS, b.token, orderIDs)
		if err != nil {
			b.stopChan <- fmt.Errorf("failed to cancel orders: %w", err)
			return
		}
	}

	// place new orders
	basePrice := b.GetBasePrice()
	buyBasePrice := basePrice
	sellBasePrice := basePrice
	for _, v := range b.config.multipliers {
		buyBasePrice -= b.config.step * float64(v)
		sellBasePrice += b.config.step * float64(v)
		b.PlaceOrder(b.NewOrder(OrderBuy, buyBasePrice, v))
		b.PlaceOrder(b.NewOrder(OrderSell, sellBasePrice, v))
	}
}

func (b *GridBot) NewOrder(side string, price float64, multiplier int) *model.Order {
	return &model.Order{
		Bot:        b.Pair(),
		Exchange:   "kraken",
		Pair:       b.config.baseCoin + "/" + b.config.quoteCoin,
		Price:      utils.FormatFloat(price, 6),
		Amount:     b.config.amount,
		Side:       side,
		Multiplier: multiplier,
		Status:     "pending",
	}
}
