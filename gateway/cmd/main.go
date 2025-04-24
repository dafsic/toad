package main

import (
	"context"
	"fmt"

	"github.com/dafsic/toad/app"
	"github.com/dafsic/toad/gateway/client"
	"github.com/dafsic/toad/gateway/http"
	"github.com/dafsic/toad/log"
	"github.com/urfave/cli/v2"
	"go.uber.org/fx"
	"go.uber.org/zap"
)

func main() {
	app := app.NewApplication("Toad Gateway", "API Gateway for Toad microservices")
	app.Install(
		&log.Module{},
		&GatewayModule{},
	)
	if err := app.Run(); err != nil {
		fmt.Println(err)
		return
	}
}

type GatewayModule struct{}

func (m *GatewayModule) Configure(app *cli.App) {
	app.Flags = append(app.Flags,
		&cli.StringFlag{
			Name:    "http_addr",
			EnvVars: []string{"HTTP_ADDR"},
			Value:   "localhost:8080",
			Usage:   "HTTP server address",
		},
		&cli.StringFlag{
			Name:    "kraken_xmrbtc_addr",
			EnvVars: []string{"KRAKEN_XMRBTC_ADDR"},
			Value:   "localhost:50051",
			Usage:   "Kraken_XMRBTC gRPC server address",
		},
		&cli.StringFlag{
			Name:    "ogre_xmrbtc_addr",
			EnvVars: []string{"OGRE_XMRBTC_ADDR"},
			Value:   "localhost:50052",
			Usage:   "Ogre_XMRBTC gRPC server address",
		},
	)
}

func (m *GatewayModule) Install(ctx *cli.Context) fx.Option {
	httpAddr := ctx.String("http_addr")
	krakenXMRBTCAddr := ctx.String("kraken_xmrbtc_addr")

	return fx.Module("gateway",
		fx.Provide(
			fx.Annotate(
				func(logger *zap.Logger, clinets ...client.GrpcClient) http.Server {
					return http.NewGinServer(logger, clinets...)
				},
				fx.ParamTags("", `group:"clients"`),
			),
			fx.Annotate(
				func(logger *zap.Logger) (*client.KrakenGridClient, error) {
					return client.NewKrakenGridClient(logger, "kraken_xmr_btc", krakenXMRBTCAddr)
				},
				fx.ResultTags(`group:"clients"`),
				fx.As(new(client.GrpcClient)),
			),
		),
		fx.Invoke(
			func(lc fx.Lifecycle, srv http.Server) {
				lc.Append(fx.Hook{
					OnStart: func(c context.Context) error {
						return srv.Run(httpAddr)
					},
					OnStop: func(c context.Context) error {
						return srv.Close()
					},
				})
			},
		),
	)
}
