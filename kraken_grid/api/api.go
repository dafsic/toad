package api

import (
	"github.com/dafsic/toad/kraken_grid/bot"
)

type API interface {
	Pair() string
	PlaceOrder(side string, multiplier int, price float64) error
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

func (a *api) PlaceOrder(side string, multiplier int, price float64) error {
	a.bot.PlaceOrder(a.bot.NewOrder(side, price, multiplier))
	return nil
}

func (a *api) SetBasePrice(price float64) {
	a.bot.SetBasePrice(price)
}
