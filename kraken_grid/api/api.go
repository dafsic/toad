package api

import "github.com/dafsic/toad/kraken_grid/bot"

type API interface {
	Name() string
}

type api struct {
	bot bot.Bot
}

func NewAPI(bot bot.Bot) API {
	return &api{
		bot: bot,
	}
}

func (a *api) Name() string {
	return a.bot.Name()
}
