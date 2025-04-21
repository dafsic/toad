package kraken

import (
	"context"

	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
	"go.uber.org/zap"
)

const ModuleName = "kraken"

type Params struct {
	fx.In
	Lc fx.Lifecycle

	Logger *zap.Logger

	Key    string `name:"kraken_key"`
	Secret string `name:"kraken_secret"`
}

type Result struct {
	fx.Out

	KrakenAPI Kraken
}

// NewFx wrap Manager with fx
func NewFx(params Params) Result {
	config := NewConfig(
		WithKey(params.Key),
		WithSecret(params.Secret),
	)
	karken := NewKraken(params.Logger, config)

	params.Lc.Append(fx.Hook{
		OnStart: func(ctx context.Context) error {
			return nil

		},
		OnStop: func(ctx context.Context) error {
			return nil
		},
	})

	return Result{
		KrakenAPI: karken,
	}
}

type Module struct{}

func (m *Module) Configure(app *cli.App) {
	app.Flags = append(app.Flags,
		&cli.StringFlag{
			Name:    "kraken_key",
			EnvVars: []string{"KRAKEN_KEY"},
			Value:   "not set",
		},
		&cli.StringFlag{
			Name:    "kraken_secret",
			EnvVars: []string{"KRAKEN_SECRET"},
			Value:   "not set",
		},
	)
}

func (m *Module) Install(ctx *cli.Context) fx.Option {
	return fx.Module(ModuleName,
		fx.Supply(fx.Annotate(ctx.String("kraken_key"), fx.ResultTags(`name:"kraken_key"`))),
		fx.Supply(fx.Annotate(ctx.String("kraken_secret"), fx.ResultTags(`name:"kraken_secret"`))),
		fx.Provide(NewFx),
	)
}
