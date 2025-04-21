package app

import (
	"fmt"
	"os"

	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
)

type Module interface {
	Configure(app *cli.App)
	Install(ctx *cli.Context) fx.Option
}

type Application struct {
	cliApp  *cli.App
	Modules []Module
}

func NewApplication(name, description string) *Application {
	app := &cli.App{
		Name:  name,
		Usage: description,
		Flags: []cli.Flag{
			&cli.BoolFlag{
				Name:    "version",
				Aliases: []string{"v"},
				Usage:   "Show version info",
			},
		},
	}

	return &Application{
		cliApp: app,
	}
}

func (app *Application) Install(modules ...Module) {
	app.Modules = append(app.Modules, modules...)
}

func (app *Application) Run() error {
	// Configure the app with the modules
	app.configure()

	return app.cliApp.Run(os.Args)
}

func (app *Application) configure() {
	app.cliApp.Action = app.action
	for _, module := range app.Modules {
		module.Configure(app.cliApp)
	}
}

func (app *Application) action(cCtx *cli.Context) error {
	if cCtx.Bool("version") {
		versionAction(cCtx)
		return nil
	}

	var opts []fx.Option
	for _, module := range app.Modules {
		opts = append(opts, module.Install(cCtx))
	}

	//opts = append(opts, fx.NopLogger)
	// Modules can not block the main thread, all blocking operations should be run in a goroutine
	fx.New(opts...).Run()
	return nil
}

func versionAction(_ *cli.Context) error {
	fmt.Printf("VERSION:         %s\n", version)
	fmt.Printf("GO_VERSION:      %s\n", go_version)
	fmt.Printf("GIT_BRANCH:      %s\n", git_branch)
	fmt.Printf("COMMIT_HASH:     %s\n", commit_hash)
	fmt.Printf("GIT_TREE_STATE:  %s\n", git_tree_state)
	fmt.Printf("BUILD_TIME:      %s\n", build_time)
	return nil
}
