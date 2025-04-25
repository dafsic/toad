package api

import (
	"github.com/dafsic/toad/kraken_grid/bot"
)

type API interface {
	Run(basePrice float64) error
	Stop(reason string)
	Status() (string, string)
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

func (a *api) Run(basePrice float64) error {
	a.bot.SetBasePrice(basePrice)
	return a.bot.Run()
}

func (a *api) Stop(reason string) {
	a.bot.Stop(reason)
}

func (a *api) Status() (string, string) {
	return a.bot.Status()
}

func (a *api) PlaceOrder(side string, multiplier int, price float64) error {
	a.bot.PlaceOrder(a.bot.NewOrder(side, price, multiplier))
	return nil
}

func (a *api) SetBasePrice(price float64) {
	a.bot.SetBasePrice(price)
}
