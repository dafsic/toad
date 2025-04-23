package api

import (
	"fmt"

	"github.com/dafsic/toad/kraken_grid/bot"
	"github.com/dafsic/toad/kraken_grid/model"
)

type API interface {
	Pair() string
	PlaceOrder(side string, multiplier int) error
	SetBasePrice(price float64)
}

type api struct {
	bot bot.Bot
}

var _ API = (*api)(nil)

func NewAPI(bot bot.Bot) *api {
	return &api{
		bot: bot,
	}
}

func (a *api) Pair() string {
	return a.bot.Pair()
}

func (a *api) PlaceOrder(side string, multiplier int) error {
	var order *model.Order
	basePrice := a.bot.GetBasePrice()

	if side == bot.OrderBuy {
		order = a.bot.NewBuyOrder(basePrice, multiplier)
	}
	if side == bot.OrderSell {
		order = a.bot.NewSellOrder(basePrice, multiplier)
	}
	if order == nil {
		return fmt.Errorf("invalid order side: %s", side)
	}
	a.bot.PlaceOrder(order)
	return nil
}

func (a *api) SetBasePrice(price float64) {
	a.bot.SetBasePrice(price)
}
