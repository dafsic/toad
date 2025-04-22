package api

import (
	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
)

const ModuleName = "api"

type Module struct{}

func (m *Module) Configure(app *cli.App) {}

func (m *Module) Install(ctx *cli.Context) fx.Option {
	return fx.Module(ModuleName,
		fx.Provide(fx.Annotate(NewAPI, fx.As(new(API)))),
	)
}
