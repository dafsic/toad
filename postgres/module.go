package postgres

import (
	"context"

	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
)

const ModuleName = "database"

type Module struct{}

func (m *Module) Configure(app *cli.App) {
	app.Flags = append(app.Flags,
		&cli.StringFlag{
			Name:    "driver",
			EnvVars: []string{"DB_DRIVER"},
			Value:   "postgres",
			Usage:   "Database driver",
		},
		&cli.StringFlag{
			Name:    "dsn",
			EnvVars: []string{"DB_DSN"},
			Value:   "host=localhost port=5432 user=postgres password=postgres dbname=postgres sslmode=disable",
			Usage:   "Postgres connection string",
		},
	)
}

func (m *Module) Install(ctx *cli.Context) fx.Option {
	return fx.Module(ModuleName,
		fx.Supply(
			fx.Annotate(WithDriver(ctx.String("driver")), fx.ResultTags(`group:"options"`)),
			fx.Annotate(WithDSN(ctx.String("dsn")), fx.ResultTags(`group:"options"`)),
		),
		fx.Provide(fx.Annotate(NewConfig, fx.ParamTags(`group:"options"`))),
		fx.Provide(
			fx.Annotate(
				NewDatabase,
				fx.As(new(Database)),
				fx.OnStart(func(ctx context.Context, db Database) error {
					return db.Ping()
				}),
			),
		),
	)
}
