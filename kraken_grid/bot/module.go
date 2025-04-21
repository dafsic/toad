package bot

import (
	"context"
	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
)

const ModuleName = "kraken_grid"

type Module struct{}

func (m *Module) Configure(app *cli.App) {
	app.Flags = append(app.Flags,
		&cli.StringFlag{
			Name:    "bot_name",
			EnvVars: []string{"BOT_NAME"},
			Value:   "XMR/BTC",
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
	return fx.Module(ModuleName,
		fx.Supply(
			fx.Annotate(WithBotName(ctx.String("bot_name")), fx.ResultTags(`group:"options"`)),
			fx.Annotate(WithSetp(ctx.Float64("grid_step")), fx.ResultTags(`group:"options"`)),
			fx.Annotate(WithGridAmount(ctx.Float64("grid_amount")), fx.ResultTags(`group:"options"`)),
			fx.Annotate(WithBasePrice(ctx.Float64("base_price")), fx.ResultTags(`group:"options"`)),
		),
		fx.Provide(fx.Annotate(NewConfig, fx.ParamTags(`group:"options"`))),
		fx.Provide(fx.Annotate(NewBot, fx.As(new(Bot)))),
		fx.Invoke(RunBot),
	)
}

func RunBot(bot Bot, lc fx.Lifecycle) {
	lc.Append(fx.Hook{
		OnStart: func(context.Context) error {
			return bot.Run()
		},
		OnStop: func(context.Context) error {
			bot.Stop()
			return nil
		},
	})
}
