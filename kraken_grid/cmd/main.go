package main

import (
	"fmt"

	"github.com/dafsic/toad/app"
	"github.com/dafsic/toad/kraken"
	"github.com/dafsic/toad/kraken_grid/api"
	"github.com/dafsic/toad/kraken_grid/bot"
	"github.com/dafsic/toad/kraken_grid/dao"
	"github.com/dafsic/toad/kraken_grid/server"
	"github.com/dafsic/toad/log"
	"github.com/dafsic/toad/postgres"
)

func main() {
	app := app.NewApplication("CryptoHunter", "A trading bot for kraken exchange")
	app.Install(
		&log.Module{},
		&kraken.Module{},
		&dao.Module{},
		&bot.Module{},
		&postgres.Module{},
		&api.Module{},
		&server.Module{},
	)
	if err := app.Run(); err != nil {
		fmt.Println(err)
		return
	}
}
