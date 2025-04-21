package dao

import (
	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
)

const ModuleName = "database"

type Module struct{}

func (m *Module) Configure(app *cli.App) {}

func (m *Module) Install(ctx *cli.Context) fx.Option {
	return fx.Module(ModuleName,
		fx.Provide(fx.Annotate(NewDao, fx.As(new(Dao)))),
	)
}
