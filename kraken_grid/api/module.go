package api

import (
	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
)

type Module struct{}

func (m *Module) Configure(app *cli.App) {}

func (m *Module) Install(ctx *cli.Context) fx.Option {
	return fx.Module("api",
		fx.Provide(fx.Annotate(NewAPI, fx.As(new(API)))),
	)
}
