package bot

import (
	"context"
	"fmt"

	"github.com/dafsic/toad/kraken_grid/dao"
	"github.com/dafsic/toad/kraken_grid/model"
	"github.com/dafsic/toad/utils"
)

func (b *Grid) PlaceOrder(order *model.Order) {
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

func (b *Grid) NewBuyOrder(basePrice float64, multiplier int) *model.Order {
	price := basePrice - (b.config.step * float64(multiplier))
	return b.newOrder(OrderBuy, price, multiplier)
}

func (b *Grid) NewSellOrder(basePrice float64, multiplier int) *model.Order {
	price := basePrice + (b.config.step * float64(multiplier))
	return b.newOrder(OrderSell, price, multiplier)
}

func (b *Grid) rebaseOrders() {
	// cancel all orders
	orders, err := b.dao.GetOpenOrders(context.TODO(), b.config.name)
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
	for _, v := range b.config.multipliers {
		b.PlaceOrder(b.newOrder(OrderBuy, basePrice, v))
		b.PlaceOrder(b.newOrder(OrderSell, basePrice, v))
	}
}

func (b *Grid) newOrder(side string, price float64, multiplier int) *model.Order {
	return &model.Order{
		Bot:        b.config.name,
		Exchange:   "kraken",
		Pair:       b.config.baseCoin + "/" + b.config.quoteCoin,
		Price:      utils.FormatFloat(price, 6),
		Amount:     b.config.amount,
		Side:       side,
		Multiplier: multiplier,
		Status:     "pending",
	}
}
