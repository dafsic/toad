package log

import (
	"context"
	"fmt"

	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
	"go.uber.org/zap"
)

const ModuleName = "log"

func NewLogger(conf *zap.Config) (*zap.Logger, error) {
	l, e := conf.Build()
	if e != nil {
		return nil, e
	}

	return l, nil
}

type Module struct{}

func (m *Module) Configure(app *cli.App) {
	app.Flags = append(app.Flags,
		&cli.StringFlag{
			Name:    "log_level",
			EnvVars: []string{"LOG_LEVEL"},
			Value:   "info",
		},
	)
}

func (m *Module) Install(ctx *cli.Context) fx.Option {
	return fx.Module(ModuleName,
		fx.Supply(
			fx.Annotate(WithLevel(Level(ctx.String("bot_name"))), fx.ResultTags(`group:"options"`)),
		),
		fx.Provide(fx.Annotate(NewConfig, fx.ParamTags(`group:"options"`))),
		fx.Provide(
			fx.Annotate(
				NewLogger,
				fx.OnStop(func(ctx context.Context, l *zap.Logger) error {
					if e := l.Sync(); e != nil {
						return fmt.Errorf("%w: could not sync before exit", e)
					}
					return nil
				}),
			),
		),
	)
}
