package bot

import (
	"context"
	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
)

type Module struct{}

func (m *Module) Configure(app *cli.App) {
	app.Flags = append(app.Flags,
		&cli.StringFlag{
			Name:    "base_coin",
			EnvVars: []string{"BASE_COIN"},
			Value:   "XMR",
		},
		&cli.StringFlag{
			Name:    "quote_coin",
			EnvVars: []string{"QUOTE_COIN"},
			Value:   "BTC",
		},
		&cli.Float64Flag{
			Name:    "grid_step",
			EnvVars: []string{"GRID_STEP"},
			Value:   0.00005,
		},
		&cli.Float64Flag{
			Name:    "grid_amount",
			EnvVars: []string{"GRID_AMOUNT"},
			Value:   1.0,
		},
		&cli.Float64Flag{
			Name:    "base_price",
			EnvVars: []string{"BASE_PRICE"},
			Value:   0,
		},
	)
}

func (m *Module) Install(ctx *cli.Context) fx.Option {
	var options []fx.Option
	if ctx.IsSet("base_coin") {
		options = append(options, fx.Supply(fx.Annotate(WithBaseCoin(ctx.String("base_coin")), fx.ResultTags(`group:"options"`))))
	}
	if ctx.IsSet("quote_coin") {
		options = append(options, fx.Supply(fx.Annotate(WithQuoteCoin(ctx.String("quote_coin")), fx.ResultTags(`group:"options"`))))
	}
	if ctx.IsSet("grid_step") {
		options = append(options, fx.Supply(fx.Annotate(WithSetp(ctx.Float64("grid_step")), fx.ResultTags(`group:"options"`))))
	}
	if ctx.IsSet("grid_amount") {
		options = append(options, fx.Supply(fx.Annotate(WithGridAmount(ctx.Float64("grid_amount")), fx.ResultTags(`group:"options"`))))
	}
	if ctx.IsSet("base_price") {
		options = append(options, fx.Supply(fx.Annotate(WithBasePrice(ctx.Float64("base_price")), fx.ResultTags(`group:"options"`))))
	}
	if ctx.IsSet("multipliers") {
		options = append(options, fx.Supply(fx.Annotate(WithMultipliers(ctx.String("multipliers")), fx.ResultTags(`group:"options"`))))
	}
	options = append(options,
		fx.Provide(fx.Annotate(NewConfig, fx.ParamTags(`group:"options"`))),
		fx.Provide(fx.Annotate(NewBot, fx.As(new(Bot)))),
		fx.Invoke(RunBot),
	)

	return fx.Module("bot", options...)
}

func RunBot(bot Bot, lc fx.Lifecycle) {
	lc.Append(fx.Hook{
		OnStart: func(context.Context) error {
			return bot.Run()
		},
		OnStop: func(context.Context) error {
			bot.Stop("received stop signal")
			return nil
		},
	})
}
